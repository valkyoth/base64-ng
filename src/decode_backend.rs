//! Decode backend dispatch boundary.
//!
//! This module mirrors the encode backend boundary. Decode acceleration is
//! admitted only for the backends and surfaces named here; future SIMD decode
//! admission must update this boundary together with canonicality, error-shape,
//! fallback, retention, timing, and release evidence.

use crate::{Alphabet, DecodeError, scalar};

/// Decode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// std `x86`/`x86_64` AVX-512 VBMI fixed-block strict decode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Avx512Vbmi,
    /// std `x86`/`x86_64` AVX2 fixed-block strict decode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Avx2,
    /// std `x86`/`x86_64` SSSE3/SSE4.1 fixed-block strict decode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        any(target_arch = "x86", target_arch = "x86_64")
    ))]
    Ssse3Sse41,
    /// little-endian std `aarch64` NEON fixed-block strict decode.
    #[cfg(all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ))]
    Neon,
    /// wasm32 `simd128` fixed-block strict decode.
    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    WasmSimd128,
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
        if crate::simd::avx512_decode_available() {
            return DecodeBackend::Avx512Vbmi;
        }

        if crate::simd::avx2_decode_available() {
            return DecodeBackend::Avx2;
        }

        if crate::simd::ssse3_sse41_decode_available() {
            return DecodeBackend::Ssse3Sse41;
        }
    }

    #[cfg(all(
        feature = "simd",
        feature = "std",
        target_arch = "aarch64",
        target_endian = "little"
    ))]
    {
        if crate::simd::neon_available() {
            return DecodeBackend::Neon;
        }
    }

    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    {
        if crate::simd::wasm_simd128_decode_available() {
            return DecodeBackend::WasmSimd128;
        }
    }

    DecodeBackend::Scalar
}

/// Decodes `input` into `output` through the admitted strict decode backend.
///
/// Normal strict `Engine::decode_slice` enters this boundary directly.
/// `Engine::decode_slice_wrapped` also enters after strict line-profile
/// validation and scalar line-ending compaction. `Engine::decode_slice_legacy`
/// enters after strict legacy validation and scalar whitespace compaction.
/// In-place decode and `ct` secret decode remain separate scalar surfaces
/// unless a future admission package explicitly adds and tests them.
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
        DecodeBackend::Avx512Vbmi => crate::simd::decode_slice_avx512::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        DecodeBackend::Avx2 => crate::simd::decode_slice_avx2::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        DecodeBackend::Ssse3Sse41 => crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output),
        #[cfg(all(
            feature = "simd",
            feature = "std",
            target_arch = "aarch64",
            target_endian = "little"
        ))]
        DecodeBackend::Neon => crate::simd::decode_slice_neon::<A, PAD>(input, output),
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        DecodeBackend::WasmSimd128 => {
            crate::simd::decode_slice_wasm_simd128::<A, PAD>(input, output)
        }
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
        if backend == DecodeBackend::Avx512Vbmi {
            assert!(crate::simd::avx512_decode_available());
            return;
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        if backend == DecodeBackend::Avx2 {
            assert!(crate::simd::avx2_decode_available());
            return;
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        if backend == DecodeBackend::Ssse3Sse41 {
            assert!(crate::simd::ssse3_sse41_decode_available());
            return;
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            target_arch = "aarch64",
            target_endian = "little"
        ))]
        if backend == DecodeBackend::Neon {
            assert!(crate::simd::neon_available());
            return;
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        if backend == DecodeBackend::WasmSimd128 {
            assert!(crate::simd::wasm_simd128_decode_available());
            return;
        }
        assert_eq!(backend, DecodeBackend::Scalar);
    }
}
