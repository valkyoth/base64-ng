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
//! x86 prototypes contain real fixed-block encode logic, the `AArch64` NEON
//! prototype contains real fixed-block encode logic for Standard and URL-safe
//! alphabets, and the wasm `simd128` prototype contains real fixed-block encode
//! logic for Standard and URL-safe alphabets. The 32-bit ARM NEON path remains
//! scalar-equivalence scaffolding. None of these prototypes are reachable from
//! runtime backend selection.

#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
use super::{Alphabet, encode_base64_value};
#[cfg(all(test, target_arch = "aarch64"))]
use core::arch::aarch64::{
    uint8x16_t, uint32x4_t, vaddq_u8, vandq_u8, vandq_u32, vbslq_u8, vceqq_u8, vcgeq_u8, vcltq_u8,
    vdupq_n_u8, vdupq_n_u32, vld1q_u8, vorrq_u32, vqtbl1q_u8, vreinterpretq_u8_u32,
    vreinterpretq_u32_u8, vshlq_n_u32, vshrq_n_u32, vst1q_u8, vsubq_u8,
};
#[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
use core::arch::arm::{uint8x16_t, vdupq_n_u8, vst1q_u8};
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
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
/// encoder is allowed to participate in dispatch. On `aarch64`, Standard and
/// URL-safe alphabets use real NEON fixed-block logic. Other alphabets and
/// 32-bit `arm+neon` builds use the scalar fallback scaffold.
///
/// Admission note: a real NEON implementation must explicitly clear every
/// vector register that carries caller data before returning, document the
/// exact cleanup sequence in `docs/UNSAFE.md`, and include generated-assembly
/// evidence. The `AArch64` prototype clears its used NEON registers before
/// return as best-effort register-retention reduction.
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
    #[cfg(target_arch = "aarch64")]
    {
        if is_standard_or_url_safe_family::<A>() {
            // SAFETY: The caller has proven NEON availability. The helper uses
            // fixed input/output arrays and supports this alphabet family.
            unsafe {
                encode_12_bytes_neon_aarch64_standard_family::<A>(input, output);
            }
            return;
        }
    }

    // Temporary 32-bit ARM/custom-alphabet scaffolding.
    #[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
    // SAFETY: `output` is a valid 16-byte mutable array and NEON availability
    // is guaranteed by this function's precondition.
    unsafe {
        let zeros: uint8x16_t = vdupq_n_u8(0);
        vst1q_u8(output.as_mut_ptr(), zeros);
    }

    scalar_encode_block::<A>(input, output);
}

#[cfg(all(
    test,
    any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
fn scalar_encode_block<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
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

#[cfg(all(test, target_arch = "aarch64"))]
fn is_standard_or_url_safe_family<A>() -> bool
where
    A: Alphabet,
{
    let encode = A::ENCODE;
    let mut index = 0;
    while index < 62 {
        if encode[index] != super::Standard::ENCODE[index] {
            return false;
        }
        index += 1;
    }

    (encode[62] == b'+' && encode[63] == b'/') || (encode[62] == b'-' && encode[63] == b'_')
}

#[cfg(all(test, target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn encode_12_bytes_neon_aarch64_standard_family<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    let mut staged = [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], 0, 0, 0, 0,
    ];
    let shuffle_mask = [2, 1, 0, 255, 5, 4, 3, 255, 8, 7, 6, 255, 11, 10, 9, 255];

    // SAFETY: Fixed arrays back every unaligned 128-bit load/store, the
    // target-feature contract enables NEON, shuffle zero lanes read only
    // staged zeros, and indices are masked to `0..=63`.
    unsafe {
        let input_vec = vld1q_u8(staged.as_ptr());
        let shuffle = vld1q_u8(shuffle_mask.as_ptr());
        let lanes = vqtbl1q_u8(input_vec, shuffle);
        let lane_words: uint32x4_t = vreinterpretq_u32_u8(lanes);

        let index0 = vandq_u32(vshrq_n_u32(lane_words, 18), vdupq_n_u32(0x0000_003f));
        let index1 = vandq_u32(vshrq_n_u32(lane_words, 4), vdupq_n_u32(0x0000_3f00));
        let index2 = vandq_u32(vshlq_n_u32(lane_words, 10), vdupq_n_u32(0x003f_0000));
        let index3 = vandq_u32(vshlq_n_u32(lane_words, 24), vdupq_n_u32(0x3f00_0000));
        let indices = vreinterpretq_u8_u32(vorrq_u32(
            vorrq_u32(index0, index1),
            vorrq_u32(index2, index3),
        ));

        let encoded = encode_standard_family_indices_neon::<A>(indices);
        vst1q_u8(output.as_mut_ptr(), encoded);
        clear_neon_registers_for_test_prototype();
    }
    crate::wipe_bytes(&mut staged);
}

#[cfg(all(test, target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn encode_standard_family_indices_neon<A>(indices: uint8x16_t) -> uint8x16_t
where
    A: Alphabet,
{
    let upper = vcltq_u8(indices, vdupq_n_u8(26));
    let lower = vandq_u8(
        vcgeq_u8(indices, vdupq_n_u8(26)),
        vcltq_u8(indices, vdupq_n_u8(52)),
    );
    let digit = vandq_u8(
        vcgeq_u8(indices, vdupq_n_u8(52)),
        vcltq_u8(indices, vdupq_n_u8(62)),
    );
    let plus = vceqq_u8(indices, vdupq_n_u8(62));
    let slash = vceqq_u8(indices, vdupq_n_u8(63));
    let plus_char = A::ENCODE[62];
    let slash_char = A::ENCODE[63];

    let mut encoded = vdupq_n_u8(0);
    encoded = vbslq_u8(upper, vaddq_u8(indices, vdupq_n_u8(b'A')), encoded);
    encoded = vbslq_u8(
        lower,
        vaddq_u8(vsubq_u8(indices, vdupq_n_u8(26)), vdupq_n_u8(b'a')),
        encoded,
    );
    encoded = vbslq_u8(
        digit,
        vaddq_u8(vsubq_u8(indices, vdupq_n_u8(52)), vdupq_n_u8(b'0')),
        encoded,
    );
    encoded = vbslq_u8(plus, vdupq_n_u8(plus_char), encoded);
    vbslq_u8(slash, vdupq_n_u8(slash_char), encoded)
}

#[cfg(all(test, target_arch = "aarch64"))]
unsafe fn clear_neon_registers_for_test_prototype() {
    // SAFETY: This test-only cleanup runs after the prototype stores its
    // output. The explicit outputs tell the compiler these vector registers are
    // clobbered while the assembly clears them.
    unsafe {
        core::arch::asm!(
            "eor v0.16b, v0.16b, v0.16b",
            "eor v1.16b, v1.16b, v1.16b",
            "eor v2.16b, v2.16b, v2.16b",
            "eor v3.16b, v3.16b, v3.16b",
            "eor v4.16b, v4.16b, v4.16b",
            "eor v5.16b, v5.16b, v5.16b",
            "eor v6.16b, v6.16b, v6.16b",
            "eor v7.16b, v7.16b, v7.16b",
            out("v0") _,
            out("v1") _,
            out("v2") _,
            out("v3") _,
            out("v4") _,
            out("v5") _,
            out("v6") _,
            out("v7") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
