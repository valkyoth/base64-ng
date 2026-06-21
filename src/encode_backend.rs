//! Encode backend dispatch boundary.
//!
//! This module is the single integration point between public encode APIs and
//! the implementation that performs encoding. SSSE3/SSE4.1 encode dispatch is
//! admitted only for std `x86`/`x86_64` builds and Standard/URL-safe alphabet
//! families; unsupported alphabets, targets, and in-place encode still fall
//! back to scalar.

use crate::{Alphabet, EncodeError, scalar, scalar_encode_in_place};

/// Encode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EncodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// std `x86`/`x86_64` SSSE3/SSE4.1 fixed-block encode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Ssse3Sse41,
}

/// Returns the encode backend selected for this build and target.
#[must_use]
pub(crate) fn active_encode_backend() -> EncodeBackend {
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Ssse3Sse41 => return EncodeBackend::Ssse3Sse41,
    }

    EncodeBackend::Scalar
}

/// Encodes `input` into `output` through the admitted encode backend.
pub(crate) fn encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    match active_encode_backend() {
        EncodeBackend::Scalar => scalar::encode_slice::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Ssse3Sse41 => {
            if crate::simd::ssse3_sse41_supports_alphabet::<A>() {
                crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output)
            } else {
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
    }
}

/// Encodes `buffer[..input_len]` in place through the admitted encode backend.
pub(crate) fn encode_in_place<A, const PAD: bool>(
    buffer: &mut [u8],
    input_len: usize,
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    match active_encode_backend() {
        EncodeBackend::Scalar => {
            scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        EncodeBackend::Ssse3Sse41 => {
            scalar_encode_in_place::encode_in_place::<A, PAD>(buffer, input_len)
        }
    }
}
