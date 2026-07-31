//! Build-only exact-backend performance evidence boundary.
//!
//! This module is intentionally available only with
//! `--cfg base64_ng_perf_evidence` and `std`. It is not a stable consumer API.

use crate::{Alphabet, DecodeError, EncodeError, Standard, UrlSafe};

/// Backend that a performance evidence campaign can request explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceBackend {
    /// Normal production runtime dispatch.
    Auto,
    /// Audited scalar implementation.
    Scalar,
    /// `x86`/`x86_64` SSSE3 and SSE4.1 implementation.
    Ssse3Sse41,
    /// `x86`/`x86_64` AVX2 implementation.
    Avx2,
    /// `x86`/`x86_64` AVX-512 VBMI implementation.
    Avx512Vbmi,
    /// Little-endian `AArch64` NEON implementation.
    Neon,
    /// WebAssembly `simd128` implementation.
    WasmSimd128,
}

impl EvidenceBackend {
    /// Every backend represented by the performance evidence schema.
    pub const ALL: [Self; 7] = [
        Self::Auto,
        Self::Scalar,
        Self::Ssse3Sse41,
        Self::Avx2,
        Self::Avx512Vbmi,
        Self::Neon,
        Self::WasmSimd128,
    ];

    /// Stable machine-readable backend name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Scalar => "scalar",
            Self::Ssse3Sse41 => "ssse3-sse4.1",
            Self::Avx2 => "avx2",
            Self::Avx512Vbmi => "avx512-vbmi",
            Self::Neon => "neon",
            Self::WasmSimd128 => "wasm-simd128",
        }
    }

    /// Whether this exact backend can execute in the current process.
    #[must_use]
    pub fn is_available(self) -> bool {
        match self {
            Self::Auto | Self::Scalar => true,
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Ssse3Sse41 => crate::simd::ssse3_sse41_decode_available(),
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Avx2 => crate::simd::avx2_decode_available(),
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Avx512Vbmi => crate::simd::avx512_decode_available(),
            #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
            Self::Neon => crate::simd::neon_available(),
            #[cfg(all(feature = "simd", target_arch = "wasm32"))]
            Self::WasmSimd128 => crate::simd::wasm_simd128_decode_available(),
            _ => false,
        }
    }
}

/// Encodes Standard Base64 through an exact evidence backend.
///
/// `None` means that the requested backend is unavailable on this build or
/// processor.
pub fn encode_standard<const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, EncodeError>> {
    encode::<Standard, PAD>(backend, input, output)
}

/// Encodes URL-safe Base64 through an exact evidence backend.
///
/// `None` means that the requested backend is unavailable on this build or
/// processor.
pub fn encode_url_safe<const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, EncodeError>> {
    encode::<UrlSafe, PAD>(backend, input, output)
}

/// Decodes strict Standard Base64 through an exact evidence backend.
///
/// `None` means that the requested backend is unavailable on this build or
/// processor.
pub fn decode_standard<const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, DecodeError>> {
    decode::<Standard, PAD>(backend, input, output)
}

/// Decodes strict URL-safe Base64 through an exact evidence backend.
///
/// `None` means that the requested backend is unavailable on this build or
/// processor.
pub fn decode_url_safe<const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, DecodeError>> {
    decode::<UrlSafe, PAD>(backend, input, output)
}

fn encode<A, const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, EncodeError>>
where
    A: Alphabet,
{
    match backend {
        EvidenceBackend::Auto => Some(crate::encode_backend::encode_slice::<A, PAD>(input, output)),
        EvidenceBackend::Scalar => Some(crate::scalar::encode_slice::<A, PAD>(input, output)),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Ssse3Sse41 if backend.is_available() => Some(
            crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output),
        ),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Avx2 if backend.is_available() => {
            Some(crate::simd::encode_slice_avx2::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Avx512Vbmi if backend.is_available() => {
            Some(crate::simd::encode_slice_avx512::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        EvidenceBackend::Neon if backend.is_available() => {
            Some(crate::simd::encode_slice_neon::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EvidenceBackend::WasmSimd128 if backend.is_available() => Some(
            crate::simd::encode_slice_wasm_simd128::<A, PAD>(input, output),
        ),
        _ => None,
    }
}

fn decode<A, const PAD: bool>(
    backend: EvidenceBackend,
    input: &[u8],
    output: &mut [u8],
) -> Option<Result<usize, DecodeError>>
where
    A: Alphabet,
{
    match backend {
        EvidenceBackend::Auto => Some(crate::decode_backend::decode_slice::<A, PAD>(input, output)),
        EvidenceBackend::Scalar => Some(crate::scalar::decode_slice::<A, PAD>(input, output)),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Ssse3Sse41 if backend.is_available() => Some(
            crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output),
        ),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Avx2 if backend.is_available() => {
            Some(crate::simd::decode_slice_avx2::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EvidenceBackend::Avx512Vbmi if backend.is_available() => {
            Some(crate::simd::decode_slice_avx512::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        EvidenceBackend::Neon if backend.is_available() => {
            Some(crate::simd::decode_slice_neon::<A, PAD>(input, output))
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EvidenceBackend::WasmSimd128 if backend.is_available() => Some(
            crate::simd::decode_slice_wasm_simd128::<A, PAD>(input, output),
        ),
        _ => None,
    }
}
