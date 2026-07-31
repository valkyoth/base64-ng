//! Heapless strict padded incremental ordinary decoding.

use core::num::NonZeroUsize;

use super::{
    contracts::{
        BackendFault, Failure, InputError, Lifecycle, OperationError, Progress, SourceSpan, Step,
    },
    specifications::{Base64, CodecSettings, StrictStandardPadded, StrictUrlSafePadded},
};

const INPUT_QUANTUM: usize = 4;
const OUTPUT_QUANTUM: usize = 3;

/// Heapless strict padded Base64 decoder state.
///
/// The current input quantum and pending output quantum are mutually
/// exclusive. This keeps retry state bounded while allowing one-byte output
/// destinations without asking the caller to replay accepted input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DecoderState {
    settings: CodecSettings,
    quantum: [u8; INPUT_QUANTUM],
    quantum_indexes: [usize; INPUT_QUANTUM],
    quantum_len: usize,
    pending: [u8; OUTPUT_QUANTUM],
    pending_start: usize,
    pending_len: usize,
    terminal_padding: bool,
    lifecycle: Lifecycle,
}

impl DecoderState {
    /// Constructs a decoder whose caller selected canonical padded decoding.
    pub(crate) const fn new_padded(settings: CodecSettings) -> Self {
        Self {
            settings,
            quantum: [0; INPUT_QUANTUM],
            quantum_indexes: [0; INPUT_QUANTUM],
            quantum_len: 0,
            pending: [0; OUTPUT_QUANTUM],
            pending_start: 0,
            pending_len: 0,
            terminal_padding: false,
            lifecycle: Lifecycle::new(),
        }
    }

