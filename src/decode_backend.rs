//! Decode backend dispatch boundary.
//!
//! This module mirrors the encode backend boundary. Decode acceleration remains
//! inactive; future SIMD decode admission must update this boundary together
//! with canonicality, error-shape, fallback, retention, timing, and release
//! evidence.

use crate::{Alphabet, DecodeError, scalar};

/// Decode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// std `x86`/`x86_64` SSSE3/SSE4.1 fixed-block strict decode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Ssse3Sse41,
}

/// Returns the decode backend selected for this build and target.
#[must_use]
pub(crate) fn active_decode_backend() -> DecodeBackend {
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    {
        if crate::simd::ssse3_sse41_decode_available() {
            return DecodeBackend::Ssse3Sse41;
        }
    }

    DecodeBackend::Scalar
}

/// Decodes `input` into `output` through the admitted strict decode backend.
///
/// Only the normal strict `Engine::decode_slice` family enters this boundary.
/// Legacy whitespace handling, wrapped decode, in-place decode, and `ct`
/// secret decode remain separate scalar surfaces unless a future admission
/// package explicitly adds and tests them.
pub(crate) fn decode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    match active_decode_backend() {
        DecodeBackend::Scalar => scalar::decode_slice::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        DecodeBackend::Ssse3Sse41 => crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output),
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeBackend, active_decode_backend};

    #[test]
    fn boundary_uses_only_admitted_backends() {
        let backend = active_decode_backend();
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        if backend == DecodeBackend::Ssse3Sse41 {
            assert!(crate::simd::ssse3_sse41_decode_available());
            return;
        }
        assert_eq!(backend, DecodeBackend::Scalar);
    }
}
