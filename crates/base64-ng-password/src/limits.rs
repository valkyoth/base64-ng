/// Finite limits for one password-record transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_field_names)]
pub struct PasswordRecordLimits {
    max_record_bytes: usize,
    max_field_bytes: usize,
    max_decoded_salt_bytes: usize,
    max_decoded_output_bytes: usize,
    max_generated_bytes: usize,
    max_work_before_output: usize,
}

impl PasswordRecordLimits {
    /// Builds a complete explicit limit set.
    #[must_use]
    pub const fn new(
        max_record_bytes: usize,
        max_field_bytes: usize,
        max_decoded_salt_bytes: usize,
        max_decoded_output_bytes: usize,
        max_generated_bytes: usize,
        max_work_before_output: usize,
    ) -> Self {
        Self {
            max_record_bytes,
            max_field_bytes,
            max_decoded_salt_bytes,
            max_decoded_output_bytes,
            max_generated_bytes,
            max_work_before_output,
        }
    }

    /// Maximum accepted encoded record bytes.
    #[must_use]
    pub const fn max_record_bytes(self) -> usize {
        self.max_record_bytes
    }

    /// Maximum bytes in one encoded salt or checksum field.
    #[must_use]
    pub const fn max_field_bytes(self) -> usize {
        self.max_field_bytes
    }

    /// Maximum decoded Passlib PBKDF2 salt bytes.
    #[must_use]
    pub const fn max_decoded_salt_bytes(self) -> usize {
        self.max_decoded_salt_bytes
    }

    /// Maximum decoded field bytes released to caller-owned output.
    #[must_use]
    pub const fn max_decoded_output_bytes(self) -> usize {
        self.max_decoded_output_bytes
    }

    /// Maximum generated record bytes.
    #[must_use]
    pub const fn max_generated_bytes(self) -> usize {
        self.max_generated_bytes
    }

    /// Maximum bytes inspected before success or output.
    #[must_use]
    pub const fn max_work_before_output(self) -> usize {
        self.max_work_before_output
    }
}

impl Default for PasswordRecordLimits {
    fn default() -> Self {
        Self::new(4096, 2048, 1024, 1024, 4096, 4096)
    }
}
