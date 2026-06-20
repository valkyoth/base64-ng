use super::{OutputQueue, encode_error_to_io, redacted_inner_state, stream_encoder_failed_error};
use crate::{Alphabet, Engine};
use std::io::{self, Write};

/// A streaming Base64 encoder for `std::io::Write`.
///
/// Like any [`Write`] implementation, [`Write::write`] may accept only
/// part of the provided input. Accepted input may be held as encoded
/// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
/// or a later write drains the wrapped writer. Use [`Write::write_all`]
/// when the whole input slice must be consumed.
pub struct Encoder<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: OutputQueue<1024>,
    finalized: bool,
    failed: bool,
}

impl<W, A, const PAD: bool> Encoder<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming encoder.
    #[must_use]
    pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: OutputQueue::new(),
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

    /// Returns the number of raw input bytes currently buffered until a
    /// complete 3-byte Base64 encode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this encoder currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional input bytes are needed to complete the
    /// currently buffered encode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            3 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of encoded bytes buffered for the wrapped
    /// writer after a previous write or flush could not fully drain them.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of encoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more encoded bytes can be buffered before this
    /// adapter must drain the wrapped writer.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this encoder has encoded output waiting to be
    /// written to the wrapped writer.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this encoder has been finalized.
    ///
    /// Once this returns `true`, later writes return an error.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether this encoder has failed closed after an unrecoverable
    /// internal encoding or buffering error.
    ///
    /// Ordinary wrapped-writer I/O errors are retryable and do not set this
    /// flag. Once this returns `true`, later writes, flushes, and finalization
    /// attempts return an error. The unchecked [`Self::into_inner`] method can
    /// still be used for explicit recovery of the wrapped writer.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// writer without discarding pending input.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && !self.has_pending_input() && !self.has_buffered_output()
    }

    /// Consumes the encoder without flushing pending input.
    ///
    /// Prefer [`Self::finish`] when the encoded output must be complete.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Consumes the encoder only when no partial input quantum is buffered.
    ///
    /// This does not flush or finalize the wrapped writer. It is a checked
    /// alternative to [`Self::into_inner`] for callers that want to avoid
    /// accidentally discarding pending input bytes.
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
            None => unreachable!("stream encoder inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream encoder inner writer was already taken"),
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

impl<W, A, const PAD: bool> Drop for Encoder<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> core::fmt::Debug for Encoder<W, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Encoder")
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
            .field("finalized", &self.finalized)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<W, A, const PAD: bool> Encoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    /// Writes any pending input and flushes the wrapped writer without
    /// consuming this encoder.
    ///
    /// After this succeeds, [`Self::pending_len`] returns `0`, later
    /// writes are rejected, and [`Self::finish`] can still be used to
    /// recover the wrapped writer.
    /// This is useful when a caller needs to finalize a framed payload
    /// while keeping the stream adapter available for diagnostics or
    /// explicit recovery.
    pub fn try_finish(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }
        if !self.finalized {
            self.queue_pending_final()?;
            self.finalized = true;
        }
        self.flush()
    }

    /// Writes any pending input, flushes the wrapped writer, and returns it.
    pub fn finish(mut self) -> io::Result<W> {
        self.try_finish()?;
        Ok(self.take_inner())
    }

    fn queue_pending_final(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 2];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        let mut encoded = [0u8; 4];
        let result = self.queue_encoded_temp(&pending[..pending_len], &mut encoded);
        crate::wipe_bytes(&mut pending);
        result?;
        self.clear_pending();
        Ok(())
    }

    fn queue_encoded_temp(&mut self, input: &[u8], encoded: &mut [u8]) -> io::Result<()> {
        let written = match self.engine.encode_slice(input, encoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(encoded);
                self.failed = true;
                return Err(encode_error_to_io(err));
            }
        };

        let result = self.output.push_slice(&encoded[..written]);
        crate::wipe_bytes(encoded);
        if result.is_err() {
            self.failed = true;
        }
        result
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
                        "base64 stream encoder could not drain buffered output",
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

impl<W, A, const PAD: bool> Write for Encoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }
        self.drain_output()?;
        if self.finalized {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64 stream encoder received input after finalization",
            ));
        }
        if input.is_empty() {
            return Ok(0);
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 3 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(input.len());
            }

            let mut chunk = [0u8; 3];
            chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            chunk[self.pending_len..].copy_from_slice(&input[..needed]);

            let mut encoded = [0u8; 4];
            let result = self.queue_encoded_temp(&chunk, &mut encoded);
            crate::wipe_bytes(&mut chunk);
            result?;
            self.clear_pending();
            consumed += needed;
        }

        let remaining = &input[consumed..];
        let full_len = remaining.len() / 3 * 3;
        if full_len > 0 {
            let max_by_queue = self.output.available_capacity() / 4 * 3;
            let mut take = core::cmp::min(full_len, core::cmp::min(768, max_by_queue));
            take -= take % 3;

            if take == 0 {
                return Ok(consumed);
            }

            let mut encoded = [0u8; 1024];
            self.queue_encoded_temp(&remaining[..take], &mut encoded)?;
            consumed += take;

            if take < full_len {
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
            return Err(stream_encoder_failed_error());
        }
        self.drain_output()?;
        self.inner_mut().flush()
    }
}
