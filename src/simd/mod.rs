#![allow(unsafe_code)]

//! SIMD admission boundary.
//!
//! This module is the only source module allowed to lower the crate-level
//! `unsafe_code` lint. Keep all future architecture-specific intrinsics behind
//! this boundary, with a local safety explanation for every unsafe block.
//!
//! The module intentionally contains no accelerated backend yet. The `simd`
//! feature remains a compile-time reservation until AVX-512, AVX2,
//! SSSE3/SSE4.1, NEON, and wasm `simd128` paths have scalar differential
//! tests, fuzz coverage, and benchmark evidence.
//!
//! The fixed-block prototypes below are test-only and non-dispatchable. The
//! SSSE3/SSE4.1 prototype contains real fixed-block encode logic for Standard
//! and URL-safe alphabets. The AVX-512, AVX2, and NEON prototypes remain
//! scalar-equivalence scaffolding that zero the destination block before a
//! scalar loop overwrites the output. None of these prototypes are compiled
//! into release library builds.

#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
use super::{Alphabet, encode_base64_value};
#[cfg(all(test, target_arch = "aarch64"))]
use core::arch::aarch64::{uint8x16_t, vdupq_n_u8, vst1q_u8};
#[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
use core::arch::arm::{uint8x16_t, vdupq_n_u8, vst1q_u8};

#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
mod x86;
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) use x86::{encode_12_bytes_ssse3_sse41, encode_24_bytes_avx2, encode_48_bytes_avx512};

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

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
fn neon_available() -> bool {
    cfg!(target_arch = "aarch64") || cfg!(target_feature = "neon")
}

#[cfg(target_arch = "wasm32")]
fn wasm_simd128_available() -> bool {
    cfg!(target_feature = "simd128")
}

/// Encodes one 12-byte block into 16 bytes through the inactive NEON prototype.
///
/// This is not an admitted fast path. It exists to exercise ARM intrinsic
/// plumbing, unsafe isolation, and scalar equivalence before a real vector
/// encoder is allowed to participate in dispatch. The current NEON operation
/// only zeroes the output before a scalar fallback loop overwrites every byte.
///
/// Admission note: a real NEON implementation must explicitly clear every
/// vector register that carries caller data before returning, document the
/// exact cleanup sequence in `docs/UNSAFE.md`, and include generated-assembly
/// evidence. This scaffold does not load caller bytes into SIMD registers.
///
/// # Safety
///
/// The caller must execute this function only when NEON is available on the
/// current CPU. NEON is mandatory on `aarch64`; `arm` builds must enable the
/// `neon` target feature. The input and output sizes are fixed by their array
/// types.
#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
unsafe fn encode_12_bytes_neon<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    // SAFETY: `output` is a valid 16-byte mutable array. NEON availability is
    // guaranteed by this function's precondition. `vdupq_n_u8` constructs one
    // NEON vector, and `vst1q_u8` writes it to the provided byte pointer.
    unsafe {
        let zeros: uint8x16_t = vdupq_n_u8(0);
        vst1q_u8(output.as_mut_ptr(), zeros);
    }

    // Temporary scaffolding: the NEON store above only clears the sentinel
    // output bytes. This scalar loop performs the actual encoding and
    // overwrites the whole block, so the current equivalence test does not
    // prove vectorized Base64 correctness.
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
mod tests;
