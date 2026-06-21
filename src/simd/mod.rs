#![allow(unsafe_code)]

//! SIMD admission boundary.
//!
//! This module is the only source module allowed to lower the crate-level
//! `unsafe_code` lint. Keep all future architecture-specific intrinsics behind
//! this boundary, with a local safety explanation for every unsafe block.
//!
//! The module admits only std `x86`/`x86_64` AVX-512 VBMI, AVX2, SSSE3/SSE4.1,
//! and std `aarch64` NEON encode backends for Standard and URL-safe alphabet
//! families. All decode paths, custom alphabets, `no_std` builds, and every
//! other SIMD candidate still execute through the scalar implementation.
//!
//! The x86 AVX-512 VBMI, AVX2, SSSE3/SSE4.1, and `AArch64` NEON fixed-block
//! encoders are reachable from runtime encode dispatch on std builds after
//! runtime CPU probing or mandatory-target feature checks. The wasm `simd128`
//! fixed-block implementation remains prototype evidence and is not reachable
//! from runtime backend selection.

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
mod neon;
#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
pub(super) use neon::encode_12_bytes_neon;
#[cfg(all(feature = "std", feature = "simd", target_arch = "aarch64"))]
pub(crate) use neon::encode_slice_neon;
#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
pub(crate) use neon::neon_available;
#[cfg(all(feature = "std", target_arch = "aarch64"))]
pub(crate) use neon::neon_supports_alphabet;
#[cfg(any(
    all(test, any(target_arch = "x86", target_arch = "x86_64")),
    all(
        feature = "std",
        feature = "simd",
        any(target_arch = "x86", target_arch = "x86_64")
    )
))]
mod x86;
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) use x86::{encode_12_bytes_ssse3_sse41, encode_24_bytes_avx2, encode_48_bytes_avx512};
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm;

/// Backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActiveBackend {
    /// The audited scalar implementation.
    Scalar,
    /// std `x86`/`x86_64` AVX-512 VBMI encode backend.
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx512Vbmi,
    /// std `x86`/`x86_64` AVX2 encode backend.
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx2,
    /// std `x86`/`x86_64` SSSE3/SSE4.1 encode backend.
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    Ssse3Sse41,
    /// std `aarch64` NEON encode backend.
    #[cfg(all(feature = "std", target_arch = "aarch64"))]
    Neon,
}

/// SIMD candidate detected for the current target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Candidate {
    /// No supported SIMD candidate was detected.
    Scalar,
    /// `x86`/`x86_64` AVX-512 VBMI is available as a future candidate.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx512Vbmi,
    /// `x86`/`x86_64` AVX2 is available as a future candidate.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2,
    /// `x86`/`x86_64` SSSE3/SSE4.1 is available as a future candidate.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Ssse3Sse41,
    /// ARM NEON is available as a future candidate.
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    Neon,
    /// wasm32 `simd128` is available as a future candidate.
    #[cfg(target_arch = "wasm32")]
    WasmSimd128,
}

/// Returns the backend that is allowed to execute for this build.
#[must_use]
pub(crate) fn active_backend() -> ActiveBackend {
    #[cfg(feature = "std")]
    {
        static ACTIVE_BACKEND: std::sync::OnceLock<ActiveBackend> = std::sync::OnceLock::new();
        *ACTIVE_BACKEND.get_or_init(detect_active_backend)
    }

    #[cfg(not(feature = "std"))]
    {
        detect_active_backend()
    }
}

fn detect_active_backend() -> ActiveBackend {
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        if avx512_vbmi_base64_available() {
            return ActiveBackend::Avx512Vbmi;
        }

        if avx2_available() {
            return ActiveBackend::Avx2;
        }

        if ssse3_sse41_available() {
            return ActiveBackend::Ssse3Sse41;
        }
    }

    #[cfg(all(feature = "std", target_arch = "aarch64"))]
    {
        if neon_available() {
            return ActiveBackend::Neon;
        }
    }

    let _ = detected_candidate();
    ActiveBackend::Scalar
}

/// Returns the fastest SIMD candidate visible to this build.
///
/// Candidate detection is intentionally separate from activation. AVX2 and
/// SSSE3/SSE4.1 encode may be active on std `x86`/`x86_64` builds. Other SIMD
/// support still executes through scalar code until its own admission evidence
/// is complete.
#[must_use]
pub(crate) fn detected_candidate() -> Candidate {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if avx512_vbmi_base64_available() {
            return Candidate::Avx512Vbmi;
        }

        if avx2_available() {
            return Candidate::Avx2;
        }

        if ssse3_sse41_available() {
            return Candidate::Ssse3Sse41;
        }
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        if neon_available() {
            return Candidate::Neon;
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        if wasm_simd128_available() {
            return Candidate::WasmSimd128;
        }
    }

    Candidate::Scalar
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
pub(crate) use x86::{
    avx2_supports_alphabet, avx512_supports_alphabet, encode_slice_avx2, encode_slice_avx512,
    encode_slice_ssse3_sse41, ssse3_sse41_supports_alphabet,
};

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
fn avx512_vbmi_base64_available() -> bool {
    std::is_x86_feature_detected!("avx512f")
        && std::is_x86_feature_detected!("avx512bw")
        && std::is_x86_feature_detected!("avx512vl")
        && std::is_x86_feature_detected!("avx512vbmi")
}

#[cfg(all(not(feature = "std"), any(target_arch = "x86", target_arch = "x86_64")))]
fn avx512_vbmi_base64_available() -> bool {
    cfg!(target_feature = "avx512f")
        && cfg!(target_feature = "avx512bw")
        && cfg!(target_feature = "avx512vl")
        && cfg!(target_feature = "avx512vbmi")
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
fn avx2_available() -> bool {
    std::is_x86_feature_detected!("avx2")
}

#[cfg(all(not(feature = "std"), any(target_arch = "x86", target_arch = "x86_64")))]
fn avx2_available() -> bool {
    cfg!(target_feature = "avx2")
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
fn ssse3_sse41_available() -> bool {
    std::is_x86_feature_detected!("ssse3") && std::is_x86_feature_detected!("sse4.1")
}

#[cfg(all(not(feature = "std"), any(target_arch = "x86", target_arch = "x86_64")))]
fn ssse3_sse41_available() -> bool {
    cfg!(target_feature = "ssse3") && cfg!(target_feature = "sse4.1")
}

#[cfg(target_arch = "wasm32")]
fn wasm_simd128_available() -> bool {
    cfg!(target_feature = "simd128")
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
