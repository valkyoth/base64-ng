//! Heapless incremental WHATWG forgiving decoder.

use core::num::NonZeroUsize;

use super::ForgivingBase64;
use crate::v2::{OutputFull, Progress, Status, Step};

const INPUT_QUANTUM: usize = 4;
const OUTPUT_QUANTUM: usize = 3;

/// Opaque WHATWG decode, lifecycle, or resource failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ForgivingError {
    /// The string does not satisfy the WHATWG forgiving Base64 algorithm.
    InvalidInput,
    /// The absolute accepted source length cannot be represented by `usize`.
    PositionOverflow,
    /// Input was supplied after finalization started.
    InputAfterFinish,
    /// Input was supplied after successful completion.
    InputAfterComplete,
    /// The caller's one-shot destination is too small.
    OutputTooSmall {
        /// Exact required output bytes.
        required: usize,
        /// Available destination bytes.
        available: usize,
    },
    /// The exact allocating result exceeds a caller-selected limit.
    AllocationLimitExceeded {
        /// Exact required output bytes.
        required: usize,
        /// Maximum permitted output bytes.
        limit: usize,
    },
    /// Exact allocation reservation failed.
    AllocationFailed {
        /// Exact requested output bytes.
        requested: usize,
    },
}

impl core::fmt::Display for ForgivingError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid forgiving Base64 input"),
            Self::PositionOverflow => {
                formatter.write_str("forgiving Base64 source position overflows usize")
            }
            Self::InputAfterFinish => formatter.write_str("input supplied after finish started"),
            Self::InputAfterComplete => formatter.write_str("input supplied after completion"),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                formatter,
                "forgiving Base64 output too small: required {required}, available {available}"
            ),
            Self::AllocationLimitExceeded { required, limit } => write!(
                formatter,
                "forgiving Base64 output length {required} exceeds allocation limit {limit}"
            ),
            Self::AllocationFailed { requested } => write!(
                formatter,
                "failed to reserve {requested} forgiving Base64 output bytes"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ForgivingError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Phase {
    Active,
    Finishing,
    Complete,
    Failed(ForgivingError),
}

/// Heapless incremental WHATWG forgiving Base64 decoder.
///
/// State is bounded to one input quantum and one pending output quantum.
/// This ordinary compatibility state does not wipe on drop and must not be
/// used for secret-bearing input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForgivingDecoder {
    quantum: [u8; INPUT_QUANTUM],
    quantum_len: usize,
    pending: [u8; OUTPUT_QUANTUM],
    pending_start: usize,
    pending_len: usize,
    terminal_padding: bool,
    source_position: usize,
    phase: Phase,
}

impl ForgivingBase64 {
    /// Constructs a fresh heapless incremental decoder.
    #[must_use]
    pub const fn decoder(self) -> ForgivingDecoder {
        ForgivingDecoder::new()
    }
}

impl ForgivingDecoder {
    const fn new() -> Self {
        Self {
            quantum: [0; INPUT_QUANTUM],
            quantum_len: 0,
            pending: [0; OUTPUT_QUANTUM],
            pending_start: 0,
            pending_len: 0,
            terminal_padding: false,
            source_position: 0,
            phase: Phase::Active,
        }
    }

    /// Accepts one string fragment and writes every decoded byte that fits.
    ///
    /// `input_consumed` counts UTF-8 bytes. Every accepted WHATWG symbol is
    /// ASCII, so a successful partial count is always a valid string boundary.
    pub fn update(&mut self, input: &str, output: &mut [u8]) -> Result<Step, ForgivingError> {
        self.require_active()?;
        self.source_position
            .checked_add(input.len())
            .ok_or_else(|| self.fail(ForgivingError::PositionOverflow))?;
        let consumed = match self.plan_update(input, output.len()) {
            Ok(consumed) => consumed,
            Err(error) => return Err(self.fail(error)),
        };

        let mut produced = self.drain_pending(output);
        if self.pending_len != 0 {
            return Ok(output_full(0, produced));
        }

        for &byte in &input.as_bytes()[..consumed] {
            if is_ascii_whitespace(byte) {
                continue;
            }

            self.quantum[self.quantum_len] = byte;
            self.quantum_len += 1;
            if self.quantum_len == INPUT_QUANTUM {
                let Some((bytes, len, terminal)) = decode_quantum(self.quantum) else {
                    return Err(self.fail(ForgivingError::InvalidInput));
                };
                self.quantum_len = 0;
                self.pending = bytes;
                self.pending_start = 0;
                self.pending_len = len;
                self.terminal_padding = terminal;
                produced += self.drain_pending(&mut output[produced..]);
                if self.pending_len != 0 {
                    break;
                }
            }
        }
        self.source_position += consumed;

        if self.pending_len != 0 || consumed != input.len() {
            Ok(output_full(consumed, produced))
        } else {
            Ok(Step::new(
                Progress::new(consumed, produced),
                Status::NeedInput,
            ))
        }
    }

