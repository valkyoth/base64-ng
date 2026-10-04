use crate::decode_preflight::{Failure, Preflight};
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
        if validation == DecodeValidation::Auto
            && crate::v2::ordinary_decode::accelerated(input.len())
            && let Some(settings) = strict_settings::<A, PAD>()
            && let Ok(proof) = crate::v2::ordinary_decode::prepare(settings, input, validation)
            && output.len() >= proof.len()
        {
            return crate::v2::ordinary_decode::write(proof, output)
                .map_err(|_| DecodeError::InvalidInput);
        }
        // Recover historical precedence and partial-write behavior on invalid
        // input or insufficient capacity; canonical errors are never translated.
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
        if validation == DecodeValidation::Auto
            && let Some(settings) = strict_settings::<A, PAD>()
            && let Ok(proof) = crate::v2::ordinary_decode::prepare(settings, input, validation)
        {
            return Ok(proof.len());
        }
        match validation {
            DecodeValidation::Auto | DecodeValidation::ScalarReference => {
                Preflight::validate(input, self, |_, input| validate_decode::<A, PAD>(input))
                    .map(|proof| proof.len())
                    .map_err(|error| match error {
                        Failure::Input(error) => error,
                        Failure::Bounds | Failure::ClassifierDisagreement => {
                            DecodeError::InvalidInput
                        }
                    })
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
        // Keep the proof bound to this source through allocation. Rejected
        // input still uses the historical validator for exact diagnostics.
        // Single-quantum calls retain the cheaper historical scalar path.
        let required = if validation == DecodeValidation::Auto
            && input.len() > 4
            && let Some(settings) = strict_settings::<A, PAD>()
        {
            if let Ok(proof) = crate::v2::ordinary_decode::prepare(settings, input, validation) {
                let mut output = alloc::vec![0; proof.len()];
                if crate::v2::ordinary_decode::write(proof, &mut output).is_err() {
                    crate::wipe_bytes(&mut output);
                    return Err(DecodeError::InvalidInput);
                }
                return Ok(output);
            }
            validate_decode::<A, PAD>(input)?
        } else {
            self.validated_decoded_len_with_validation(input, validation)?
        };
        let mut output = alloc::vec![0; required];
        let written =
            self.decode_slice_clear_tail_with_validation(input, &mut output, validation)?;
        output.truncate(written);
        Ok(output)
    }
}

fn strict_settings<A: Alphabet, const PAD: bool>() -> Option<crate::CodecSettings> {
    match (A::ENCODE, PAD) {
        (table, true) if table == crate::Standard::ENCODE => {
            Some(crate::STRICT_STANDARD_PADDED.settings())
        }
        (table, false) if table == crate::Standard::ENCODE => {
            Some(crate::STRICT_STANDARD_UNPADDED.settings())
        }
        (table, true) if table == crate::UrlSafe::ENCODE => {
            Some(crate::STRICT_URL_SAFE_PADDED.settings())
        }
        (table, false) if table == crate::UrlSafe::ENCODE => {
            Some(crate::STRICT_URL_SAFE_UNPADDED.settings())
        }
        _ => None,
    }
}
