use super::{OutputQueue, encode_error_to_io, redacted_inner_state, stream_encoder_failed_error};
use crate::{Alphabet, Engine};
use std::io::{self, Read};

/// A streaming Base64 encoder for `std::io::Read`.
pub struct EncoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<R>,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: OutputQueue<1024>,
    finished: bool,
    failed: bool,
}

impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming encoder reader.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
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

    /// Returns the number of raw input bytes currently buffered until a
    /// complete 3-byte Base64 encode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this encoder reader currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional raw input bytes are needed to complete
    /// the currently buffered encode quantum.
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

    /// Returns the number of encoded bytes currently buffered and ready to
    /// be read before this adapter polls the wrapped reader again.
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
    /// adapter must return bytes to the caller.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this encoder reader currently has encoded output
    /// waiting in its internal queue.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this encoder reader has reached EOF in the wrapped
    /// reader.
    ///
    /// This may become `true` before [`Self::is_finished`] when encoded
    /// output is still buffered for the caller.
    #[must_use]
    pub const fn has_finished_input(&self) -> bool {
        self.finished
    }

    /// Returns whether this reader has reached EOF and has no encoded
    /// output buffered for the caller.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished && self.output.is_empty()
    }

    /// Returns whether this adapter has failed closed after an internal
    /// stream error.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// reader without discarding pending input or buffered encoded output.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        self.is_finished() && !self.failed
    }

    /// Consumes the encoder reader and returns the wrapped reader.
    #[must_use]
    pub fn into_inner(mut self) -> R {
        self.take_inner()
    }

    /// Consumes the encoder reader only after the encoded stream is fully
    /// drained.
    ///
    /// This is a checked alternative to [`Self::into_inner`] for callers
    /// that want to avoid accidentally discarding pending input or encoded
    /// output buffered inside the adapter.
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
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut R {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn take_inner(&mut self) -> R {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }
}

impl<R, A, const PAD: bool> Drop for EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.output.clear_all();
    }
}

impl<R, A, const PAD: bool> core::fmt::Debug for EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncoderReader")
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
            .field("failed", &self.failed)
            .finish()
    }
}

impl<R, A, const PAD: bool> Read for EncoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }

        if output.is_empty() {
            return Ok(0);
        }

        while self.output.is_empty() && !self.finished {
            self.fill_output()?;
        }

        Ok(self.output.pop_slice(output))
    }
}

impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn fill_output(&mut self) -> io::Result<()> {
        let mut input = [0u8; 768];
        let read = match self.inner_mut().read(&mut input) {
            Ok(read) => read,
            Err(err) => {
                crate::wipe_bytes(&mut input);
                return Err(err);
            }
        };
        if read == 0 {
            crate::wipe_bytes(&mut input);
            self.finished = true;
            if let Err(err) = self.push_final_pending() {
                self.failed = true;
                return Err(err);
            }
            return Ok(());
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 3 - self.pending_len;
            if read < needed {
                self.pending[self.pending_len..self.pending_len + read]
                    .copy_from_slice(&input[..read]);
                self.pending_len += read;
                crate::wipe_bytes(&mut input);
                return Ok(());
            }

            let mut chunk = [0u8; 3];
            chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            chunk[self.pending_len..].copy_from_slice(&input[..needed]);
            let result = self.push_encoded(&chunk);
            crate::wipe_bytes(&mut chunk);
            if let Err(err) = result {
                crate::wipe_bytes(&mut input);
                self.failed = true;
                return Err(err);
            }
            self.clear_pending();
            consumed += needed;
        }

        let remaining = &input[consumed..read];
        let full_len = remaining.len() / 3 * 3;
        let tail_len = remaining.len() - full_len;
        let mut tail = [0u8; 2];
        tail[..tail_len].copy_from_slice(&remaining[full_len..]);
        let result = if full_len > 0 {
            self.push_encoded(&remaining[..full_len])
        } else {
            Ok(())
        };
        crate::wipe_bytes(&mut input);
        if let Err(err) = result {
            crate::wipe_bytes(&mut tail);
            self.failed = true;
            return Err(err);
        }
        self.pending[..tail_len].copy_from_slice(&tail[..tail_len]);
        crate::wipe_bytes(&mut tail);
        self.pending_len = tail_len;
        Ok(())
    }

    fn push_final_pending(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 2];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        self.clear_pending();
        let result = self.push_encoded(&pending[..pending_len]);
        crate::wipe_bytes(&mut pending);
        result
    }

    fn push_encoded(&mut self, input: &[u8]) -> io::Result<()> {
        let mut encoded = [0u8; 1024];
        let written = match self.engine.encode_slice(input, &mut encoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(&mut encoded);
                return Err(encode_error_to_io(err));
            }
        };
        let result = self.output.push_slice(&encoded[..written]);
        crate::wipe_bytes(&mut encoded);
        result
    }
}
