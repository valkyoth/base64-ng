/// Finite resource limits for one RFC 7468 operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_field_names)]
pub struct PemLimits {
    max_input_bytes: usize,
    max_encoded_output_bytes: usize,
    max_decoded_output_bytes: usize,
    max_physical_line_bytes: usize,
    max_label_bytes: usize,
    max_blocks: usize,
    max_adjacent_text_bytes: usize,
    max_work_before_output: usize,
}

impl PemLimits {
    /// Builds a complete explicit limit set.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        max_input_bytes: usize,
        max_encoded_output_bytes: usize,
        max_decoded_output_bytes: usize,
        max_physical_line_bytes: usize,
        max_label_bytes: usize,
        max_blocks: usize,
        max_adjacent_text_bytes: usize,
        max_work_before_output: usize,
    ) -> Self {
        Self {
            max_input_bytes,
            max_encoded_output_bytes,
            max_decoded_output_bytes,
            max_physical_line_bytes,
            max_label_bytes,
            max_blocks,
            max_adjacent_text_bytes,
            max_work_before_output,
        }
    }

    /// Maximum source document bytes.
    #[must_use]
    pub const fn max_input_bytes(self) -> usize {
        self.max_input_bytes
    }

    /// Maximum generated textual bytes.
    #[must_use]
    pub const fn max_encoded_output_bytes(self) -> usize {
        self.max_encoded_output_bytes
    }

    /// Maximum decoded payload bytes across all blocks.
    #[must_use]
    pub const fn max_decoded_output_bytes(self) -> usize {
        self.max_decoded_output_bytes
    }

    /// Maximum bytes in one physical input line, excluding its ending.
    #[must_use]
    pub const fn max_physical_line_bytes(self) -> usize {
        self.max_physical_line_bytes
    }

    /// Maximum label bytes.
    #[must_use]
    pub const fn max_label_bytes(self) -> usize {
        self.max_label_bytes
    }

    /// Maximum textual encoding instances in one document.
    #[must_use]
    pub const fn max_blocks(self) -> usize {
        self.max_blocks
    }

    /// Maximum bytes before, between, and after instances.
    #[must_use]
    pub const fn max_adjacent_text_bytes(self) -> usize {
        self.max_adjacent_text_bytes
    }

    /// Maximum source bytes inspected before decoded output is released.
    #[must_use]
    pub const fn max_work_before_output(self) -> usize {
        self.max_work_before_output
    }
}

impl Default for PemLimits {
    fn default() -> Self {
        Self::new(
            16 * 1024 * 1024,
            24 * 1024 * 1024,
            16 * 1024 * 1024,
            16 * 1024,
            128,
            256,
            1024 * 1024,
            16 * 1024 * 1024,
        )
    }
}