    fn plan_update(&self, input: &str, output_len: usize) -> Result<usize, ForgivingError> {
        let pending_written = self.pending_len.min(output_len);
        if self.pending_len != pending_written {
            return Ok(0);
        }

        let mut available_output = output_len - pending_written;
        let mut quantum = self.quantum;
        let mut quantum_len = self.quantum_len;
        let mut terminal_padding = self.terminal_padding;
        let mut consumed = 0;

        for &byte in input.as_bytes() {
            if is_ascii_whitespace(byte) {
                consumed += 1;
                continue;
            }
            if terminal_padding
                || (!is_standard_symbol(byte) && byte != b'=')
                || (byte == b'=' && quantum_len < 2)
                || (quantum_len == 3 && quantum[2] == b'=' && byte != b'=')
            {
                return Err(ForgivingError::InvalidInput);
            }

            quantum[quantum_len] = byte;
            quantum_len += 1;
            consumed += 1;
            if quantum_len == INPUT_QUANTUM {
                let Some((_, decoded_len, terminal)) = decode_quantum(quantum) else {
                    return Err(ForgivingError::InvalidInput);
                };
                quantum_len = 0;
                terminal_padding = terminal;
                let written = decoded_len.min(available_output);
                available_output -= written;
                if written != decoded_len {
                    break;
                }
            }
        }
        Ok(consumed)
    }

    /// Finalizes omitted padding and drains any bounded pending output.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<Step, ForgivingError> {
        match self.phase {
            Phase::Active => self.phase = Phase::Finishing,
            Phase::Finishing => {}
            Phase::Complete => return Ok(Step::new(Progress::ZERO, Status::Complete)),
            Phase::Failed(error) => return Err(error),
        }

        let mut produced = self.drain_pending(output);
        if self.pending_len != 0 {
            return Ok(output_full(0, produced));
        }
        if self.quantum_len != 0 {
            let Some((bytes, len)) = decode_tail(&self.quantum[..self.quantum_len]) else {
                return Err(self.fail(ForgivingError::InvalidInput));
            };
            self.quantum_len = 0;
            self.pending = bytes;
            self.pending_start = 0;
            self.pending_len = len;
            produced += self.drain_pending(&mut output[produced..]);
            if self.pending_len != 0 {
                return Ok(output_full(0, produced));
            }
        }
        self.phase = Phase::Complete;
        Ok(Step::new(Progress::new(0, produced), Status::Complete))
    }

    /// Resets all ordinary state for an unrelated web string.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Returns the number of UTF-8 source bytes accepted since reset.
    #[must_use]
    pub const fn source_position(&self) -> usize {
        self.source_position
    }

    fn require_active(&self) -> Result<(), ForgivingError> {
        match self.phase {
            Phase::Active => Ok(()),
            Phase::Finishing => Err(ForgivingError::InputAfterFinish),
            Phase::Complete => Err(ForgivingError::InputAfterComplete),
            Phase::Failed(error) => Err(error),
        }
    }

    fn fail(&mut self, error: ForgivingError) -> ForgivingError {
        if let Phase::Failed(existing) = self.phase {
            existing
        } else {
            self.phase = Phase::Failed(error);
            error
        }
    }

    fn drain_pending(&mut self, output: &mut [u8]) -> usize {
        let written = self.pending_len.min(output.len());
        let end = self.pending_start + written;
        output[..written].copy_from_slice(&self.pending[self.pending_start..end]);
        self.pending_start = end;
        self.pending_len -= written;
        if self.pending_len == 0 {
            self.pending_start = 0;
        }
        written
    }
}

fn output_full(consumed: usize, produced: usize) -> Step {
    Step::new(
        Progress::new(consumed, produced),
        Status::OutputFull(OutputFull::new(NonZeroUsize::MIN)),
    )
}

pub(super) const fn is_ascii_whitespace(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | 0x0c | b'\r' | b' ')
}

const fn is_standard_symbol(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/')
}

const fn decode_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_quantum(input: [u8; 4]) -> Option<([u8; 3], usize, bool)> {
    let first = decode_value(input[0])?;
    let second = decode_value(input[1])?;
    let one = (first << 2) | (second >> 4);
    match (input[2], input[3]) {
        (b'=', b'=') => Some(([one, 0, 0], 1, true)),
        (b'=', _) => None,
        (third, b'=') => {
            let third = decode_value(third)?;
            Some(([one, (second << 4) | (third >> 2), 0], 2, true))
        }
        (third, fourth) => {
            let third = decode_value(third)?;
            let fourth = decode_value(fourth)?;
            Some((
                [one, (second << 4) | (third >> 2), (third << 6) | fourth],
                3,
                false,
            ))
        }
    }
}

fn decode_tail(input: &[u8]) -> Option<([u8; 3], usize)> {
    match input {
        [] => Some(([0; 3], 0)),
        [first, second] => {
            let first = decode_value(*first)?;
            let second = decode_value(*second)?;
            Some(([(first << 2) | (second >> 4), 0, 0], 1))
        }
        [first, second, third] => {
            let first = decode_value(*first)?;
            let second = decode_value(*second)?;
            let third = decode_value(*third)?;
            Some((
                [
                    (first << 2) | (second >> 4),
                    (second << 4) | (third >> 2),
                    0,
                ],
                2,
            ))
        }
        _ => None,
    }
}
