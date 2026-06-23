use super::{
    OutputQueue, decode_error_to_io, redacted_inner_state, stream_decoder_failed_error,
    trailing_input_after_padding_error,
};
use crate::{Alphabet, Engine};
use std::io::{self, Write};

/// A streaming Base64 decoder for `std::io::Write`.
///
/// Like any [`Write`] implementation, [`Write::write`] may accept only
/// part of the provided input. Accepted input may be held as decoded
/// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
/// or a later write drains the wrapped writer. Use [`Write::write_all`]
/// when the whole input slice must be consumed.
///
/// # Security
///
/// This adapter uses the normal strict decoder, not the [`crate::ct`]
/// module. It may branch or return early based on malformed input and it
/// preserves strict error diagnostics. Do not use it for secret-bearing
/// payloads when malformed-input timing matters; decode a complete frame
/// with the matching `ct` engine instead.
///
/// Decoded bytes are written to the wrapped writer as valid quads are
/// accepted. If a later quad in the same logical frame is malformed, already
/// written bytes cannot be recalled from sockets, pipes, files, or other
/// external sinks. For atomic frame semantics, decode into an in-memory buffer
/// first and transfer to the final writer only after [`Self::finish`] succeeds.
///
/// If malformed input is detected after earlier quads in the same
/// [`Write::write`] call were accepted, the adapter may return `Ok(consumed)`
/// for the accepted prefix while latching itself as failed. The next write,
/// flush, or finish call then returns the stored failure state. Callers must
/// follow normal `Write` partial-progress rules and continue checking for the
/// terminal error.
pub struct Decoder<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: OutputQueue<1024>,
    finished: bool,
    failed: bool,
    finalized: bool,
}

impl<W, A, const PAD: bool> Decoder<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming decoder.
    ///
    /// # Security
    ///
    /// Streaming decoders use the normal strict decode path. They are not
    /// constant-time-oriented secret decoders.
    #[must_use]
    pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
            finalized: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped writer.
    #[must_use]
    pub fn get_ref(&self) -> &W {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }

    /// Returns the Base64 engine used by this adapter.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this adapter uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns the number of encoded input bytes currently buffered until
    /// a complete 4-byte Base64 decode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this decoder currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional input bytes are needed to complete the
    /// currently buffered decode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            4 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of decoded bytes buffered for the wrapped writer
    /// after a previous write or flush could not fully drain them.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of decoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more decoded bytes can be buffered before this
    /// adapter must drain the wrapped writer.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this decoder has decoded output waiting to be
    /// written to the wrapped writer.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this decoder has processed a terminal padded block.
    ///
    /// Once this returns `true`, later calls to [`Write::write`] with
    /// additional input return an error because strict Base64 does not
    /// permit trailing payload bytes after padding.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.finished
    }

    /// Returns whether this decoder has been finalized.
    ///
    /// Once this returns `true`, later non-empty writes return an error.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether this decoder has rejected malformed Base64 input.
    ///
    /// Once this returns `true`, later writes, flushes, and finalization
    /// attempts return an error. The unchecked [`Self::into_inner`] method
    /// can still be used for explicit recovery of the wrapped writer.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// writer without discarding pending encoded input.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && !self.has_pending_input() && !self.has_buffered_output()
    }

    /// Consumes the decoder without flushing pending input.
    ///
    /// Prefer [`Self::finish`] when the decoded output must be complete.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Consumes the decoder only when no partial input quantum is buffered.
    ///
    /// This does not flush or finalize the wrapped writer. It is a checked
    /// alternative to [`Self::into_inner`] for callers that want to avoid
    /// accidentally discarding pending encoded input bytes.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &W {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }

    fn clear_output(&mut self) {
        self.output.clear_all();
    }
}

