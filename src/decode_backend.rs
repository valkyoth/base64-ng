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
}

/// Returns the decode backend selected for this build and target.
#[must_use]
pub(crate) fn active_decode_backend() -> DecodeBackend {
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx512Vbmi => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx2 => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Ssse3Sse41 => {}
        #[cfg(all(feature = "std", target_arch = "aarch64"))]
        crate::simd::ActiveBackend::Neon => {}
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
    }
}
