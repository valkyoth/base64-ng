//! Encode backend dispatch boundary.
//!
//! This module is the single integration point between public encode APIs and
//! the implementation that performs encoding. The current release still forces
//! scalar execution; future SIMD admission must update this boundary together
//! with admission evidence, fallback tests, runtime reports, and documentation.

use crate::{Alphabet, EncodeError, scalar, scalar_encode_in_place};

/// Encode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EncodeBackend {
    /// The audited scalar implementation.
    Scalar,
}

/// Returns the encode backend selected for this build and target.
#[must_use]
pub(crate) fn active_encode_backend() -> EncodeBackend {
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
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
    }
}
