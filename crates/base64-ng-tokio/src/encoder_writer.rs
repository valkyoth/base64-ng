use base64_ng::{Alphabet, Engine};
use core::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tokio::io::{self, AsyncWrite};

use crate::{encode_io_error, queue::OutputQueue, wipe_bytes};

const ENCODE_INPUT_CAP: usize = 768;
const ENCODE_OUTPUT_CAP: usize = 1024;
/// Async writer that accepts raw bytes and writes Base64 to the wrapped writer.
///
/// `poll_write` may accept only part of the input, following normal
/// [`AsyncWrite`] rules. Accepted bytes may remain buffered internally until
/// a later write, [`AsyncWrite::poll_flush`], or [`AsyncWrite::poll_shutdown`].
/// Shutdown is the finalization boundary: it encodes any trailing partial
/// quantum, drains all buffered output, flushes, and then shuts down `inner`.
///
/// # Security
///
/// Internal cleanup is best-effort and limited to this adapter's fixed pending
/// and output buffers. It cannot clear copies held by the wrapped writer, the
/// caller's buffers, registers, caches, swap, or crash dumps.
///
/// I/O errors from the wrapped writer during drain do not set [`Self::is_failed`];
/// only internal protocol or capacity violations latch a permanent failure.
pub struct EncoderWriter<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: OutputQueue<ENCODE_OUTPUT_CAP>,
    finalized: bool,
    failed: bool,
    _alphabet: PhantomData<A>,
}

impl<W, A, const PAD: bool> EncoderWriter<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new async Base64 encoder writer.
    #[must_use]
    pub fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: OutputQueue::new(),
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
    /// This does not finalize pending input. Prefer
    /// [`AsyncWriteExt::shutdown`](tokio::io::AsyncWriteExt::shutdown) before
    /// calling this when the Base64 stream must be complete.
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

    /// Returns the number of raw bytes buffered until a full encode quantum is
    /// available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns the number of encoded bytes currently buffered for `inner`.
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
            None => unreachable!("tokio encoder writer inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("tokio encoder writer inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("tokio encoder writer inner writer was already taken"),
        }
    }

    fn queue_encoded_temp(&mut self, input: &[u8], encoded: &mut [u8]) -> io::Result<()> {
        let written = match self.engine.encode_slice(input, encoded) {
            Ok(written) => written,
            Err(error) => {
                wipe_bytes(encoded);
                self.failed = true;
                return Err(encode_io_error(error));
            }
        };

        let result = self.output.push_slice(&encoded[..written]);
        wipe_bytes(encoded);
        if result.is_err() {
            self.failed = true;
        }
        result
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
        wipe_bytes(&mut pending);
        result?;
        self.clear_pending();
        Ok(())
    }

    fn process_input(&mut self, input: &[u8]) -> io::Result<usize> {
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

            let mut quantum = [0u8; 3];
            quantum[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            quantum[self.pending_len..].copy_from_slice(&input[..needed]);
            let mut encoded = [0u8; 4];
            let result = self.queue_encoded_temp(&quantum, &mut encoded);
            wipe_bytes(&mut quantum);
            result?;
            self.clear_pending();
            consumed += needed;
        }

        let remaining = &input[consumed..];
        let full_len = remaining.len() / 3 * 3;
        if full_len != 0 {
            let max_by_queue = self.output.available_capacity() / 4 * 3;
            let mut take = core::cmp::min(full_len, core::cmp::min(ENCODE_INPUT_CAP, max_by_queue));
            take -= take % 3;

            if take == 0 {
                return Ok(consumed);
            }

            let mut encoded = [0u8; ENCODE_OUTPUT_CAP];
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
}

impl<W, A, const PAD: bool> Drop for EncoderWriter<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> EncoderWriter<W, A, PAD>
where
    W: AsyncWrite + Unpin,
    A: Alphabet + Unpin,
{
    fn poll_drain_output(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut chunk = [0u8; ENCODE_OUTPUT_CAP];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = Pin::new(self.inner_mut()).poll_write(context, &chunk[..pending]);
            wipe_bytes(&mut chunk[..pending]);
            match result {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64-ng-tokio encoder writer could not drain buffered output",
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

impl<W, A, const PAD: bool> AsyncWrite for EncoderWriter<W, A, PAD>
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
                "base64-ng-tokio encoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        if self.finalized {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64-ng-tokio encoder writer received input after shutdown",
            )));
        }

        Poll::Ready(self.process_input(input))
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio encoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        Pin::new(self.inner_mut()).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio encoder writer is failed",
            )));
        }

        ready!(self.poll_drain_output(context))?;
        if !self.finalized {
            if let Err(error) = self.queue_pending_final() {
                self.failed = true;
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