impl<W, A, const PAD: bool> Drop for Decoder<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> core::fmt::Debug for Decoder<W, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Decoder")
            .field("inner", &redacted_inner_state(self.inner.is_some()))
            .field("engine", &self.engine)
            .field("pending", &"<redacted>")
            .field("pending_len", &self.pending_len)
            .field("pending_input_needed_len", &self.pending_input_needed_len())
            .field("buffered_output_len", &self.output.len())
            .field("buffered_output_capacity", &self.output.capacity())
            .field(
                "buffered_output_remaining_capacity",
                &self.output.available_capacity(),
            )
            .field("can_into_inner", &self.can_into_inner())
            .field("terminal_padding", &self.finished)
            .field("finalized", &self.finalized)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<W, A, const PAD: bool> Decoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    /// Validates any final pending input and flushes the wrapped writer
    /// without consuming this decoder.
    ///
    /// After this succeeds, [`Self::pending_len`] returns `0`, later
    /// writes are rejected, and [`Self::finish`] can still be used to
    /// recover the wrapped writer.
    /// If the final buffered input is malformed, an error is returned and
    /// the caller still owns the decoder for diagnostics or explicit
    /// recovery.
    pub fn try_finish(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if !self.finalized {
            self.queue_pending_final()?;
            self.finalized = true;
        }
        self.flush()
    }

    /// Validates final pending input, flushes the wrapped writer, and returns it.
    pub fn finish(mut self) -> io::Result<W> {
        self.try_finish()?;
        Ok(self.take_inner())
    }

    fn queue_pending_final(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 4];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&pending[..pending_len], &mut decoded);
        crate::wipe_bytes(&mut pending);
        if let Err(err) = result {
            self.clear_pending();
            return Err(err);
        }
        self.clear_pending();
        Ok(())
    }

    fn queue_full_quad(&mut self, mut input: [u8; 4]) -> io::Result<()> {
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&input, &mut decoded);
        crate::wipe_bytes(&mut input);
        let written = result?;
        if written < 3 {
            self.finished = true;
        }
        Ok(())
    }

    fn queue_decoded_temp(&mut self, input: &[u8], decoded: &mut [u8]) -> io::Result<usize> {
        let written = match self.engine.decode_slice(input, decoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(decoded);
                self.failed = true;
                return Err(decode_error_to_io(err));
            }
        };

        let result = self.output.push_slice(&decoded[..written]);
        crate::wipe_bytes(decoded);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        Ok(written)
    }

    fn drain_output(&mut self) -> io::Result<()> {
        let mut chunk = [0u8; 1024];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = self.inner_mut().write(&chunk[..pending]);
            crate::wipe_bytes(&mut chunk[..pending]);
            match result {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64 stream decoder could not drain buffered output",
                    ));
                }
                Ok(written) => {
                    if written > pending {
                        self.failed = true;
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "wrapped writer reported more bytes than provided",
                        ));
                    }
                    self.output.discard_front(written);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

impl<W, A, const PAD: bool> Write for Decoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if input.is_empty() {
            self.drain_output()?;
            return Ok(0);
        }
        self.drain_output()?;
        if self.finalized {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64 stream decoder received input after finalization",
            ));
        }
        if self.finished {
            self.failed = true;
            return Err(trailing_input_after_padding_error());
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 4 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(input.len());
            }

            let mut quad = [0u8; 4];
            quad[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            quad[self.pending_len..].copy_from_slice(&input[..needed]);
            let result = self.queue_full_quad(quad);
            crate::wipe_bytes(&mut quad);
            if let Err(err) = result {
                self.clear_pending();
                return Err(err);
            }
            self.clear_pending();
            consumed += needed;

            if self.finished {
                return Ok(consumed);
            }
        }

        while input.len() - consumed >= 4 {
            if self.output.available_capacity() < 3 {
                return Ok(consumed);
            }

            let mut quad = [
                input[consumed],
                input[consumed + 1],
                input[consumed + 2],
                input[consumed + 3],
            ];
            let mut decoded = [0u8; 3];
            let written = match self.engine.decode_slice(&quad, &mut decoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(&mut quad);
                    crate::wipe_bytes(&mut decoded);
                    self.failed = true;
                    if consumed > 0 {
                        // Report accepted prefix progress per `io::Write`.
                        // The adapter is now failed; the next write/flush/
                        // finish call returns an error for the malformed quad.
                        return Ok(consumed);
                    }

                    return Err(decode_error_to_io(err));
                }
            };

            let result = self.output.push_slice(&decoded[..written]);
            crate::wipe_bytes(&mut quad);
            crate::wipe_bytes(&mut decoded);
            result?;
            consumed += 4;

            if written < 3 {
                self.finished = true;
                return Ok(consumed);
            }
        }

        let tail = &input[consumed..];
        self.pending[..tail.len()].copy_from_slice(tail);
        self.pending_len = tail.len();
        consumed += tail.len();

        Ok(consumed)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        self.drain_output()?;
        self.inner_mut().flush()
    }
}