    /// Accepts a strict padded input prefix and writes decoded output that fits.
    pub(crate) fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Step, OperationError> {
        let span = self.lifecycle.reserve_input(input.len())?;
        let consumed = match self.plan_update(input, output.len(), span) {
            Ok(consumed) => consumed,
            Err(failure) => return Err(self.lifecycle.fail(failure)),
        };
        self.lifecycle.commit_input(span, consumed)?;

        let mut produced = self.drain_pending(output);
        let source_start = self.lifecycle.source_position() - consumed;
        let mut input_offset = 0;
        while input_offset < consumed {
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

    /// Declares end of input and rejects an incomplete padded quantum.
    pub(crate) fn finish(&mut self, output: &mut [u8]) -> Result<Step, OperationError> {
        match self.lifecycle.need_input(Progress::ZERO) {
            Ok(_) => {}
            Err(OperationError::Terminal(_)) => {
                return self.lifecycle.finish(Progress::ZERO);
            }
            Err(error) => return Err(error),
        }

        let produced = self.drain_pending(output);
        let progress = Progress::new(0, produced);
        if self.pending_len != 0 {
            return self.lifecycle.output_full(progress, NonZeroUsize::MIN);
        }
        if self.quantum_len != 0 {
            let failure = Failure::Input(InputError::TruncatedInput {
                index: self.lifecycle.source_position(),
            });
            return Err(self.lifecycle.fail(failure));
        }
        self.lifecycle.finish(progress)
    }

    /// Resets the state for one unrelated ordinary message.
    pub(crate) fn reset(&mut self) {
        self.quantum = [0; INPUT_QUANTUM];
        self.quantum_indexes = [0; INPUT_QUANTUM];
        self.quantum_len = 0;
        self.pending = [0; OUTPUT_QUANTUM];
        self.pending_start = 0;
        self.pending_len = 0;
        self.terminal_padding = false;
        self.lifecycle.reset();
    }

    pub(crate) const fn source_position(&self) -> usize {
        self.lifecycle.source_position()
    }

    fn plan_update(
        &self,
        input: &[u8],
        output_len: usize,
        span: SourceSpan,
    ) -> Result<usize, Failure> {
        let pending_written = self.pending_len.min(output_len);
        let mut pending = self.pending_len - pending_written;
        if pending != 0 {
            return Ok(0);
        }

        let mut available_output = output_len - pending_written;
        let mut quantum = self.quantum;
        let mut indexes = self.quantum_indexes;
        let mut quantum_len = self.quantum_len;
        let mut terminal_padding = self.terminal_padding;
        let mut consumed = 0;

        while consumed < input.len() {
            let index = span
                .index(consumed)
                .ok_or(Failure::Backend(BackendFault::ImpossibleState))?;
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
        Ok(consumed)
    }

    fn drain_pending(&mut self, output: &mut [u8]) -> usize {
        let written = self.pending_len.min(output.len());
        let pending_end = self.pending_start + written;
        output[..written].copy_from_slice(&self.pending[self.pending_start..pending_end]);
        self.pending_start = pending_end;
        self.pending_len -= written;
        if self.pending_len == 0 {
            self.pending_start = 0;
        }
        written
    }

    #[cfg(kani)]
    pub(crate) fn proof_invariants(&self) -> bool {
        self.quantum_len < INPUT_QUANTUM
            && self.pending_start <= OUTPUT_QUANTUM
            && self.pending_len <= OUTPUT_QUANTUM
            && self.pending_start + self.pending_len <= OUTPUT_QUANTUM
            && !(self.quantum_len != 0 && self.pending_len != 0)
            && self.settings.decode_padding()
                == super::specifications::DecodePadding::RequireCanonical
            && self.settings.trailing_bits()
                == super::specifications::TrailingBits::RequireCanonical
    }
}

impl Base64<StrictStandardPadded> {
    /// Constructs a fresh strict Standard padded decoder.
    pub(crate) fn decoder(self) -> DecoderState {
        DecoderState::new_padded(self.settings())
    }
}

impl Base64<StrictUrlSafePadded> {
    /// Constructs a fresh strict URL-safe padded decoder.
    pub(crate) fn decoder(self) -> DecoderState {
        DecoderState::new_padded(self.settings())
    }
}

#[derive(Clone, Copy)]
struct DecodedQuantum {
    bytes: [u8; OUTPUT_QUANTUM],
    len: usize,
    terminal_padding: bool,
}

fn validate_partial_symbol(
    settings: CodecSettings,
    quantum: [u8; INPUT_QUANTUM],
    indexes: &[usize; INPUT_QUANTUM],
    position: usize,
    byte: u8,
    index: usize,
) -> Result<(), InputError> {
    match position {
        0 | 1 => decode_symbol(settings, byte, index).map(|_| ()),
        2 => {
            if byte == b'=' {
                Ok(())
            } else {
                decode_symbol(settings, byte, index).map(|_| ())
            }
        }
        3 if quantum[2] == b'=' && byte != b'=' => {
            Err(InputError::InvalidPadding { index: indexes[2] })
        }
        3 => {
            if byte == b'=' {
                Ok(())
            } else {
                decode_symbol(settings, byte, index).map(|_| ())
            }
        }
        _ => Err(InputError::InvalidLength),
    }
}

fn decode_quantum(
    settings: CodecSettings,
    input: [u8; INPUT_QUANTUM],
    indexes: [usize; INPUT_QUANTUM],
) -> Result<DecodedQuantum, InputError> {
    let first = decode_symbol(settings, input[0], indexes[0])?;
    let second = decode_symbol(settings, input[1], indexes[1])?;

    match (input[2], input[3]) {
        (b'=', b'=') => {
            if second & 0x0f != 0 {
                return Err(InputError::NonCanonicalTrailingBits { index: indexes[1] });
            }
            Ok(DecodedQuantum {
                bytes: [(first << 2) | (second >> 4), 0, 0],
                len: 1,
                terminal_padding: true,
            })
        }
        (b'=', _) => Err(InputError::InvalidPadding { index: indexes[2] }),
        (third, b'=') => {
            let third = decode_symbol(settings, third, indexes[2])?;
            if third & 0x03 != 0 {
                return Err(InputError::NonCanonicalTrailingBits { index: indexes[2] });
            }
            Ok(DecodedQuantum {
                bytes: [
                    (first << 2) | (second >> 4),
                    (second << 4) | (third >> 2),
                    0,
                ],
                len: 2,
                terminal_padding: true,
            })
        }
        (third, fourth) => {
            let third = decode_symbol(settings, third, indexes[2])?;
            let fourth = decode_symbol(settings, fourth, indexes[3])?;
            Ok(DecodedQuantum {
                bytes: [
                    (first << 2) | (second >> 4),
                    (second << 4) | (third >> 2),
                    (third << 6) | fourth,
                ],
                len: 3,
                terminal_padding: false,
            })
        }
    }
}

fn decode_symbol(settings: CodecSettings, byte: u8, index: usize) -> Result<u8, InputError> {
    match settings.alphabet().decode_byte(byte) {
        Some(value) => Ok(value),
        None if byte == b'=' => Err(InputError::InvalidPadding { index }),
        None => Err(InputError::InvalidByte { index, byte }),
    }
}
