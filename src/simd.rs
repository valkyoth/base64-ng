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
//! The fixed-block prototypes below are test-only scaffolding. Their SIMD
//! operations currently zero the destination block, then a scalar loop
//! overwrites the entire output. The prototype tests therefore validate
//! target-feature gating, unsafe isolation, and fixed-size plumbing, not
//! vectorized Base64 correctness. They are not compiled into release library
//! builds.

#[cfg(all(
    test,
    any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    )
))]
use super::{Alphabet, encode_base64_value};
#[cfg(all(test, target_arch = "aarch64"))]
use core::arch::aarch64::{uint8x16_t, vdupq_n_u8, vst1q_u8};
#[cfg(all(test, target_arch = "arm", target_feature = "neon"))]
use core::arch::arm::{uint8x16_t, vdupq_n_u8, vst1q_u8};
// Keep intrinsic imports limited to operations used by the current scaffolding.
// Adding shuffle, table-lookup, permutation, compare, or arithmetic intrinsics
// is SIMD admission work and must update docs/SIMD_ACTIVATION_CHECKLIST.md,
// unsafe inventory, differential tests, fuzz evidence, and benchmark evidence.
#[cfg(all(test, target_arch = "x86"))]
use core::arch::x86::{
    __m128i, __m256i, __m512i, _mm_setzero_si128, _mm_storeu_si128, _mm256_setzero_si256,
    _mm256_storeu_si256, _mm512_setzero_si512, _mm512_storeu_si512,
};
#[cfg(all(test, target_arch = "x86_64"))]
use core::arch::x86_64::{
    __m128i, __m256i, __m512i, _mm_setzero_si128, _mm_storeu_si128, _mm256_setzero_si256,
    _mm256_storeu_si256, _mm512_setzero_si512, _mm512_storeu_si512,
};

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

/// Encodes one 48-byte block into 64 bytes through the inactive AVX-512 prototype.
///
/// This is not an admitted fast path. It exists to exercise AVX-512 target
/// feature plumbing, unsafe isolation, and scalar equivalence tests before a
/// real vector encoder is allowed to participate in dispatch. The current
/// SIMD operation only zeroes the output before a scalar fallback loop
/// overwrites every byte.
///
/// Admission note: a real AVX-512 implementation must explicitly clear every
/// ZMM/YMM/XMM register that carries caller data before returning, document the
/// exact cleanup sequence in `docs/UNSAFE.md`, and include generated-assembly
/// evidence. This scaffold does not load caller bytes into SIMD registers.
///
/// # Safety
///
/// The caller must execute this function only when the full AVX-512 Base64
/// candidate bundle is available on the current CPU: `avx512f`, `avx512bw`,
/// `avx512vl`, and `avx512vbmi`. The input and output sizes are fixed by their
/// array types.
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm512_storeu_si512 accepts unaligned pointers"
)]
#[target_feature(enable = "avx512f,avx512bw,avx512vl,avx512vbmi")]
unsafe fn encode_48_bytes_avx512<A>(input: &[u8; 48], output: &mut [u8; 64])
where
    A: Alphabet,
{
    let zeros = _mm512_setzero_si512();
    // SAFETY: `output` is a valid 64-byte mutable array. The full AVX-512
    // candidate bundle is guaranteed by this function's target-feature
    // precondition, and the unaligned store does not require stronger pointer
    // alignment.
    unsafe {
        _mm512_storeu_si512(output.as_mut_ptr().cast::<__m512i>(), zeros);
    }

    // Temporary scaffolding: the AVX-512 store above only clears the sentinel
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

/// Encodes one 24-byte block into 32 bytes through the inactive AVX2 prototype.
///
/// This is not an admitted fast path. It exists to exercise target-feature
/// gating, unsafe isolation, and scalar equivalence tests before a real vector
/// encoder is allowed to participate in dispatch. The current SIMD operation
/// only zeroes the output before a scalar fallback loop overwrites every byte.
///
/// Admission note: a real AVX2 implementation must explicitly clear every
/// YMM/XMM register that carries caller data before returning, include the
/// required AVX/SSE transition cleanup such as `vzeroupper` where applicable,
/// document the exact sequence in `docs/UNSAFE.md`, and include
/// generated-assembly evidence. This scaffold does not load caller bytes into
/// SIMD registers.
///
/// # Safety
///
/// The caller must execute this function only when AVX2 is available on the
/// current CPU. The input and output sizes are fixed by their array types.
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm256_storeu_si256 accepts unaligned pointers"
)]
#[target_feature(enable = "avx2")]
unsafe fn encode_24_bytes_avx2<A>(input: &[u8; 24], output: &mut [u8; 32])
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

    // Temporary scaffolding: the AVX2 store above only clears the sentinel
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

