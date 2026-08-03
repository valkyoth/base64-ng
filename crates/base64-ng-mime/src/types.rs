/// Whether a generated RFC 2045 body ends in `CRLF`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MimeBodyTerminalLineEnding {
    /// Omit a terminal line ending after the final encoded character.
    Omit,
    /// Emit `CRLF` after the final non-empty encoded line.
    IncludeCrLf,
}

/// Which RFC 2045 Base64 content-transfer body input contract to enforce.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MimeBodyDecodePolicy {
    /// Require the crate's canonical 76-column `CRLF` layout.
    ///
    /// Interior lines are exactly 76 Base64 characters. The final line may be
    /// shorter, and its terminal `CRLF` is optional.
    Canonical,
    /// Apply RFC 2045 Section 6.8 interoperable decoding.
    ///
    /// Bytes outside Table 1 are ignored subject to explicit finite limits.
    /// Padding and unused trailing bits remain canonical.
    Rfc2045Compatible,
}

/// Cumulative result metadata for RFC 2045 body decoding.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MimeBodyDecodeReport {
    output_bytes: usize,
    skipped_nonalphabet_bytes: usize,
    skipped_non_whitespace_bytes: usize,
    bare_line_endings: usize,
}

impl MimeBodyDecodeReport {
    pub(crate) const fn new(
        output_bytes: usize,
        skipped_nonalphabet_bytes: usize,
        skipped_non_whitespace_bytes: usize,
        bare_line_endings: usize,
    ) -> Self {
        Self {
            output_bytes,
            skipped_nonalphabet_bytes,
            skipped_non_whitespace_bytes,
            bare_line_endings,
        }
    }

    /// Returns decoded payload bytes produced.
    #[must_use]
    pub const fn output_bytes(self) -> usize {
        self.output_bytes
    }

    /// Returns all ignored bytes outside RFC 2045 Table 1.
    #[must_use]
    pub const fn skipped_nonalphabet_bytes(self) -> usize {
        self.skipped_nonalphabet_bytes
    }

    /// Returns ignored bytes that were not ASCII transport whitespace.
    #[must_use]
    pub const fn skipped_non_whitespace_bytes(self) -> usize {
        self.skipped_non_whitespace_bytes
    }

    /// Returns accepted bare `CR` or `LF` line endings.
    #[must_use]
    pub const fn bare_line_endings(self) -> usize {
        self.bare_line_endings
    }

    /// Returns whether interoperable decoding accepted suspicious transport
    /// content that an application may choose to warn about.
    #[must_use]
    pub const fn has_transport_warning(self) -> bool {
        self.skipped_non_whitespace_bytes != 0 || self.bare_line_endings != 0
    }
}

/// Progress committed by one incremental RFC 2045 body call.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct MimeBodyProgress {
    input_consumed: usize,
    output_produced: usize,
}

impl MimeBodyProgress {
    pub(crate) const fn new(input_consumed: usize, output_produced: usize) -> Self {
        Self {
            input_consumed,
            output_produced,
        }
    }

    /// Returns source bytes accepted by this call.
    #[must_use]
    pub const fn input_consumed(self) -> usize {
        self.input_consumed
    }

    /// Returns destination bytes initialized by this call.
    #[must_use]
    pub const fn output_produced(self) -> usize {
        self.output_produced
    }
}

/// Non-failing incremental RFC 2045 body state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MimeBodyStatus {
    /// More source input or an explicit finish call is required.
    NeedInput,
    /// Pending output must be drained before more source can be accepted.
    OutputFull,
    /// The body completed and accepts no more source input.
    Complete,
}

/// One incremental RFC 2045 body result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MimeBodyStep {
    progress: MimeBodyProgress,
    status: MimeBodyStatus,
}

impl MimeBodyStep {
    pub(crate) const fn new(progress: MimeBodyProgress, status: MimeBodyStatus) -> Self {
        Self { progress, status }
    }

    /// Returns exact committed progress.
    #[must_use]
    pub const fn progress(self) -> MimeBodyProgress {
        self.progress
    }

    /// Returns the resulting state class.
    #[must_use]
    pub const fn status(self) -> MimeBodyStatus {
        self.status
    }
}
