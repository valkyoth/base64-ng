use base64_ng::STRICT_STANDARD_PADDED;

use crate::{
    MimeBodyDecodePolicy, MimeBodyDecodeReport, MimeBodyError, MimeBodyErrorKind, MimeBodyLimits,
    MimeBodyProgress, MimeBodyStatus, MimeBodyStep,
};

const INPUT_QUANTUM: usize = 4;
const OUTPUT_QUANTUM: usize = 3;
const CANONICAL_LINE_WIDTH: usize = 76;

/// Incremental bounded RFC 2045 Base64 content-transfer body decoder.
///
/// Output is prefix-committing. Use the crate's one-shot decode helper when a
/// malformed later byte must leave a caller-owned destination unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct MimeBodyDecoder {
    policy: MimeBodyDecodePolicy,
    limits: MimeBodyLimits,
    quantum: [u8; INPUT_QUANTUM],
    quantum_len: usize,
    pending: [u8; OUTPUT_QUANTUM],
    pending_start: usize,
    pending_len: usize,
    input_bytes: usize,
    output_bytes: usize,
    skipped: usize,
    skipped_non_whitespace: usize,
    bare_line_endings: usize,
    physical_line_bytes: usize,
    canonical_line_symbols: usize,
    completed_canonical_line_symbols: Option<usize>,
    work_before_output: usize,
    pending_cr: bool,
    compatible_previous_cr: bool,
    terminal_padding: bool,
    complete: bool,
    failed: bool,
}

impl MimeBodyDecoder {
    /// Constructs a bounded body decoder with an explicit compatibility policy.
    #[must_use]
    pub const fn new(policy: MimeBodyDecodePolicy, limits: MimeBodyLimits) -> Self {
        Self {
            policy,
            limits,
            quantum: [0; INPUT_QUANTUM],
            quantum_len: 0,
            pending: [0; OUTPUT_QUANTUM],
            pending_start: 0,
            pending_len: 0,
            input_bytes: 0,
            output_bytes: 0,
            skipped: 0,
            skipped_non_whitespace: 0,
            bare_line_endings: 0,
            physical_line_bytes: 0,
            canonical_line_symbols: 0,
            completed_canonical_line_symbols: None,
            work_before_output: 0,
            pending_cr: false,
            compatible_previous_cr: false,
            terminal_padding: false,
            complete: false,
            failed: false,
        }
    }

    /// Accepts a source fragment and emits as much decoded output as fits.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for malformed input, finite-limit failure,
    /// terminal-state reuse, arithmetic overflow, or an internal invariant.
    pub fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<MimeBodyStep, MimeBodyError> {
        self.require_active()?;
        let mut consumed = 0;
        let mut produced = self.drain(output);
        while consumed < input.len() && self.pending_len == 0 {
            let index = self.input_bytes;
            self.accept_input_byte(input[consumed], index)?;
            self.input_bytes += 1;
            consumed += 1;
            produced += self.drain(&mut output[produced..]);
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

    /// Validates the final body state and drains any pending decoded bytes.
    ///
    /// # Errors
    ///
    /// Returns [`MimeBodyError`] for incomplete input, malformed canonical
    /// layout, terminal-state reuse, or an internal invariant.
    pub fn finish(&mut self, output: &mut [u8]) -> Result<MimeBodyStep, MimeBodyError> {
        self.require_active()?;
        let produced = self.drain(output);
        if self.pending_len != 0 {
            return Ok(MimeBodyStep::new(
                MimeBodyProgress::new(0, produced),
                MimeBodyStatus::OutputFull,
            ));
        }
        if self.pending_cr || self.quantum_len != 0 {
            return self.fail(MimeBodyError::at(
                if self.pending_cr {
                    MimeBodyErrorKind::InvalidCanonicalLayout
                } else {
                    MimeBodyErrorKind::InvalidBase64
                },
                self.input_bytes,
            ));
        }
        if self.compatible_previous_cr {
            self.compatible_previous_cr = false;
            self.bare_line_endings += 1;
        }
        self.complete = true;
        Ok(MimeBodyStep::new(
            MimeBodyProgress::new(0, produced),
            MimeBodyStatus::Complete,
        ))
    }

    /// Returns cumulative interoperable-decoding metadata.
    #[must_use]
    pub const fn mime_body_decode_report(&self) -> MimeBodyDecodeReport {
        MimeBodyDecodeReport::new(
            self.output_bytes,
            self.skipped,
            self.skipped_non_whitespace,
            self.bare_line_endings,
        )
    }

    fn accept_input_byte(&mut self, byte: u8, index: usize) -> Result<(), MimeBodyError> {
        self.require_input_capacity(index)?;
        self.work_before_output = self
            .work_before_output
            .checked_add(1)
            .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
        if self.work_before_output > self.limits.max_work_before_output() {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::WorkBeforeOutputLimitExceeded,
                index,
            ));
        }

        match self.policy {
            MimeBodyDecodePolicy::Canonical => self.accept_canonical(byte, index),
            MimeBodyDecodePolicy::Rfc2045Compatible => self.accept_compatible(byte, index),
        }
    }

