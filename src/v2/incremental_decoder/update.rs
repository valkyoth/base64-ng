//! Per-call planning binds an optional bulk proof before any output mutation.
use super::{DecoderState, INPUT_QUANTUM, InputMode, decode_quantum, validate_partial_symbol};
use crate::{
    DecodeValidation, OneShotError,
    decode_preflight::Preflight,
    v2::{
        contracts::{
            BackendFault, Failure, InputError, OperationError, Progress, SourceSpan, Step,
        },
        ordinary_decode::{self, Prepared},
        ordinary_scalar::Family,
    },
};
use core::num::NonZeroUsize;

struct Bulk<'a> {
    start: usize,
    proof: Preflight<'a, Prepared>,
}

struct UpdatePlan<'a> {
    consumed: usize,
    bulk: Option<Bulk<'a>>,
}

impl DecoderState {
    /// Accepts a strict input prefix and writes decoded output that fits.
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<Step, OperationError> {
        self.update_with_validation(input, output, DecodeValidation::Auto)
    }

    /// Uses an explicit validation policy for input accepted by this call.
    ///
    /// The complete accepted prefix is validated before any destination write,
    /// including draining previously pending output. Earlier successful calls
    /// cannot be rolled back. Pending output was validated by its original call;
    /// this policy does not retroactively revalidate previously accepted bytes.
    /// Finish always applies the existing scalar terminal-quantum rules.
    pub fn update_with_validation(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<Step, OperationError> {
        if input.len() < 516 || output.len() < 384 {
            self.update_impl::<false>(input, output, validation)
        } else {
            self.update_impl::<true>(input, output, validation)
        }
    }

    // The independent validator must never recursively select the bulk route.
    pub(crate) fn update_reference(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Step, OperationError> {
        self.update_impl::<false>(input, output, DecodeValidation::ScalarReference)
    }

    fn update_impl<const BULK: bool>(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<Step, OperationError> {
        let span = self.lifecycle.reserve_input(input.len())?;
        let plan = match self.plan_update::<BULK>(input, output.len(), span, validation) {
            Ok(plan) => plan,
            Err(failure) => return Err(self.lifecycle.fail(failure)),
        };
        let consumed = plan.consumed;
        let mut bulk = plan.bulk;
        self.lifecycle.commit_input(span, consumed)?;

        let mut produced = self.drain_pending(output);
        let source_start = self.lifecycle.source_position() - consumed;
        let mut input_offset = 0;
        while input_offset < consumed {
            if BULK
                && bulk.as_ref().is_some_and(|bulk| bulk.start == input_offset)
                && let Some(bulk) = bulk.take()
            {
                let len = bulk.proof.input().len();
                let written = bulk.proof.len();
                // Planning reserved this exact disjoint output span. The only
                // write error is a capacity mismatch, excluded by that plan.
                if ordinary_decode::write(bulk.proof, &mut output[produced..produced + written])
                    .is_err()
                {
                    return Err(self
                        .lifecycle
                        .fail(Failure::Backend(BackendFault::ImpossibleState)));
                }
                input_offset += len;
                produced += written;
                self.quantum
                    .copy_from_slice(&input[input_offset - 4..input_offset]);
                for (lane, index) in self.quantum_indexes.iter_mut().enumerate() {
                    *index = source_start + input_offset - 4 + lane;
                }
                self.pending
                    .copy_from_slice(&output[produced - 3..produced]);
                continue;
            }
            if self.ignores(input[input_offset]) {
                input_offset += 1;
                continue;
            }
            self.quantum[self.quantum_len] = input[input_offset];
            self.quantum_indexes[self.quantum_len] = source_start + input_offset;
            self.quantum_len += 1;
            input_offset += 1;

            if self.quantum_len == INPUT_QUANTUM {
                let Ok(decoded) = decode_quantum(self.settings, self.quantum, self.quantum_indexes)
                else {
                    return Err(self
                        .lifecycle
                        .fail(Failure::Backend(BackendFault::ImpossibleState)));
                };
                self.pending = decoded.bytes;
                self.pending_start = 0;
                self.pending_len = decoded.len;
                self.terminal_padding = decoded.terminal_padding;
                self.quantum_len = 0;
                produced += self.drain_pending(&mut output[produced..]);
            }
        }

        let progress = Progress::new(consumed, produced);
        if self.pending_len != 0 || consumed != input.len() {
            self.lifecycle.output_full(progress, NonZeroUsize::MIN)
        } else {
            self.lifecycle.need_input(progress)
        }
    }

    #[inline]
    fn plan_update<'a, const BULK: bool>(
        &self,
        input: &'a [u8],
        output_len: usize,
        span: SourceSpan,
        validation: DecodeValidation,
    ) -> Result<UpdatePlan<'a>, Failure> {
        let mut bulk = None;
        let mut attempted = !BULK || self.input_mode != InputMode::Strict;
        let pending_written = self.pending_len.min(output_len);
        let mut pending = self.pending_len - pending_written;
        if pending != 0 {
            return Ok(UpdatePlan { consumed: 0, bulk });
        }

        let mut available_output = output_len - pending_written;
        let mut quantum = self.quantum;
        let mut indexes = self.quantum_indexes;
        let mut quantum_len = self.quantum_len;
        let mut terminal_padding = self.terminal_padding;
        let mut consumed = 0;

        while consumed < input.len() {
            if !attempted && quantum_len == 0 && !terminal_padding {
                attempted = true;
                // Keep the final quantum with the state-machine padding logic.
                // Never inspect beyond the full quanta that fit this output.
                let quanta =
                    ((input.len() - consumed).saturating_sub(4) / 4).min(available_output / 3);
                if quanta >= 128 && Family::for_settings(self.settings).is_some() {
                    let bytes = &input[consumed..consumed + quanta * 4];
                    match ordinary_decode::prepare(self.settings, bytes, validation) {
                        Ok(proof) if proof.len() == quanta * 3 => {
                            bulk = Some(Bulk {
                                start: consumed,
                                proof,
                            });
                            consumed += quanta * 4;
                            available_output -= quanta * 3;
                            continue;
                        }
                        // Preserve incremental diagnostics and original indexes.
                        // Try bulk only once: malformed input stays linear.
                        Ok(_) | Err(OneShotError::Input(_)) => {}
                        Err(OneShotError::Backend(fault)) => return Err(Failure::Backend(fault)),
                        Err(_) => return Err(Failure::Backend(BackendFault::ImpossibleState)),
                    }
                }
            }
            let index = span
                .index(consumed)
                .ok_or(Failure::Backend(BackendFault::ImpossibleState))?;
            if self.ignores(input[consumed]) {
                consumed += 1;
                continue;
            }
            if terminal_padding {
                return Err(Failure::Input(InputError::TrailingData { index }));
            }

            validate_partial_symbol(
                self.settings,
                quantum,
                &indexes,
                quantum_len,
                input[consumed],
                index,
            )
            .map_err(Failure::Input)?;
            quantum[quantum_len] = input[consumed];
            indexes[quantum_len] = index;
            quantum_len += 1;
            consumed += 1;

            if quantum_len == INPUT_QUANTUM {
                let decoded =
                    decode_quantum(self.settings, quantum, indexes).map_err(Failure::Input)?;
                quantum_len = 0;
                terminal_padding = decoded.terminal_padding;
                let written = decoded.len.min(available_output);
                available_output -= written;
                pending = decoded.len - written;
                if pending != 0 {
                    break;
                }
            }
        }
        Ok(UpdatePlan { consumed, bulk })
    }

    /// Fuzz-only access to the original scalar planner and writer.
    #[cfg(fuzzing)]
    #[doc(hidden)]
    pub fn update_scalar_oracle(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Step, OperationError> {
        self.update_reference(input, output)
    }
}

#[cfg(test)]
mod tests;
