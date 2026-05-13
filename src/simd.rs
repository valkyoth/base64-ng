#![allow(unsafe_code)]

//! SIMD admission boundary.
//!
//! This module is the only source module allowed to lower the crate-level
//! `unsafe_code` lint. Keep all future architecture-specific intrinsics behind
//! this boundary, with a local safety explanation for every unsafe block.
//!
//! The module intentionally contains no accelerated backend yet. The `simd`
//! feature remains a compile-time reservation until the AVX2/NEON paths have
//! scalar differential tests, fuzz coverage, and benchmark evidence.

/// Backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ActiveBackend {
    /// The audited scalar implementation.
    Scalar,
}

/// SIMD candidate detected for the current target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Candidate {
    /// No supported SIMD candidate was detected.
    Scalar,
    /// `x86`/`x86_64` AVX2 is available as a future candidate.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2,
    /// ARM NEON is available as a future candidate.
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    Neon,
}

/// Returns the backend that is allowed to execute for this build.
#[must_use]
pub(super) fn active_backend() -> ActiveBackend {
    let _candidate = detected_candidate();
    ActiveBackend::Scalar
}

/// Returns the fastest SIMD candidate visible to this build.
///
/// Candidate detection is intentionally separate from activation. Until an
/// accelerated backend has differential tests and benchmark evidence, detected
/// SIMD support must still execute through [`ActiveBackend::Scalar`].
#[must_use]
pub(super) fn detected_candidate() -> Candidate {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if avx2_available() {
            return Candidate::Avx2;
        }
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        if neon_available() {
            return Candidate::Neon;
        }
    }

    Candidate::Scalar
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
fn avx2_available() -> bool {
    std::is_x86_feature_detected!("avx2")
}

#[cfg(all(not(feature = "std"), any(target_arch = "x86", target_arch = "x86_64")))]
fn avx2_available() -> bool {
    cfg!(target_feature = "avx2")
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
fn neon_available() -> bool {
    cfg!(target_arch = "aarch64") || cfg!(target_feature = "neon")
}