/// Encodes one 12-byte block into 16 bytes through the inactive SSSE3/SSE4.1 prototype.
///
/// This is not an admitted fast path. It exists to exercise lower-tier x86
/// target-feature plumbing, unsafe isolation, and scalar equivalence before a
/// real vector encoder is allowed to participate in dispatch. The current
/// SIMD operation only zeroes the output before a scalar fallback loop
/// overwrites every byte.
///
/// Admission note: a real SSSE3/SSE4.1 implementation must explicitly clear
/// every XMM register that carries caller data before returning, document the
/// exact cleanup sequence in `docs/UNSAFE.md`, and include generated-assembly
/// evidence. This scaffold does not load caller bytes into SIMD registers.
///
/// # Safety
///
/// The caller must execute this function only when SSSE3 and SSE4.1 are
/// available on the current CPU. The input and output sizes are fixed by their
/// array types.
#[cfg(all(test, any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(dead_code, reason = "inactive prototype is not dispatchable yet")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "_mm_storeu_si128 accepts unaligned pointers"
)]
#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn encode_12_bytes_ssse3_sse41<A>(input: &[u8; 12], output: &mut [u8; 16])
where
    A: Alphabet,
{
    let zeros = _mm_setzero_si128();
    // SAFETY: `output` is a valid 16-byte mutable array. SSSE3/SSE4.1
    // availability is guaranteed by this function's target-feature
    // precondition, and the unaligned store does not require stronger pointer
    // alignment.
    unsafe {
        _mm_storeu_si128(output.as_mut_ptr().cast::<__m128i>(), zeros);
    }

    // Temporary scaffolding: the SSSE3/SSE4.1 store above only clears the
    // sentinel output bytes. This scalar loop performs the actual encoding and
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
mod tests {
    use super::*;
    use crate::{Engine, Standard, UrlSafe};

    fn fill_pattern(output: &mut [u8], seed: usize) {
        for (index, byte) in output.iter_mut().enumerate() {
            let value = (index * 73 + seed * 19) % 256;
            *byte = u8::try_from(value).unwrap();
        }
    }

    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    #[test]
    fn avx512_encode_prototype_matches_scalar_when_available() {
        if !avx512_vbmi_base64_available() {
            println!(
                "skipped: AVX-512 VBMI prototype test requires avx512f,avx512bw,avx512vl,avx512vbmi"
            );
            return;
        }

        let mut input = [0; 48];
        for seed in 0..64 {
            fill_pattern(&mut input, seed);

            let mut avx512_standard = [0x55; 64];
            let mut scalar_standard = [0xaa; 64];
            // SAFETY: The candidate check above uses runtime AVX-512 feature
            // bundle detection on std builds and compile-time target-feature
            // detection otherwise.
            unsafe {
                encode_48_bytes_avx512::<Standard>(&input, &mut avx512_standard);
            }
            let scalar_len = Engine::<Standard, true>::new()
                .encode_slice(&input, &mut scalar_standard)
                .unwrap();
            assert_eq!(scalar_len, avx512_standard.len());
            assert_eq!(avx512_standard, scalar_standard);

            let mut avx512_url_safe = [0x55; 64];
            let mut scalar_url_safe = [0xaa; 64];
            // SAFETY: The candidate check above proves the AVX-512 feature
            // bundle is available for this test invocation.
            unsafe {
                encode_48_bytes_avx512::<UrlSafe>(&input, &mut avx512_url_safe);
            }
            let scalar_len = Engine::<UrlSafe, true>::new()
                .encode_slice(&input, &mut scalar_url_safe)
                .unwrap();
            assert_eq!(scalar_len, avx512_url_safe.len());
            assert_eq!(avx512_url_safe, scalar_url_safe);
        }
    }

    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    #[test]
    fn avx2_encode_prototype_matches_scalar_when_available() {
        if !avx2_available() {
            println!("skipped: AVX2 prototype test requires avx2");
            return;
        }

        let mut input = [0; 24];
        for seed in 0..64 {
            fill_pattern(&mut input, seed);

            let mut avx2_standard = [0x55; 32];
            let mut scalar_standard = [0xaa; 32];
            // SAFETY: The feature check above uses runtime AVX2 detection on
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
            // SAFETY: The feature check above proves AVX2 availability for
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

    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    #[test]
    fn ssse3_sse41_encode_prototype_matches_scalar_when_available() {
        if !ssse3_sse41_available() {
            println!("skipped: SSSE3/SSE4.1 prototype test requires ssse3 and sse4.1");
            return;
        }

        let mut input = [0; 12];
        for seed in 0..64 {
            fill_pattern(&mut input, seed);

            let mut ssse3_standard = [0x55; 16];
            let mut scalar_standard = [0xaa; 16];
            // SAFETY: The feature check above uses runtime SSSE3/SSE4.1
            // detection on std builds and compile-time target-feature
            // detection otherwise.
            unsafe {
                encode_12_bytes_ssse3_sse41::<Standard>(&input, &mut ssse3_standard);
            }
            let scalar_len = Engine::<Standard, true>::new()
                .encode_slice(&input, &mut scalar_standard)
                .unwrap();
            assert_eq!(scalar_len, ssse3_standard.len());
            assert_eq!(ssse3_standard, scalar_standard);

            let mut ssse3_url_safe = [0x55; 16];
            let mut scalar_url_safe = [0xaa; 16];
            // SAFETY: The feature check above proves SSSE3/SSE4.1
            // availability for this test invocation.
            unsafe {
                encode_12_bytes_ssse3_sse41::<UrlSafe>(&input, &mut ssse3_url_safe);
            }
            let scalar_len = Engine::<UrlSafe, true>::new()
                .encode_slice(&input, &mut scalar_url_safe)
                .unwrap();
            assert_eq!(scalar_len, ssse3_url_safe.len());
            assert_eq!(ssse3_url_safe, scalar_url_safe);
        }
    }

    #[cfg(any(
        target_arch = "aarch64",
        all(target_arch = "arm", target_feature = "neon")
    ))]
    #[test]
    fn neon_encode_prototype_matches_scalar_when_available() {
        if !neon_available() {
            println!("skipped: NEON prototype test requires aarch64 or arm+neon");
            return;
        }

        let mut input = [0; 12];
        for seed in 0..64 {
            fill_pattern(&mut input, seed);

            let mut neon_standard = [0x55; 16];
            let mut scalar_standard = [0xaa; 16];
            // SAFETY: The candidate check above proves NEON availability for
            // this test invocation.
            unsafe {
                encode_12_bytes_neon::<Standard>(&input, &mut neon_standard);
            }
            let scalar_len = Engine::<Standard, true>::new()
                .encode_slice(&input, &mut scalar_standard)
                .unwrap();
            assert_eq!(scalar_len, neon_standard.len());
            assert_eq!(neon_standard, scalar_standard);

            let mut neon_url_safe = [0x55; 16];
            let mut scalar_url_safe = [0xaa; 16];
            // SAFETY: The candidate check above proves NEON availability for
            // this test invocation.
            unsafe {
                encode_12_bytes_neon::<UrlSafe>(&input, &mut neon_url_safe);
            }
            let scalar_len = Engine::<UrlSafe, true>::new()
                .encode_slice(&input, &mut scalar_url_safe)
                .unwrap();
            assert_eq!(scalar_len, neon_url_safe.len());
            assert_eq!(neon_url_safe, scalar_url_safe);
        }
    }
}
