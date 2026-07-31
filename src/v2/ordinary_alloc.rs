//! Fallible allocating ordinary one-shot operations.

use alloc::{string::String, vec::Vec};

use super::{
    ordinary::OneShotError,
    specifications::{Base64, Codec},
};

impl<S: Codec> Base64<S> {
    /// Encodes into a newly allocated string.
    ///
    /// Allocation uses `try_reserve_exact`; allocation failure is returned as
    /// [`OneShotError::AllocationFailed`]. Process-aborting allocators remain
    /// outside Rust's returned-error contract.
    pub fn encode_to_string(&self, input: &[u8]) -> Result<String, OneShotError> {
        self.encode_to_string_with_limit(input, usize::MAX)
    }

    /// Encodes into a string subject to an exact output-byte limit.
    pub fn encode_to_string_with_limit(
        &self,
        input: &[u8],
        max_output_len: usize,
    ) -> Result<String, OneShotError> {
        self.encode_to_string_with_reserver(input, max_output_len, |output, required| {
            output
                .try_reserve_exact(required)
                .map_err(|_| OneShotError::AllocationFailed {
                    requested: required,
                })
        })
    }

    fn encode_to_string_with_reserver<F>(
        &self,
        input: &[u8],
        max_output_len: usize,
        reserve: F,
    ) -> Result<String, OneShotError>
    where
        F: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
    {
        let required = self.encoded_len(input.len())?;
        require_allocation_limit(required, max_output_len)?;
        let mut output = Vec::new();
        reserve(&mut output, required)?;
        output.resize(required, 0);
        self.encode_into(input, &mut output)?;
        String::from_utf8(output)
            .map_err(|_| OneShotError::Backend(super::contracts::BackendFault::ImpossibleState))
    }

    /// Decodes into a newly allocated byte vector.
    ///
    /// Complete validation and exact sizing happen before allocation. No
    /// plaintext is materialized before the full allocation is reserved.
    pub fn decode_to_vec(&self, input: &[u8]) -> Result<Vec<u8>, OneShotError> {
        self.decode_to_vec_with_limit(input, usize::MAX)
    }

    /// Decodes into a byte vector subject to an exact output-byte limit.
    pub fn decode_to_vec_with_limit(
        &self,
        input: &[u8],
        max_output_len: usize,
    ) -> Result<Vec<u8>, OneShotError> {
        self.decode_to_vec_with_reserver(input, max_output_len, |output, required| {
            output
                .try_reserve_exact(required)
                .map_err(|_| OneShotError::AllocationFailed {
                    requested: required,
                })
        })
    }

    fn decode_to_vec_with_reserver<F>(
        &self,
        input: &[u8],
        max_output_len: usize,
        reserve: F,
    ) -> Result<Vec<u8>, OneShotError>
    where
        F: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
    {
        let required = self.decoded_len(input)?;
        require_allocation_limit(required, max_output_len)?;
        let mut output = Vec::new();
        reserve(&mut output, required)?;
        output.resize(required, 0);
        self.decode_into(input, &mut output)?;
        Ok(output)
    }

    #[cfg(test)]
    pub(super) fn decode_to_vec_with_injected_reserver<F>(
        &self,
        input: &[u8],
        max_output_len: usize,
        reserve: F,
    ) -> Result<Vec<u8>, OneShotError>
    where
        F: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
    {
        self.decode_to_vec_with_reserver(input, max_output_len, reserve)
    }

    #[cfg(test)]
    pub(super) fn encode_to_string_with_injected_reserver<F>(
        &self,
        input: &[u8],
        max_output_len: usize,
        reserve: F,
    ) -> Result<String, OneShotError>
    where
        F: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
    {
        self.encode_to_string_with_reserver(input, max_output_len, reserve)
    }
}

fn require_allocation_limit(required: usize, limit: usize) -> Result<(), OneShotError> {
    if required > limit {
        Err(OneShotError::AllocationLimitExceeded { required, limit })
    } else {
        Ok(())
    }
}
