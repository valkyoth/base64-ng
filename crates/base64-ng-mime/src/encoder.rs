use base64_ng::STRICT_STANDARD_PADDED;

use crate::{
    MimeBodyError, MimeBodyErrorKind, MimeBodyLimits, MimeBodyProgress, MimeBodyStatus,
    MimeBodyStep, MimeBodyTerminalLineEnding,
};

const INPUT_QUANTUM: usize = 3;
const ENCODED_QUANTUM: usize = 4;
const MIME_LINE_WIDTH: usize = 76;
const PENDING_CAPACITY: usize = 6;

/// Heapless incremental RFC 2045 Base64 content-transfer body encoder.
///
/// The state emits canonical Standard Base64 in 76-column lines separated by
/// `CRLF`. Caller-visible output is prefix-committing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MimeBodyEncoder {
    limits: MimeBodyLimits,
    terminal: MimeBodyTerminalLineEnding,
    tail: [u8; INPUT_QUANTUM],
    tail_len: usize,
    pending: [u8; PENDING_CAPACITY],
    pending_start: usize,
    pending_len: usize,
    column: usize,
    input_bytes: usize,
    output_bytes: usize,
    finishing: bool,
    complete: bool,
    failed: bool,
}

impl MimeBodyEncoder {
    /// Constructs a bounded canonical body encoder.
    #[must_use]
    pub const fn new(limits: MimeBodyLimits, terminal: MimeBodyTerminalLineEnding) -> Self {
        Self {
            limits,
            terminal,
            tail: [0; INPUT_QUANTUM],
            tail_len: 0,
            pending: [0; PENDING_CAPACITY],
            pending_start: 0,
            pending_len: 0,
            column: 0,
            input_bytes: 0,
            output_bytes: 0,
            finishing: false,
            complete: false,
            failed: false,
        }
    }

