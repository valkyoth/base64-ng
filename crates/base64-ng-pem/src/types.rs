use alloc::{string::String, vec::Vec};

use crate::{PemError, PemErrorKind, PemLabel};

/// One accepted input policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PemParsePolicy {
    /// Require Figure 3 body layout, matching labels, and no boundary blanks.
    Strict,
    /// Accept RFC 7468 parser latitude under limits and report deviations.
    Rfc7468Compatible,
}

/// Generated document line ending.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PemLineEnding {
    /// Internet CRLF line endings.
    CrLf,
    /// Unix LF line endings.
    Lf,
}

impl PemLineEnding {
    pub(crate) const fn bytes(self) -> &'static [u8] {
        match self {
            Self::CrLf => b"\r\n",
            Self::Lf => b"\n",
        }
    }
}

/// Canonical generator options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PemGenerationOptions {
    line_ending: PemLineEnding,
    terminal_line_ending: bool,
}

impl PemGenerationOptions {
    /// Builds generator options.
    #[must_use]
    pub const fn new(line_ending: PemLineEnding, terminal_line_ending: bool) -> Self {
        Self {
            line_ending,
            terminal_line_ending,
        }
    }

    /// Returns the selected line ending.
    #[must_use]
    pub const fn line_ending(self) -> PemLineEnding {
        self.line_ending
    }

    /// Returns whether the END boundary is followed by a line ending.
    #[must_use]
    pub const fn terminal_line_ending(self) -> bool {
        self.terminal_line_ending
    }
}

impl Default for PemGenerationOptions {
    fn default() -> Self {
        Self::new(PemLineEnding::CrLf, true)
    }
}

/// Bounded parser deviation report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PemParseReport {
    /// Bytes outside textual encoding instances.
    pub adjacent_text_bytes: usize,
    /// Body bytes ignored because they were not Base64 symbols or padding.
    pub skipped_body_bytes: usize,
    /// Lines whose layout differed from strict Figure 3.
    pub noncanonical_body_lines: usize,
    /// BEGIN or END boundary lines accepted with surrounding blanks.
    pub noncanonical_boundary_lines: usize,
    /// CR-only or LF-only line endings observed.
    pub non_crlf_line_endings: usize,
    /// END boundaries accepted with a different label.
    pub mismatched_end_labels: usize,
    /// Labels accepted with lowercase ASCII letters.
    pub noncanonical_labels: usize,
}

/// One decoded RFC 7468 textual encoding instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PemBlock {
    label: PemLabel,
    contents: Vec<u8>,
}

impl PemBlock {
    pub(crate) const fn new(label: PemLabel, contents: Vec<u8>) -> Self {
        Self { label, contents }
    }

    /// Returns the boundary label.
    #[must_use]
    pub const fn label(&self) -> &PemLabel {
        &self.label
    }

    /// Returns decoded payload bytes without interpreting ASN.1.
    #[must_use]
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    /// Consumes the block and returns decoded payload bytes.
    #[must_use]
    pub fn into_contents(self) -> Vec<u8> {
        self.contents
    }
}

/// A bounded document containing one or more textual encoding instances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PemDocument {
    blocks: Vec<PemBlock>,
    report: PemParseReport,
}

impl PemDocument {
    pub(crate) const fn new(blocks: Vec<PemBlock>, report: PemParseReport) -> Self {
        Self { blocks, report }
    }

    /// Returns decoded instances in source order.
    #[must_use]
    pub fn blocks(&self) -> &[PemBlock] {
        &self.blocks
    }

    /// Consumes the document and returns decoded instances.
    #[must_use]
    pub fn into_blocks(self) -> Vec<PemBlock> {
        self.blocks
    }

    /// Returns bounded compatibility deviations.
    #[must_use]
    pub const fn report(&self) -> PemParseReport {
        self.report
    }
}

pub(crate) fn checked_string(bytes: Vec<u8>) -> Result<String, PemError> {
    // Generator output is assembled exclusively from ASCII constants, a
    // validated ASCII label, and the RFC 4648 Standard alphabet.
    String::from_utf8(bytes).map_err(|_| PemError::new(PemErrorKind::InternalInvariantViolation))
}
