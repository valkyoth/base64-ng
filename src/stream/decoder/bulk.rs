use super::Decoder;
use crate::{Alphabet, Failure, OperationError};
use std::io;

impl<W, A: Alphabet, const PAD: bool> Decoder<W, A, PAD> {
    pub(super) fn queue_bulk(&mut self, input: &[u8]) -> io::Result<usize> {
        // Preserve the old quantum-sized acceptance ceiling, including pending
        // input. Never let the core buffer an extra quantum past the queue.
        let pending = self.pending_len();
        let quanta =
            ((input.len().saturating_add(pending)) / 4).min(self.output.available_capacity() / 3);
        let take = (quanta * 4).saturating_sub(pending);
        if take < 516 {
            return Ok(0);
        }
        let mut candidate = self.driver.clone();
        let mut decoded = [0u8; 1024];
        let result = candidate.update(&input[..take], &mut decoded);
        let step = match result {
            Ok(step) => step,
            Err(error) => {
                crate::wipe_bytes(&mut decoded);
                candidate.wipe();
                // Malformed input must retain the historical accepted prefix
                // and exact error: replay through the unchanged quantum loop.
                if matches!(
                    error
                        .get_ref()
                        .and_then(|cause| cause.downcast_ref::<OperationError>()),
                    Some(OperationError::Failed(Failure::Input(_)))
                ) {
                    return Ok(0);
                }
                self.failed = true;
                self.clear_pending();
                return Err(error);
            }
        };
        let progress = step.progress();
        if progress.input_consumed() != take || progress.output_produced() > quanta * 3 {
            crate::wipe_bytes(&mut decoded);
            candidate.wipe();
            self.failed = true;
            self.clear_pending();
            return Err(io::Error::other(
                "base64 stream bulk decoder made invalid progress",
            ));
        }
        let queued = self
            .output
            .push_slice(&decoded[..progress.output_produced()]);
        crate::wipe_bytes(&mut decoded);
        if let Err(error) = queued {
            candidate.wipe();
            self.failed = true;
            self.clear_pending();
            return Err(error);
        }
        self.driver.wipe();
        self.driver = candidate;
        self.finished = self.driver.has_terminal_padding();
        Ok(take)
    }
}
