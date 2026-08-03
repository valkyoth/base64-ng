/// Finite resource limits for one RFC 2045 Base64 content-transfer body.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MimeBodyLimits {
    input_bytes: usize,
    encoded_output_bytes: usize,
    decoded_output_bytes: usize,
    physical_line_bytes: usize,
    skipped_nonalphabet_bytes: usize,
    work_before_output: usize,
}

impl MimeBodyLimits {
    /// Conservative defaults for an ordinary MIME body.
    ///
    /// The defaults admit at most 16 MiB of source input, 24 MiB of encoded
    /// output, 16 MiB of decoded output, 998 physical bytes per transport
    /// line, 1 MiB of ignored bytes, and 4096 source bytes before one decoded
    /// quantum must be produced.
    pub const DEFAULT: Self = Self::new(
        16 * 1024 * 1024,
        24 * 1024 * 1024,
        16 * 1024 * 1024,
        998,
        1024 * 1024,
        4096,
    );

    /// Constructs an explicit finite limit set.
    #[must_use]
    pub const fn new(
        max_input_bytes: usize,
        max_encoded_output_bytes: usize,
        max_decoded_output_bytes: usize,
        max_physical_line_bytes: usize,
        max_skipped_nonalphabet_bytes: usize,
        max_work_before_output: usize,
    ) -> Self {
        Self {
            input_bytes: max_input_bytes,
            encoded_output_bytes: max_encoded_output_bytes,
            decoded_output_bytes: max_decoded_output_bytes,
            physical_line_bytes: max_physical_line_bytes,
            skipped_nonalphabet_bytes: max_skipped_nonalphabet_bytes,
            work_before_output: max_work_before_output,
        }
    }

    /// Returns the maximum accepted source bytes.
    #[must_use]
    pub const fn max_input_bytes(self) -> usize {
        self.input_bytes
    }

    /// Returns the maximum generated encoded-body bytes.
    #[must_use]
    pub const fn max_encoded_output_bytes(self) -> usize {
        self.encoded_output_bytes
    }

    /// Returns the maximum decoded payload bytes.
    #[must_use]
    pub const fn max_decoded_output_bytes(self) -> usize {
        self.decoded_output_bytes
    }

    /// Returns the maximum physical transport-line width.
    #[must_use]
    pub const fn max_physical_line_bytes(self) -> usize {
        self.physical_line_bytes
    }

    /// Returns the maximum bytes outside RFC 2045 Table 1 that may be ignored.
    #[must_use]
    pub const fn max_skipped_nonalphabet_bytes(self) -> usize {
        self.skipped_nonalphabet_bytes
    }

    /// Returns the maximum source work allowed between decoded quanta.
    #[must_use]
    pub const fn max_work_before_output(self) -> usize {
        self.work_before_output
    }
}

impl Default for MimeBodyLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
