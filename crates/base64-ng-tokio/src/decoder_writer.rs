use base64_ng::{Alphabet, Engine};
use core::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tokio::io::{self, AsyncWrite};

use crate::{decode_io_error, queue::OutputQueue, wipe_bytes};

const DECODE_OUTPUT_CAP: usize = 1024;

/// Async writer that accepts Base64 bytes and writes decoded bytes to `inner`.
///
/// The adapter validates strict Base64 quanta as input is accepted. Shutdown is
/// the finalization boundary and validates any final unpadded tail.
///
/// # Security
///
/// Streaming decode is not atomic. Decoded bytes from valid leading quanta may
/// already have been written before a later malformed quantum is observed. For
/// atomic or secret-bearing frames, collect a bounded frame and use a
/// non-streaming strict or `ct` decode path.
pub struct DecoderWriter<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: OutputQueue<DECODE_OUTPUT_CAP>,
    terminal_padding: bool,
    finalized: bool,
    failed: bool,
    _alphabet: PhantomData<A>,
}

impl<W, A, const PAD: bool> DecoderWriter<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new async Base64 decoder writer.
    #[must_use]
    pub fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: OutputQueue::new(),
            terminal_padding: false,
            finalized: false,
            failed: false,
            _alphabet: PhantomData,
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

    /// Consumes the adapter and returns the wrapped writer.
    ///
    /// This does not validate final pending input. Prefer
    /// [`AsyncWriteExt::shutdown`](tokio::io::AsyncWriteExt::shutdown) before
    /// calling this when the decoded stream must be complete.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Returns whether this adapter has encountered an unrecoverable error.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether shutdown has finalized this adapter.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether a terminal padded quantum has been accepted.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.terminal_padding
    }

    /// Returns the number of encoded bytes buffered until a full decode
    /// quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns the number of decoded bytes currently buffered for `inner`.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    fn clear_pending(&mut self) {
        wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }

    fn clear_output(&mut self) {
        self.output.clear_all();
    }

    fn inner_ref(&self) -> &W {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("tokio decoder writer inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("tokio decoder writer inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("tokio decoder writer inner writer was already taken"),
        }
    }

    fn queue_decoded_temp(&mut self, input: &[u8], decoded: &mut [u8]) -> io::Result<usize> {
        let written = match self.engine.decode_slice(input, decoded) {
            Ok(written) => written,
            Err(error) => {
                wipe_bytes(decoded);
                self.failed = true;
                return Err(decode_io_error(error));
            }
        };

        let result = self.output.push_slice(&decoded[..written]);
        wipe_bytes(decoded);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        Ok(written)
    }

    fn queue_full_quad(&mut self, mut input: [u8; 4]) -> io::Result<()> {
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&input, &mut decoded);
        wipe_bytes(&mut input);
        let written = result?;
        if written < 3 {
            self.terminal_padding = true;
        }
        Ok(())
    }

    fn queue_pending_final(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        if PAD || self.pending_len == 1 {
            self.failed = true;
            self.clear_pending();
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio decoder writer received incomplete final quantum",
            ));
        }

        let mut pending = [0u8; 4];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&pending[..pending_len], &mut decoded);
        wipe_bytes(&mut pending);
        result?;
        self.clear_pending();
        Ok(())
    }

    fn trailing_input_error(&mut self) -> io::Error {
        self.failed = true;
        io::Error::new(
            io::ErrorKind::InvalidData,
            "base64-ng-tokio decoder writer received trailing input after padding",
        )
    }

    fn process_input(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        if self.terminal_padding {
            return Err(self.trailing_input_error());
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
            wipe_bytes(&mut quad);
            if let Err(error) = result {
                self.clear_pending();
                return Err(error);
            }
            self.clear_pending();
            consumed += needed;

            if self.terminal_padding {
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
                Err(error) => {
                    wipe_bytes(&mut quad);
                    wipe_bytes(&mut decoded);
                    if consumed > 0 {
                        return Ok(consumed);
                    }
                    self.failed = true;
                    return Err(decode_io_error(error));
                }
            };

            let result = self.output.push_slice(&decoded[..written]);
            wipe_bytes(&mut quad);
            wipe_bytes(&mut decoded);
            if result.is_err() {
                self.failed = true;
            }
            result?;
            consumed += 4;

            if written < 3 {
                self.terminal_padding = true;
                return Ok(consumed);
            }
        }

        let tail = &input[consumed..];
        self.pending[..tail.len()].copy_from_slice(tail);
        self.pending_len = tail.len();
        consumed += tail.len();
        Ok(consumed)
    }
}

impl<W, A, const PAD: bool> Drop for DecoderWriter<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> DecoderWriter<W, A, PAD>
where
    W: AsyncWrite + Unpin,
    A: Alphabet + Unpin,
{
    fn poll_drain_output(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut chunk = [0u8; DECODE_OUTPUT_CAP];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = Pin::new(self.inner_mut()).poll_write(context, &chunk[..pending]);
            wipe_bytes(&mut chunk[..pending]);
            match result {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64-ng-tokio decoder writer could not drain buffered output",
                    )));
                }
                Poll::Ready(Ok(written)) => {
                    if written > pending {
                        self.failed = true;
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "wrapped async writer reported more bytes than provided",
                        )));
                    }
                    self.output.discard_front(written);
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            }
        }

        Poll::Ready(Ok(()))
    }
}

impl<W, A, const PAD: bool> AsyncWrite for DecoderWriter<W, A, PAD>
where
    W: AsyncWrite + Unpin,
    A: Alphabet + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio decoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        if self.finalized {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64-ng-tokio decoder writer received input after shutdown",
            )));
        }

        Poll::Ready(self.process_input(input))
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio decoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        Pin::new(self.inner_mut()).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio decoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        if !self.finalized {
            if let Err(error) = self.queue_pending_final() {
                self.clear_output();
                return Poll::Ready(Err(error));
            }
            self.finalized = true;
        }
        ready!(self.poll_drain_output(context))?;
        ready!(Pin::new(self.inner_mut()).poll_flush(context))?;
        Pin::new(self.inner_mut()).poll_shutdown(context)
    }
}
