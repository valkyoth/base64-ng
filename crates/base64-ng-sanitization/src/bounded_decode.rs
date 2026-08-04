use crate::{
    SecretVecDecodeError, preflight::enforce_encoded_input_limit,
    stack::enforce_stack_secret_capacity,
};
use base64_ng::{Alphabet, DecodeError, ct::CtEngine};
use sanitization::{SecretVec, SecureSanitize};

/// Explicitly bounded, fallibly allocated dynamic secret decode helpers.
pub trait CtDecodeSanitizationBoundedExt {
    /// Decode into a clear-on-drop secret after enforcing `MAX` decoded bytes.
    ///
    /// The public encoded-input limit is checked before constant-time-oriented
    /// validation. Capacity validation and fallible reservation complete before
    /// plaintext can be written to the output allocation.
    ///
    /// # Errors
    ///
    /// Returns [`SecretVecDecodeError::EncodedInputLimit`] before validation
    /// when input exceeds the public bound, [`SecretVecDecodeError::Decode`]
    /// for malformed input,
    /// [`SecretVecDecodeError::CapacityLimit`] when decoded output exceeds
    /// `MAX`, or [`SecretVecDecodeError::AllocationFailed`] when reservation
    /// fails.
    fn decode_secret_vec_bounded<const MAX: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, SecretVecDecodeError>;

    /// Decode through at most `STAGE` stack bytes after enforcing `MAX`.
    ///
    /// `STAGE` may not exceed
    /// [`crate::MAX_SANITIZATION_STACK_SECRET_BYTES`]. Both public limits and
    /// staging capacity are checked before heap allocation.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::decode_secret_vec_bounded`], or a
    /// staged [`DecodeError::StagingTooSmall`] when `STAGE` cannot hold the
    /// decoded value.
    fn decode_secret_vec_staged_bounded<const MAX: usize, const STAGE: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, SecretVecDecodeError>;
}

impl<A, const PAD: bool> CtDecodeSanitizationBoundedExt for CtEngine<A, PAD>
where
    A: Alphabet,
{
    fn decode_secret_vec_bounded<const MAX: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, SecretVecDecodeError> {
        decode_bounded::<A, PAD, MAX>(self, input)
    }

    fn decode_secret_vec_staged_bounded<const MAX: usize, const STAGE: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretVec, SecretVecDecodeError> {
        const { enforce_stack_secret_capacity::<STAGE>() }
        decode_staged_bounded::<A, PAD, MAX, STAGE>(self, input)
    }
}

pub(crate) fn decode_bounded<A, const PAD: bool, const MAX: usize>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
) -> Result<SecretVec, SecretVecDecodeError>
where
    A: Alphabet,
{
    decode_bounded_with::<A, PAD, MAX, _>(engine, input, allocate_zeroed)
}

fn decode_bounded_with<A, const PAD: bool, const MAX: usize, F>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
    allocate: F,
) -> Result<SecretVec, SecretVecDecodeError>
where
    A: Alphabet,
    F: FnOnce(usize) -> Result<alloc::vec::Vec<u8>, SecretVecDecodeError>,
{
    let required = preflight(engine, input, MAX, MAX)?;
    let mut output = allocate(required)?;
    let written = engine
        .decode_slice_clear_tail(input, &mut output)
        .map_err(SecretVecDecodeError::Decode)?;
    output.truncate(written);
    Ok(SecretVec::from_vec(output))
}

pub(crate) fn decode_staged_bounded<A, const PAD: bool, const MAX: usize, const STAGE: usize>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
) -> Result<SecretVec, SecretVecDecodeError>
where
    A: Alphabet,
{
    const { enforce_stack_secret_capacity::<STAGE>() }
    decode_staged_bounded_with::<A, PAD, MAX, STAGE, _>(engine, input, allocate_zeroed)
}

fn decode_staged_bounded_with<A, const PAD: bool, const MAX: usize, const STAGE: usize, F>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
    allocate: F,
) -> Result<SecretVec, SecretVecDecodeError>
where
    A: Alphabet,
    F: FnOnce(usize) -> Result<alloc::vec::Vec<u8>, SecretVecDecodeError>,
{
    let required = preflight(engine, input, core::cmp::min(MAX, STAGE), MAX)?;
    if required > STAGE {
        return Err(SecretVecDecodeError::Decode(DecodeError::StagingTooSmall {
            required,
            available: STAGE,
        }));
    }

    let mut output = allocate(required)?;
    let mut staging = [0u8; STAGE];
    let written = match engine.decode_slice_staged_clear_tail(input, &mut output, &mut staging) {
        Ok(written) => written,
        Err(error) => {
            output.secure_sanitize();
            staging.secure_sanitize();
            return Err(SecretVecDecodeError::Decode(error));
        }
    };
    staging.secure_sanitize();
    output.truncate(written);
    Ok(SecretVec::from_vec(output))
}

fn preflight<A, const PAD: bool>(
    engine: &CtEngine<A, PAD>,
    input: &[u8],
    encoded_limit_decoded: usize,
    output_limit: usize,
) -> Result<usize, SecretVecDecodeError>
where
    A: Alphabet,
{
    enforce_encoded_input_limit::<PAD>(encoded_limit_decoded, input.len()).map_err(|limit| {
        SecretVecDecodeError::EncodedInputLimit {
            maximum: limit.maximum,
            actual: limit.actual,
        }
    })?;
    let required = engine
        .decoded_len(input)
        .map_err(SecretVecDecodeError::Decode)?;
    if required > output_limit {
        return Err(SecretVecDecodeError::CapacityLimit {
            maximum: output_limit,
            actual: required,
        });
    }
    Ok(required)
}

fn allocate_zeroed(required: usize) -> Result<alloc::vec::Vec<u8>, SecretVecDecodeError> {
    let mut output = alloc::vec::Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| SecretVecDecodeError::AllocationFailed {
            requested: required,
        })?;
    output.resize(required, 0);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{allocate_zeroed, decode_bounded_with, decode_staged_bounded_with};
    use crate::SecretVecDecodeError;
    use base64_ng::{DecodeError, ct};

    #[test]
    fn capacity_overflow_is_a_reported_allocation_failure() {
        assert!(matches!(
            allocate_zeroed(usize::MAX),
            Err(SecretVecDecodeError::AllocationFailed {
                requested: usize::MAX
            })
        ));
    }

    #[test]
    fn capacity_limit_returns_without_calling_allocator() {
        let result = decode_bounded_with::<_, true, 4, _>(&ct::STANDARD, b"aGVsbG8=", |_| {
            panic!("allocator called after capacity rejection")
        });
        assert!(matches!(
            result,
            Err(SecretVecDecodeError::CapacityLimit {
                maximum: 4,
                actual: 5
            })
        ));
    }

    #[test]
    fn encoded_input_limit_returns_without_calling_allocator() {
        let result = decode_bounded_with::<_, true, 4, _>(&ct::STANDARD, b"!!!!!!!!!!!!", |_| {
            panic!("allocator called after encoded-input rejection")
        });
        assert!(matches!(
            result,
            Err(SecretVecDecodeError::EncodedInputLimit {
                maximum: 8,
                actual: 12
            })
        ));
    }

    #[test]
    fn staging_limit_returns_without_calling_allocator() {
        let result =
            decode_staged_bounded_with::<_, true, 64, 4, _>(&ct::STANDARD, b"aGVsbG8=", |_| {
                panic!("allocator called after staging rejection")
            });
        assert!(matches!(
            result,
            Err(SecretVecDecodeError::Decode(DecodeError::StagingTooSmall {
                required: 5,
                available: 4
            }))
        ));
    }
}
