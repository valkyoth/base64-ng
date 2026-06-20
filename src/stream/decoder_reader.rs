use super::{OutputQueue, decode_error_to_io, redacted_inner_state, stream_decoder_failed_error};
use crate::{Alphabet, Engine};
use std::io::{self, Read};

/// A streaming Base64 decoder for `std::io::Read`.
///
/// For padded engines, this reader stops at the terminal padded Base64
/// block and leaves later bytes unread in the wrapped reader. This preserves
/// boundaries for callers that decode one Base64 payload from a larger
/// stream.
///
/// # Security
///
/// This adapter uses the normal strict decoder, not the [`crate::ct`]
/// module. It may branch or return early based on malformed input and it
/// preserves strict error diagnostics. Do not use it for secret-bearing
/// payloads when malformed-input timing matters; decode a complete frame
/// with the matching `ct` engine instead.
pub struct DecoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<R>,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: OutputQueue<3>,
    finished: bool,
    terminal_seen: bool,
    failed: bool,
}

impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming decoder reader.
    ///
    /// # Security
    ///
    /// Streaming decoder readers use the normal strict decode path. They
    /// are not constant-time-oriented secret decoders.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
            terminal_seen: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped reader.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped reader.
    pub fn get_mut(&mut self) -> &mut R {
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

    /// Returns whether this decoder reader currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional encoded input bytes are needed to
    /// complete the currently buffered decode quantum.
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

    /// Returns the number of decoded bytes currently buffered and ready to
    /// be read before this adapter polls the wrapped reader again.
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
    /// adapter must return bytes to the caller.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this decoder reader currently has decoded output
    /// waiting in its internal queue.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this decoder reader has seen terminal padding.
    ///
    /// For padded engines, this becomes `true` after the terminal padded
    /// block is decoded. The wrapped reader is then left positioned after
    /// that Base64 block so adjacent framed bytes can be read by the
    /// caller.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.terminal_seen
    }

    /// Returns whether this decoder reader has reached EOF or terminal
    /// padding in the wrapped reader.
    ///
    /// This may become `true` before [`Self::is_finished`] when decoded
    /// output is still buffered for the caller.
    #[must_use]
    pub const fn has_finished_input(&self) -> bool {
        self.finished
    }

    /// Returns whether this reader has reached EOF or terminal padding
    /// and has no decoded output buffered for the caller.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished && self.output.is_empty()
    }

    /// Returns whether this decoder reader has rejected malformed Base64
    /// input.
    ///
    /// Once this returns `true`, later reads return an error. The unchecked
    /// [`Self::into_inner`] method can still be used for explicit recovery
    /// of the wrapped reader.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// reader without discarding buffered decoded output.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && self.is_finished()
    }

    /// Consumes the decoder reader and returns the wrapped reader.
    #[must_use]
    pub fn into_inner(mut self) -> R {
        self.take_inner()
    }

    /// Consumes the decoder reader only after the Base64 payload is fully
    /// drained.
    ///
    /// For padded streams, terminal padding may leave adjacent framed bytes
    /// unread in the wrapped reader. This method succeeds only after all
    /// decoded output buffered by this adapter has been read, so recovering
    /// the wrapped reader does not silently discard decoded bytes.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<R, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &R {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut R {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn take_inner(&mut self) -> R {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }
}

impl<R, A, const PAD: bool> Drop for DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.output.clear_all();
    }
}

impl<R, A, const PAD: bool> core::fmt::Debug for DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DecoderReader")
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
            .field("finished", &self.finished)
            .field("terminal_padding", &self.terminal_seen)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<R, A, const PAD: bool> Read for DecoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.failed {
            return Err(stream_decoder_failed_error());
        }

        while self.output.is_empty() && !self.finished {
            self.fill_output()?;
        }

        Ok(self.output.pop_slice(output))
    }
}

impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn fill_output(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if self.terminal_seen {
            self.finished = true;
            return Ok(());
        }

        let mut input = [0u8; 4];
        let available = 4 - self.pending_len;
        let read = match self.inner_mut().read(&mut input[..available]) {
            Ok(read) => read,
            Err(err) => {
                crate::wipe_bytes(&mut input);
                return Err(err);
            }
        };
        if read == 0 {
            crate::wipe_bytes(&mut input);
            self.finished = true;
            self.push_final_pending()?;
            return Ok(());
        }

        self.pending[self.pending_len..self.pending_len + read].copy_from_slice(&input[..read]);
        crate::wipe_bytes(&mut input);
        self.pending_len += read;
        if self.pending_len < 4 {
            return Ok(());
        }

        let mut quad = self.pending;
        self.clear_pending();
        let result = self.push_decoded(&quad);
        crate::wipe_bytes(&mut quad);
        result?;
        if self.terminal_seen {
            self.finished = true;
        }
        Ok(())
    }

    fn push_final_pending(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 4];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        self.clear_pending();
        let result = self.push_decoded(&pending[..pending_len]);
        crate::wipe_bytes(&mut pending);
        result
    }

    fn push_decoded(&mut self, input: &[u8]) -> io::Result<()> {
        let mut decoded = [0u8; 3];
        let written = match self.engine.decode_slice(input, &mut decoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(&mut decoded);
                self.failed = true;
                return Err(decode_error_to_io(err));
            }
        };
        let result = self.output.push_slice(&decoded[..written]);
        crate::wipe_bytes(&mut decoded);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        if input.len() == 4 && written < 3 {
            self.terminal_seen = true;
        }
        Ok(())
    }
}