    fn accept_canonical(&mut self, byte: u8, index: usize) -> Result<(), MimeBodyError> {
        if self.pending_cr {
            self.pending_cr = false;
            if byte != b'\n' {
                return self.fail(MimeBodyError::at(
                    MimeBodyErrorKind::InvalidCanonicalLayout,
                    index,
                ));
            }
            self.completed_canonical_line_symbols = Some(self.canonical_line_symbols);
            self.canonical_line_symbols = 0;
            self.physical_line_bytes = 0;
            return Ok(());
        }
        if byte == b'\r' {
            if self.canonical_line_symbols == 0 {
                return self.fail(MimeBodyError::at(
                    MimeBodyErrorKind::InvalidCanonicalLayout,
                    index,
                ));
            }
            self.pending_cr = true;
            return Ok(());
        }
        if byte == b'\n' || !is_table_one(byte) {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::InvalidCanonicalLayout,
                index,
            ));
        }
        if let Some(previous) = self.completed_canonical_line_symbols.take()
            && previous != CANONICAL_LINE_WIDTH
        {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::InvalidCanonicalLayout,
                index,
            ));
        }
        self.canonical_line_symbols += 1;
        self.physical_line_bytes += 1;
        if self.canonical_line_symbols > CANONICAL_LINE_WIDTH
            || self.physical_line_bytes > self.limits.max_physical_line_bytes()
        {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::InvalidCanonicalLayout,
                index,
            ));
        }
        self.accept_symbol(byte, index)
    }

    fn accept_compatible(&mut self, byte: u8, index: usize) -> Result<(), MimeBodyError> {
        if self.compatible_previous_cr && byte != b'\n' {
            self.compatible_previous_cr = false;
            self.bare_line_endings += 1;
        }
        if byte == b'\n' {
            if self.compatible_previous_cr {
                self.compatible_previous_cr = false;
                self.record_skipped(byte, index)?;
            } else {
                self.bare_line_endings += 1;
                self.record_skipped(byte, index)?;
            }
            self.physical_line_bytes = 0;
            return Ok(());
        }
        if byte == b'\r' {
            self.compatible_previous_cr = true;
            self.record_skipped(byte, index)?;
            self.physical_line_bytes = 0;
            return Ok(());
        }
        self.compatible_previous_cr = false;
        self.physical_line_bytes = self
            .physical_line_bytes
            .checked_add(1)
            .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
        if self.physical_line_bytes > self.limits.max_physical_line_bytes() {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::PhysicalLineTooLong,
                index,
            ));
        }
        if is_table_one(byte) {
            self.accept_symbol(byte, index)
        } else {
            self.record_skipped(byte, index)
        }
    }

    fn accept_symbol(&mut self, byte: u8, index: usize) -> Result<(), MimeBodyError> {
        if self.terminal_padding {
            return self.fail(MimeBodyError::at(MimeBodyErrorKind::InvalidBase64, index));
        }
        self.quantum[self.quantum_len] = byte;
        self.quantum_len += 1;
        if self.quantum_len != INPUT_QUANTUM {
            return Ok(());
        }

        let mut decoded = [0u8; OUTPUT_QUANTUM];
        let Ok(written) = STRICT_STANDARD_PADDED.decode_into(&self.quantum, &mut decoded) else {
            return self.fail(MimeBodyError::at(MimeBodyErrorKind::InvalidBase64, index));
        };
        let next_output = self
            .output_bytes
            .checked_add(written)
            .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
        if next_output > self.limits.max_decoded_output_bytes() {
            return self.fail(MimeBodyError::at(
                MimeBodyErrorKind::OutputLimitExceeded,
                index,
            ));
        }
        self.pending[..written].copy_from_slice(&decoded[..written]);
        self.pending_start = 0;
        self.pending_len = written;
        self.output_bytes = next_output;
        self.terminal_padding = self.quantum.contains(&b'=');
        self.quantum_len = 0;
        self.work_before_output = 0;
        Ok(())
    }

    fn record_skipped(&mut self, byte: u8, index: usize) -> Result<(), MimeBodyError> {
        self.skipped = self
            .skipped
            .checked_add(1)
            .ok_or_else(|| MimeBodyError::new(MimeBodyErrorKind::LengthOverflow))?;
        if !matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
            self.skipped_non_whitespace += 1;
        }
        if self.skipped > self.limits.max_skipped_nonalphabet_bytes() {
            self.fail(MimeBodyError::at(
                MimeBodyErrorKind::SkippedNonalphabetLimitExceeded,
                index,
            ))
        } else {
            Ok(())
        }
    }

    fn require_input_capacity(&mut self, index: usize) -> Result<(), MimeBodyError> {
        if self.input_bytes >= self.limits.max_input_bytes() {
            self.fail(MimeBodyError::at(
                MimeBodyErrorKind::InputLimitExceeded,
                index,
            ))
        } else {
            Ok(())
        }
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

pub(crate) const fn is_table_one(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=')
}
