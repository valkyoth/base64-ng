use base64_ng::{Base64, Codec, DecoderState, Status};
use core::{
    pin::Pin,
    task::{Context, Poll, ready},
};
use tokio::io::{self, AsyncWrite};

use crate::{operation_io_error, queue::OutputQueue, wipe_bytes};

const DECODE_INPUT_CAP: usize = 1364;
const DECODE_OUTPUT_CAP: usize = 1024;

/// Async writer that accepts Base64 and writes decoded bytes to `inner`.
///
/// `poll_write` drains previously accepted output before accepting another
/// bounded encoded prefix. Once it returns `Ready(Ok(n))`, those `n` input
/// bytes are retained by the shared strict decoder or represented by queued
/// plaintext. `Poll::Pending` never accepts new input.
///
/// This is an ordinary prefix-delivering decoder. Plaintext accepted by the
/// wrapped writer is irrevocably exposed even if a later suffix is malformed.
/// Secret-bearing frames must use a bounded validate-before-release secret API;
/// this adapter intentionally provides no unbounded secret mode.
///
/// Cancellation while this adapter remains alive preserves queued plaintext.
/// Dropping it clears and discards uncommitted state. Ordinary wrapped-writer
/// I/O errors are retryable; malformed input and internal protocol violations
/// latch an absorbing failure and clear retained state.
pub struct DecoderWriter<W> {
    inner: Option<W>,
    state: DecoderState,
    output: OutputQueue<DECODE_OUTPUT_CAP>,
    input_accepted: usize,
    output_committed: usize,
    finalized: bool,
    shutdown_complete: bool,
    failed: bool,
}

impl<W> DecoderWriter<W> {
    /// Creates an async strict decoder over the selected 2.0 codec.
    #[must_use]
    pub fn new<S: Codec>(inner: W, codec: &Base64<S>) -> Self {
        Self {
            inner: Some(inner),
            state: codec.decoder(),
            output: OutputQueue::new(),
            input_accepted: 0,
            output_committed: 0,
            finalized: false,
            shutdown_complete: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped writer.
    #[must_use]
    pub fn get_ref(&self) -> &W {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped writer.
    ///
    /// Writing directly to it while plaintext is buffered can reorder output.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }

    /// Returns whether this adapter has entered its absorbing failure state.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether strict Base64 finalization has completed.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether the wrapped writer completed shutdown.
    #[must_use]
    pub const fn is_shutdown(&self) -> bool {
        self.shutdown_complete
    }

    /// Returns whether a terminal padded quantum has been accepted.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.state.has_terminal_padding()
    }

    /// Returns encoded input bytes accepted by this adapter.
    #[must_use]
    pub const fn input_accepted(&self) -> usize {
        self.input_accepted
    }

    /// Returns plaintext bytes accepted by the wrapped writer.
    #[must_use]
    pub const fn output_committed(&self) -> usize {
        self.output_committed
    }

    /// Returns encoded input bytes retained until a complete quantum.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.state.buffered_input_len()
    }

    /// Returns plaintext bytes queued for the wrapped writer.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns whether checked recovery can avoid discarding accepted input.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.failed && self.pending_len() == 0 && self.output.is_empty()
    }

    /// Consumes the adapter and returns the wrapped writer without finalizing.
    ///
    /// Accepted encoded input or queued plaintext may be discarded. Prefer
    /// `shutdown` and [`Self::try_into_inner`] when completeness matters.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Recovers the wrapped writer only when no accepted input would be lost.
    ///
    /// This does not flush or shut down the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns this adapter unchanged when it has failed or retains pending
    /// encoded input or queued plaintext.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn clear_internal(&mut self) {
        self.state.clear();
        self.output.clear_all();
    }

    fn latch_failure(&mut self) {
        self.failed = true;
        self.clear_internal();
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

    fn process_input(&mut self, input: &[u8]) -> io::Result<usize> {
        let offered = input.len().min(DECODE_INPUT_CAP);
        if offered == 0 {
            return Ok(0);
        }

        let mut decoded = [0u8; DECODE_OUTPUT_CAP];
        let result = self.state.update(&input[..offered], &mut decoded);
        let step = match result {
            Ok(step) => step,
            Err(error) => {
                wipe_bytes(&mut decoded);
                self.latch_failure();
                return Err(operation_io_error(error));
            }
        };
        let progress = step.progress();
        let consumed = progress.input_consumed();
        let produced = progress.output_produced();
        if consumed == 0 || consumed > offered || produced > decoded.len() {
            wipe_bytes(&mut decoded);
            self.latch_failure();
            return Err(io::Error::other(
                "base64-ng-tokio decoder made invalid progress",
            ));
        }
        if let Err(error) = self.output.push_slice(&decoded[..produced]) {
            wipe_bytes(&mut decoded);
            self.latch_failure();
            return Err(error);
        }
        wipe_bytes(&mut decoded);
        self.input_accepted = self.state.source_position();
        Ok(consumed)
    }

    fn finalize(&mut self) -> io::Result<()> {
        if self.finalized {
            return Ok(());
        }
        let mut decoded = [0u8; 3];
        let result = self.state.finish(&mut decoded);
        let step = match result {
            Ok(step) => step,
            Err(error) => {
                wipe_bytes(&mut decoded);
                self.latch_failure();
                return Err(operation_io_error(error));
            }
        };
        let produced = step.progress().output_produced();
        if produced > decoded.len() || step.status() != Status::Complete {
            wipe_bytes(&mut decoded);
            self.latch_failure();
            return Err(io::Error::other(
                "base64-ng-tokio decoder finalization made invalid progress",
            ));
        }
        if let Err(error) = self.output.push_slice(&decoded[..produced]) {
            wipe_bytes(&mut decoded);
            self.latch_failure();
            return Err(error);
        }
        wipe_bytes(&mut decoded);
        self.finalized = true;
        Ok(())
    }
}

impl<W> Drop for DecoderWriter<W> {
    fn drop(&mut self) {
        self.clear_internal();
    }
}

impl<W> DecoderWriter<W>
where
    W: AsyncWrite + Unpin,
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
                        "base64-ng-tokio decoder could not drain buffered output",
                    )));
                }
                Poll::Ready(Ok(written)) if written <= pending => {
                    let Some(committed) = self.output_committed.checked_add(written) else {
                        self.latch_failure();
                        return Poll::Ready(Err(io::Error::other(
                            "base64-ng-tokio decoder output position overflow",
                        )));
                    };
                    self.output.discard_front(written);
                    self.output_committed = committed;
                }
                Poll::Ready(Ok(_)) => {
                    self.latch_failure();
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrapped async writer reported more bytes than provided",
                    )));
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl<W> AsyncWrite for DecoderWriter<W>
where
    W: AsyncWrite + Unpin,
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
        if self.shutdown_complete || self.finalized {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64-ng-tokio decoder writer received input after shutdown",
            )));
        }
        ready!(self.poll_drain_output(context))?;
        Poll::Ready(self.process_input(input))
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio decoder writer is failed",
            )));
        }
        if self.shutdown_complete {
            return Poll::Ready(Ok(()));
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
        if self.shutdown_complete {
            return Poll::Ready(Ok(()));
        }
        ready!(self.poll_drain_output(context))?;
        self.finalize()?;
        ready!(self.poll_drain_output(context))?;
        ready!(Pin::new(self.inner_mut()).poll_flush(context))?;
        ready!(Pin::new(self.inner_mut()).poll_shutdown(context))?;
        self.shutdown_complete = true;
        Poll::Ready(Ok(()))
    }
}
