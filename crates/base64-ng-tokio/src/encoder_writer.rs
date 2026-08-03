use base64_ng::{Base64, Codec, EncoderState, Status};
use core::{
    pin::Pin,
    task::{Context, Poll, ready},
};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use tokio::io::{self, AsyncWrite};

use crate::{operation_io_error, queue::OutputQueue, wipe_bytes};

const ENCODE_INPUT_CAP: usize = 768;
const ENCODE_OUTPUT_CAP: usize = 1024;

/// Async writer that accepts raw bytes and writes Base64 to the wrapped writer.
///
/// `poll_write` drains previously accepted output before accepting another
/// bounded input prefix. Once it returns `Ready(Ok(n))`, those `n` input bytes
/// are owned by this adapter and represented either by the shared incremental
/// encoder or the fixed output queue. `Poll::Pending` never accepts new input.
///
/// Output bytes accepted by the wrapped writer are irrevocably committed.
/// Cancellation while this adapter remains alive preserves all uncommitted
/// output. Dropping the adapter discards and clears uncommitted state, as is
/// normal for a buffered writer; call `shutdown` before recovery when a
/// complete stream is required.
///
/// Ordinary wrapped-writer I/O errors are retryable and retain queued output.
/// Internal transform, accounting, or wrapped-writer protocol violations latch
/// an absorbing failure and clear retained state.
pub struct EncoderWriter<W> {
    inner: Option<W>,
    state: EncoderState,
    output: OutputQueue<ENCODE_OUTPUT_CAP>,
    input_accepted: usize,
    output_committed: usize,
    finalized: bool,
    shutdown_complete: bool,
    failed: bool,
}

impl<W> EncoderWriter<W> {
    /// Creates an async writer over the selected 2.0 codec specification.
    #[must_use]
    pub fn new<S: Codec>(inner: W, codec: &Base64<S>) -> Self {
        Self {
            inner: Some(inner),
            state: codec.encoder(),
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
    /// Writing directly to it while this adapter has buffered output can
    /// reorder the stream.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }

    /// Returns whether this adapter has entered its absorbing failure state.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether Base64 finalization has completed.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether the wrapped writer completed shutdown.
    #[must_use]
    pub const fn is_shutdown(&self) -> bool {
        self.shutdown_complete
    }

    /// Returns raw input bytes accepted by this adapter.
    #[must_use]
    pub const fn input_accepted(&self) -> usize {
        self.input_accepted
    }

    /// Returns encoded bytes accepted by the wrapped writer.
    #[must_use]
    pub const fn output_committed(&self) -> usize {
        self.output_committed
    }

    /// Returns raw input bytes retained until a complete encode quantum.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.state.buffered_input_len()
    }

    /// Returns encoded bytes queued for the wrapped writer.
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
    /// Accepted input or queued output may be discarded. Prefer `shutdown` and
    /// [`Self::try_into_inner`] when completeness matters.
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
    /// input or output.
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

    fn process_input(&mut self, input: &[u8]) -> io::Result<usize> {
        let offered = input.len().min(ENCODE_INPUT_CAP);
        if offered == 0 {
            return Ok(0);
        }

        let mut encoded = [0u8; ENCODE_OUTPUT_CAP];
        let result = self.state.update(&input[..offered], &mut encoded);
        let step = match result {
            Ok(step) => step,
            Err(error) => {
                wipe_bytes(&mut encoded);
                self.latch_failure();
                return Err(operation_io_error(error));
            }
        };
        let progress = step.progress();
        let consumed = progress.input_consumed();
        let produced = progress.output_produced();
        if consumed == 0 || consumed > offered || produced > encoded.len() {
            wipe_bytes(&mut encoded);
            self.latch_failure();
            return Err(io::Error::other(
                "base64-ng-tokio encoder made invalid progress",
            ));
        }
        if let Err(error) = self.output.push_slice(&encoded[..produced]) {
            wipe_bytes(&mut encoded);
            self.latch_failure();
            return Err(error);
        }
        wipe_bytes(&mut encoded);
        self.input_accepted = self.state.source_position();
        Ok(consumed)
    }

    fn finalize(&mut self) -> io::Result<()> {
        if self.finalized {
            return Ok(());
        }
        let mut encoded = [0u8; 4];
        let result = self.state.finish(&mut encoded);
        let step = match result {
            Ok(step) => step,
            Err(error) => {
                wipe_bytes(&mut encoded);
                self.latch_failure();
                return Err(operation_io_error(error));
            }
        };
        let produced = step.progress().output_produced();
        if produced > encoded.len() || step.status() != Status::Complete {
            wipe_bytes(&mut encoded);
            self.latch_failure();
            return Err(io::Error::other(
                "base64-ng-tokio encoder finalization made invalid progress",
            ));
        }
        if let Err(error) = self.output.push_slice(&encoded[..produced]) {
            wipe_bytes(&mut encoded);
            self.latch_failure();
            return Err(error);
        }
        wipe_bytes(&mut encoded);
        self.finalized = true;
        Ok(())
    }
}

impl<W> Drop for EncoderWriter<W> {
    fn drop(&mut self) {
        self.clear_internal();
    }
}

impl<W> EncoderWriter<W>
where
    W: AsyncWrite + Unpin,
{
    fn poll_inner_flush(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            Pin::new(self.inner_mut()).poll_flush(context)
        }));
        match result {
            Ok(polled) => polled,
            Err(payload) => {
                self.latch_failure();
                resume_unwind(payload);
            }
        }
    }

    fn poll_inner_shutdown(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            Pin::new(self.inner_mut()).poll_shutdown(context)
        }));
        match result {
            Ok(polled) => polled,
            Err(payload) => {
                self.latch_failure();
                resume_unwind(payload);
            }
        }
    }

    fn poll_drain_output(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut chunk = [0u8; ENCODE_OUTPUT_CAP];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = catch_unwind(AssertUnwindSafe(|| {
                Pin::new(self.inner_mut()).poll_write(context, &chunk[..pending])
            }));
            wipe_bytes(&mut chunk[..pending]);
            let result = match result {
                Ok(polled) => polled,
                Err(payload) => {
                    self.latch_failure();
                    resume_unwind(payload);
                }
            };
            match result {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64-ng-tokio encoder could not drain buffered output",
                    )));
                }
                Poll::Ready(Ok(written)) if written <= pending => {
                    let Some(committed) = self.output_committed.checked_add(written) else {
                        self.latch_failure();
                        return Poll::Ready(Err(io::Error::other(
                            "base64-ng-tokio encoder output position overflow",
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

impl<W> AsyncWrite for EncoderWriter<W>
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
                "base64-ng-tokio encoder writer is failed",
            )));
        }
        if self.shutdown_complete || self.finalized {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64-ng-tokio encoder writer received input after shutdown",
            )));
        }
        ready!(self.poll_drain_output(context))?;
        Poll::Ready(self.process_input(input))
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio encoder writer is failed",
            )));
        }
        if self.shutdown_complete {
            return Poll::Ready(Ok(()));
        }
        ready!(self.poll_drain_output(context))?;
        self.poll_inner_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio encoder writer is failed",
            )));
        }
        if self.shutdown_complete {
            return Poll::Ready(Ok(()));
        }
        ready!(self.poll_drain_output(context))?;
        self.finalize()?;
        ready!(self.poll_drain_output(context))?;
        ready!(self.poll_inner_flush(context))?;
        ready!(self.poll_inner_shutdown(context))?;
        self.shutdown_complete = true;
        Poll::Ready(Ok(()))
    }
}
