//! Fallible allocating WHATWG forgiving decode operations.

use alloc::vec::Vec;

use super::{ForgivingBase64, ForgivingError};

impl ForgivingBase64 {
    /// Decodes a web string into a newly allocated byte vector.
    pub fn decode_to_vec(self, input: &str) -> Result<Vec<u8>, ForgivingError> {
        self.decode_to_vec_with_limit(input, usize::MAX)
    }

    /// Decodes subject to an exact caller-selected output limit.
    pub fn decode_to_vec_with_limit(
        self,
        input: &str,
        max_output_len: usize,
    ) -> Result<Vec<u8>, ForgivingError> {
        let required = self.decoded_len(input)?;
        if required > max_output_len {
            return Err(ForgivingError::AllocationLimitExceeded {
                required,
                limit: max_output_len,
            });
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(required)
            .map_err(|_| ForgivingError::AllocationFailed {
                requested: required,
            })?;
        output.resize(required, 0);
        self.decode_into(input, &mut output)?;
        Ok(output)
    }
}
