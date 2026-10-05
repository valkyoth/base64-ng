//! Borrowed, codec-bound ordinary Base64 input.

use super::{
    Base64, Codec, CodecSettings, OneShotError,
    ordinary_decode::{self, retained::ValidatedInput},
};
use crate::DecodeValidation;

/// An immutable borrowed Base64 input validated under one exact codec policy.
///
/// Construction validates the whole input without copying or allocating.
/// Repeated `Auto` decoding reuses that grammar proof, but still checks backend
/// health and performs `checked-backend` output comparisons. A changed validator
/// health generation forces reference revalidation. `ScalarReference` requests
/// reference validation at construction and on **every** decode.
///
/// This is ordinary, non-constant-time data, not a secret container or an
/// attestation. Encoded bytes are exposed explicitly; output is not wiped on
/// drop. Health snapshots do not synchronously revoke in-flight operations.
///
/// ```
/// use base64_ng::{Base64Ref, STRICT_STANDARD_PADDED};
/// let view = Base64Ref::parse(STRICT_STANDARD_PADDED, b"aGVsbG8=").unwrap();
/// assert_eq!(view.decoded_len(), 5);
/// let mut output = [0; 5];
/// assert_eq!(view.decode_into(&mut output), Ok(5));
/// assert_eq!(&output, b"hello");
/// ```
/// The input cannot be mutated while the view remains in use:
/// ```compile_fail
/// use base64_ng::{Base64Ref, STRICT_STANDARD_PADDED};
/// let mut input = *b"Zg==";
/// let view = Base64Ref::parse(STRICT_STANDARD_PADDED, &input).unwrap();
/// input[0] = b'!';
/// view.decode_into(&mut [0; 1]).unwrap();
/// ```
/// The view cannot outlive its input:
/// ```compile_fail
/// use base64_ng::{Base64Ref, StrictStandardPadded, STRICT_STANDARD_PADDED};
/// fn escape() -> Base64Ref<'static, StrictStandardPadded> {
///     let input = *b"Zg==";
///     Base64Ref::parse(STRICT_STANDARD_PADDED, &input).unwrap()
/// }
/// ```
/// Fields are private; there is no unchecked constructor:
/// ```compile_fail
/// use base64_ng::{Base64Ref, STRICT_STANDARD_PADDED};
/// let forged = Base64Ref { codec: STRICT_STANDARD_PADDED, validated: () };
/// ```
pub struct Base64Ref<'a, S: Codec> {
    codec: Base64<S>,
    validated: ValidatedInput<'a>,
}

impl<'a, S: Codec> Base64Ref<'a, S> {
    /// Validates and borrows encoded bytes using `Auto` validation.
    pub fn parse(codec: Base64<S>, encoded: &'a [u8]) -> Result<Self, OneShotError> {
        Self::parse_with_validation(codec, encoded, DecodeValidation::Auto)
    }

    /// Validates and retains the requested policy for all later decodes.
    pub fn parse_with_validation(
        codec: Base64<S>,
        encoded: &'a [u8],
        validation: DecodeValidation,
    ) -> Result<Self, OneShotError> {
        let validated = ValidatedInput::new(codec.settings(), encoded, validation)?;
        Ok(Self { codec, validated })
    }

    /// Returns the borrowed encoded bytes. Passing them to another codec
    /// deliberately discards the retained validation and policy.
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.validated.input()
    }

    /// Returns the encoded byte length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Returns whether the encoded input is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Returns the exact decoded length established at construction.
    #[must_use]
    pub const fn decoded_len(&self) -> usize {
        self.validated.len()
    }

    /// Returns the retained codec, without permitting changes to its policy.
    #[must_use]
    pub const fn codec(&self) -> &Base64<S> {
        &self.codec
    }

    /// Returns the owned settings snapshot used to validate the input.
    #[must_use]
    pub fn settings(&self) -> CodecSettings {
        self.validated.settings()
    }

    /// Returns the validation policy applied at construction and decoding.
    #[must_use]
    pub const fn validation(&self) -> DecodeValidation {
        self.validated.validation()
    }

    /// Decodes transactionally into caller-owned storage, without allocation.
    ///
    /// Every returned error leaves the complete destination unchanged. Health
    /// changes can require revalidation and return an error before writing.
    /// A cached proof never disables checked comparison or scalar recovery.
    pub fn decode_into(&self, output: &mut [u8]) -> Result<usize, OneShotError> {
        ordinary_decode::write(self.validated.writer_proof()?, output)
    }

    /// Allocates ordinary decoded bytes after applying the retained policy.
    #[cfg(feature = "alloc")]
    pub fn decode_to_vec(&self) -> Result<alloc::vec::Vec<u8>, OneShotError> {
        self.decode_to_vec_with_limit(usize::MAX)
    }

    /// Decodes with an exact output-byte limit and fallible reservation.
    /// Required revalidation precedes the limit and allocation checks.
    #[cfg(feature = "alloc")]
    pub fn decode_to_vec_with_limit(
        &self,
        limit: usize,
    ) -> Result<alloc::vec::Vec<u8>, OneShotError> {
        self.decode_with_reserver(limit, |output, required| {
            output
                .try_reserve_exact(required)
                .map_err(|_| OneShotError::AllocationFailed {
                    requested: required,
                })
        })
    }

    #[cfg(feature = "alloc")]
    fn decode_with_reserver(
        &self,
        limit: usize,
        reserve: impl FnOnce(&mut alloc::vec::Vec<u8>, usize) -> Result<(), OneShotError>,
    ) -> Result<alloc::vec::Vec<u8>, OneShotError> {
        let proof = self.validated.writer_proof()?;
        let required = proof.len();
        if required > limit {
            return Err(OneShotError::AllocationLimitExceeded { required, limit });
        }
        let mut output = alloc::vec::Vec::new();
        reserve(&mut output, required)?;
        output.resize(required, 0);
        ordinary_decode::write(proof, &mut output)?;
        Ok(output)
    }
}

impl<S: Codec> AsRef<[u8]> for Base64Ref<'_, S> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<S: Codec> core::fmt::Debug for Base64Ref<'_, S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Base64Ref")
            .field("encoded_len", &self.len())
            .field("decoded_len", &self.decoded_len())
            .field("validation", &self.validation())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
