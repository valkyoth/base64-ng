use crate::{Alphabet, DecodeError, DecodeValidation, Engine, decode_backend, validate_decode};

impl<A: Alphabet, const PAD: bool> Engine<A, PAD> {
    /// Decodes ordinary input with an explicit validation strategy.
    ///
    /// Preserves [`Self::decode_slice`]'s errors, precedence, and destination
    /// behavior, including its historical non-transactional scalar error path.
    /// Use [`crate::Base64::decode_into_with_validation`] for transactionality.
    pub fn decode_slice_with_validation(
        &self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<usize, DecodeError> {
        // Both strategies retain today's scalar checks until separate admission.
        match validation {
            DecodeValidation::Auto | DecodeValidation::ScalarReference => {
                decode_backend::decode_slice::<A, PAD>(input, output)
            }
        }
    }

    /// Like [`Self::decode_slice_clear_tail`] with an explicit ordinary policy.
    /// Errors clear the entire destination; success clears the unused tail.
    pub fn decode_slice_clear_tail_with_validation(
        &self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<usize, DecodeError> {
        match self.decode_slice_with_validation(input, output, validation) {
            Ok(written) => {
                crate::wipe_tail(output, written);
                Ok(written)
            }
            Err(error) => {
                crate::wipe_bytes(output);
                Err(error)
            }
        }
    }

    /// Fully validates input and returns its exact decoded length.
    ///
    /// Unlike historical [`Self::decoded_len`], this checks alphabet membership
    /// and canonical tail bits, not just length and padding shape.
    pub fn validated_decoded_len_with_validation(
        &self,
        input: &[u8],
        validation: DecodeValidation,
    ) -> Result<usize, DecodeError> {
        match validation {
            DecodeValidation::Auto | DecodeValidation::ScalarReference => {
                validate_decode::<A, PAD>(input)
            }
        }
    }

    /// Like [`Self::validate_result`] with an explicit ordinary policy.
    pub fn validate_result_with_validation(
        &self,
        input: &[u8],
        validation: DecodeValidation,
    ) -> Result<(), DecodeError> {
        self.validated_decoded_len_with_validation(input, validation)
            .map(|_| ())
    }

    /// Like [`Self::decode_vec`] with an explicit ordinary policy.
    /// Complete validation precedes allocation. This is not secret storage.
    #[cfg(feature = "alloc")]
    pub fn decode_vec_with_validation(
        &self,
        input: &[u8],
        validation: DecodeValidation,
    ) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = self.validated_decoded_len_with_validation(input, validation)?;
        let mut output = alloc::vec![0; required];
        let written =
            self.decode_slice_clear_tail_with_validation(input, &mut output, validation)?;
        output.truncate(written);
        Ok(output)
    }
}
