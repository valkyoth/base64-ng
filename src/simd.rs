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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use super::{Alphabet, encode_base64_value};
#[cfg(target_arch = "x86")]
use core::arch::x86::{__m256i, _mm256_setzero_si256, _mm256_storeu_si256};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__m256i, _mm256_setzero_si256, _mm256_storeu_si256};

/// Backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActiveBackend {
    /// The audited scalar implementation.
    Scalar,
}

/// SIMD candidate detected for the current target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Candidate {
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
pub(crate) fn active_backend() -> ActiveBackend {
    let _candidate = detected_candidate();
    ActiveBackend::Scalar
}

/// Returns the fastest SIMD candidate visible to this build.
///
/// Candidate detection is intentionally separate from activation. Until an
/// accelerated backend has differential tests and benchmark evidence, detected
/// SIMD support must still execute through [`ActiveBackend::Scalar`].
#[must_use]
pub(crate) fn detected_candidate() -> Candidate {
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

/// Encodes one 24-byte block into 32 bytes through the inactive AVX2 prototype.
///
/// This is not an admitted fast path. It exists to exercise target-feature
/// gating, unsafe isolation, and scalar equivalence tests before a real vector
/// encoder is allowed to participate in dispatch.
///
/// # Safety
///
/// The caller must execute this function only when AVX2 is available on the
/// current CPU. The input and output sizes are fixed by their array types.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm256_storeu_si256 accepts unaligned pointers"
)]
#[target_feature(enable = "avx2")]
pub(super) unsafe fn encode_24_bytes_avx2<A>(input: &[u8; 24], output: &mut [u8; 32])
where
    A: Alphabet,
{
    let zeros = _mm256_setzero_si256();
    // SAFETY: `output` is a valid 32-byte mutable array. AVX2 is guaranteed by
    // this function's target feature precondition, and the unaligned store does
    // not require any stronger pointer alignment.
    unsafe {
        _mm256_storeu_si256(output.as_mut_ptr().cast::<__m256i>(), zeros);
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];

        output[write] = encode_base64_value::<A>(b0 >> 2);
        output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
        output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
        output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

        read += 3;
        write += 4;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, Standard, UrlSafe};

    fn fill_pattern(output: &mut [u8], seed: usize) {
        for (index, byte) in output.iter_mut().enumerate() {
            let value = (index * 73 + seed * 19) % 256;
            *byte = u8::try_from(value).unwrap();
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn avx2_encode_prototype_matches_scalar_when_available() {
        if detected_candidate() != Candidate::Avx2 {
            return;
        }

        let mut input = [0; 24];
        for seed in 0..64 {
            fill_pattern(&mut input, seed);

            let mut avx2_standard = [0x55; 32];
            let mut scalar_standard = [0xaa; 32];
            // SAFETY: The candidate check above uses runtime AVX2 detection on
            // std builds and compile-time target-feature detection otherwise.
            unsafe {
                encode_24_bytes_avx2::<Standard>(&input, &mut avx2_standard);
            }
            let scalar_len = Engine::<Standard, true>::new()
                .encode_slice(&input, &mut scalar_standard)
                .unwrap();
            assert_eq!(scalar_len, avx2_standard.len());
            assert_eq!(avx2_standard, scalar_standard);

            let mut avx2_url_safe = [0x55; 32];
            let mut scalar_url_safe = [0xaa; 32];
            // SAFETY: The candidate check above proves AVX2 availability for
            // this test invocation.
            unsafe {
                encode_24_bytes_avx2::<UrlSafe>(&input, &mut avx2_url_safe);
            }
            let scalar_len = Engine::<UrlSafe, true>::new()
                .encode_slice(&input, &mut scalar_url_safe)
                .unwrap();
            assert_eq!(scalar_len, avx2_url_safe.len());
            assert_eq!(avx2_url_safe, scalar_url_safe);
        }
    }
}
