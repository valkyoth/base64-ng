/// Finite resource limits for one `OpenPGP` armor operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_field_names)]
pub struct OpenPgpLimits {
    max_input_bytes: usize,
    max_encoded_output_bytes: usize,
    max_decoded_output_bytes: usize,
    max_physical_line_bytes: usize,
    max_header_count: usize,
    max_total_header_bytes: usize,
    max_label_bytes: usize,
    max_blocks: usize,
    max_adjacent_document_bytes: usize,
    max_work_before_output: usize,
}

impl OpenPgpLimits {
    /// Builds a complete explicit limit set.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        max_input_bytes: usize,
        max_encoded_output_bytes: usize,
        max_decoded_output_bytes: usize,
        max_physical_line_bytes: usize,
        max_header_count: usize,
        max_total_header_bytes: usize,
        max_label_bytes: usize,
        max_blocks: usize,
        max_adjacent_document_bytes: usize,
        max_work_before_output: usize,
    ) -> Self {
        Self {
            max_input_bytes,
            max_encoded_output_bytes,
            max_decoded_output_bytes,
            max_physical_line_bytes,
            max_header_count,
            max_total_header_bytes,
            max_label_bytes,
            max_blocks,
            max_adjacent_document_bytes,
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
    /// Maximum decoded bytes across all blocks.
    #[must_use]
    pub const fn max_decoded_output_bytes(self) -> usize {
        self.max_decoded_output_bytes
    }
    /// Maximum bytes in one physical line, excluding its ending.
    #[must_use]
    pub const fn max_physical_line_bytes(self) -> usize {
        self.max_physical_line_bytes
    }
    /// Maximum armor headers per block.
    #[must_use]
    pub const fn max_header_count(self) -> usize {
        self.max_header_count
    }
    /// Maximum retained header bytes across the document.
    #[must_use]
    pub const fn max_total_header_bytes(self) -> usize {
        self.max_total_header_bytes
    }
    /// Maximum boundary label bytes.
    #[must_use]
    pub const fn max_label_bytes(self) -> usize {
        self.max_label_bytes
    }
    /// Maximum armor blocks per document.
    #[must_use]
    pub const fn max_blocks(self) -> usize {
        self.max_blocks
    }
    /// Maximum whitespace bytes outside armor blocks.
    #[must_use]
    pub const fn max_adjacent_document_bytes(self) -> usize {
        self.max_adjacent_document_bytes
    }
    /// Maximum source bytes inspected before decoded output release.
    #[must_use]
    pub const fn max_work_before_output(self) -> usize {
        self.max_work_before_output
    }
}

impl Default for OpenPgpLimits {
    fn default() -> Self {
        Self::new(
            16 * 1024 * 1024,
            24 * 1024 * 1024,
            16 * 1024 * 1024,
            16 * 1024,
            64,
            64 * 1024,
            64,
            256,
            1024 * 1024,
            16 * 1024 * 1024,
        )
    }
}
