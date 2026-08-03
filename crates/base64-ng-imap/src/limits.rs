/// Finite limits for one modified-Base64 payload transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImapPayloadLimits {
    input_bytes: usize,
    output_bytes: usize,
    work_before_output: usize,
}

impl ImapPayloadLimits {
    /// Constructs explicit source, destination, and work limits.
    #[must_use]
    pub const fn new(
        max_input_bytes: usize,
        max_output_bytes: usize,
        max_work_before_output: usize,
    ) -> Self {
        Self {
            input_bytes: max_input_bytes,
            output_bytes: max_output_bytes,
            work_before_output: max_work_before_output,
        }
    }

    /// Returns the maximum accepted source bytes.
    #[must_use]
    pub const fn max_input_bytes(self) -> usize {
        self.input_bytes
    }

    /// Returns the maximum produced destination bytes.
    #[must_use]
    pub const fn max_output_bytes(self) -> usize {
        self.output_bytes
    }

    /// Returns the maximum source work accepted before completion.
    #[must_use]
    pub const fn max_work_before_output(self) -> usize {
        self.work_before_output
    }
}