    /// Accepts source bytes and emits as much canonical body output as fits.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for finite-limit failure, terminal-state
    /// reuse, arithmetic overflow, or an internal invariant failure.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<MimeBodyStep, MimeBodyError> {
        self.require_active()?;
        if self.finishing {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::TerminalState));
        }

        let mut consumed = 0;
        let mut produced = self.drain(output);
        while consumed < input.len() && self.pending_len == 0 {
            self.require_input_capacity(1)?;
            self.tail[self.tail_len] = input[consumed];
            self.tail_len += 1;
            self.input_bytes += 1;
            consumed += 1;
            if self.tail_len == INPUT_QUANTUM {
                let tail = self.tail;
                self.queue_quantum(tail)?;
                self.tail_len = 0;
                produced += self.drain(&mut output[produced..]);
            }
        }

        let status = if self.pending_len == 0 && consumed == input.len() {
            MimeBodyStatus::NeedInput
        } else {
            MimeBodyStatus::OutputFull
        };
        Ok(MimeBodyStep::new(
            MimeBodyProgress::new(consumed, produced),
            status,
        ))
    }

    /// Emits the final padded quantum and selected terminal line ending.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for finite-limit failure, terminal-state
    /// reuse, or an internal invariant failure.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<MimeBodyStep, MimeBodyError> {
        self.require_active()?;
        self.finishing = true;
        let mut produced = self.drain(output);
        if self.pending_len == 0 && self.tail_len != 0 {
            let mut quantum = [0u8; ENCODED_QUANTUM];
            let written = match STRICT_STANDARD_PADDED
                .encode_into(&self.tail[..self.tail_len], &mut quantum)
            {
                Ok(written) => written,
                Err(error) => return self.fail(crate::error::map_base64(error)),
            };
            self.queue_encoded(&quantum[..written])?;
            self.tail_len = 0;
            produced += self.drain(&mut output[produced..]);
        }
        if self.pending_len == 0
            && matches!(self.terminal, MimeBodyTerminalLineEnding::IncludeCrLf)
            && self.column != 0
        {
            self.queue_raw(b"\r\n")?;
            self.column = 0;
            produced += self.drain(&mut output[produced..]);
        }

        let status = if self.pending_len == 0 {
            self.complete = true;
            MimeBodyStatus::Complete
        } else {
            MimeBodyStatus::OutputFull
        };
        Ok(MimeBodyStep::new(
            MimeBodyProgress::new(0, produced),
            status,
        ))
    }

    /// Returns cumulative source bytes accepted.
    #[must_use]
    pub const fn mime_body_input_bytes(&self) -> usize {
        self.input_bytes
    }

    /// Returns cumulative encoded body bytes emitted or retained for output.
    #[must_use]
    pub const fn mime_body_output_bytes(&self) -> usize {
        self.output_bytes
    }

    fn queue_quantum(&mut self, input: [u8; INPUT_QUANTUM]) -> Result<(), MimeBodyError> {
        let mut encoded = [0u8; ENCODED_QUANTUM];
        if let Err(error) = STRICT_STANDARD_PADDED.encode_into(&input, &mut encoded) {
            return self.fail(crate::error::map_base64(error));
        }
        self.queue_encoded(&encoded)
    }

    fn queue_encoded(&mut self, encoded: &[u8]) -> Result<(), MimeBodyError> {
        if encoded.len() > self.limits.max_physical_line_bytes() {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::PhysicalLineTooLong));
        }
        let next_column = if self.column == MIME_LINE_WIDTH {
            encoded.len()
        } else {
            let Some(next_column) = self.column.checked_add(encoded.len()) else {
                return self.fail(MimeBodyError::new(MimeBodyErrorKind::LengthOverflow));
            };
            if next_column > self.limits.max_physical_line_bytes() {
                return self.fail(MimeBodyError::new(MimeBodyErrorKind::PhysicalLineTooLong));
            }
            next_column
        };
        if self.column == MIME_LINE_WIDTH {
            let mut combined = [0u8; PENDING_CAPACITY];
            combined[..2].copy_from_slice(b"\r\n");
            combined[2..2 + encoded.len()].copy_from_slice(encoded);
            self.queue_raw(&combined[..2 + encoded.len()])?;
        } else {
            self.queue_raw(encoded)?;
        }
        self.column = next_column;
        Ok(())
    }

    fn queue_raw(&mut self, bytes: &[u8]) -> Result<(), MimeBodyError> {
        let Some(next) = self.output_bytes.checked_add(bytes.len()) else {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::LengthOverflow));
        };
        if next > self.limits.max_encoded_output_bytes() {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::OutputLimitExceeded));
        }
        let Some(pending_end) = self.pending_len.checked_add(bytes.len()) else {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::LengthOverflow));
        };
        if pending_end > self.pending.len() {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::InternalInvariant));
        }
        self.pending[..bytes.len()].copy_from_slice(bytes);
        self.pending_start = 0;
        self.pending_len = bytes.len();
        self.output_bytes = next;
        Ok(())
    }

    fn drain(&mut self, output: &mut [u8]) -> usize {
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

    fn require_input_capacity(&mut self, additional: usize) -> Result<(), MimeBodyError> {
        let Some(next) = self.input_bytes.checked_add(additional) else {
            return self.fail(MimeBodyError::new(MimeBodyErrorKind::LengthOverflow));
        };
        if next > self.limits.max_input_bytes() {
            self.fail(MimeBodyError::new(MimeBodyErrorKind::InputLimitExceeded))
        } else {
            Ok(())
        }
    }

    fn require_active(&self) -> Result<(), MimeBodyError> {
        if self.failed || self.complete {
            Err(MimeBodyError::new(MimeBodyErrorKind::TerminalState))
        } else {
            Ok(())
        }
    }

    fn fail<T>(&mut self, error: MimeBodyError) -> Result<T, MimeBodyError> {
        self.failed = true;
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNBOUNDED: MimeBodyLimits = MimeBodyLimits::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    );

    #[test]
    fn output_length_overflow_latches_partially_filled_quantum() {
        let mut encoder = MimeBodyEncoder::new(UNBOUNDED, MimeBodyTerminalLineEnding::Omit);
        encoder.tail[..2].copy_from_slice(b"ab");
        encoder.tail_len = 2;
        encoder.output_bytes = usize::MAX - 1;

        let error = encoder.update(b"c", &mut []).unwrap_err();

        assert_eq!(error.kind(), MimeBodyErrorKind::LengthOverflow);
        assert_eq!(encoder.tail_len, INPUT_QUANTUM);
        assert!(encoder.failed);
        assert_eq!(
            encoder.update(b"d", &mut []).unwrap_err().kind(),
            MimeBodyErrorKind::TerminalState
        );
    }

    #[test]
    fn input_length_overflow_latches_without_mutating_tail() {
        let mut encoder = MimeBodyEncoder::new(UNBOUNDED, MimeBodyTerminalLineEnding::Omit);
        encoder.input_bytes = usize::MAX;

        let error = encoder.update(b"a", &mut []).unwrap_err();

        assert_eq!(error.kind(), MimeBodyErrorKind::LengthOverflow);
        assert_eq!(encoder.tail_len, 0);
        assert!(encoder.failed);
    }
}
