//! Pure strict-decode dispatch policy and frozen automatic thresholds.

#[cfg(any(
    all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
    all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
    all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    )
))]
use super::DecodeBackend;

pub(super) const MIN_SIMD_INPUT: usize = 16;

#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) const X86_AVX2_MIN_INPUT: usize = 32;

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(super) const NEON_MIN_INPUT: usize = 256;

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
pub(super) const RVV_MIN_INPUT: usize = 192;

#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) fn select_x86(
    candidate: DecodeBackend,
    input_len: usize,
    mut admit: impl FnMut(DecodeBackend) -> bool,
) -> DecodeBackend {
    // AVX-512 remains an exact/static backend. Two Commit 34 campaigns did
    // not establish the required automatic advantage over AVX2.
    if matches!(candidate, DecodeBackend::Avx512Vbmi | DecodeBackend::Avx2)
        && input_len >= X86_AVX2_MIN_INPUT
        && admit(DecodeBackend::Avx2)
    {
        return DecodeBackend::Avx2;
    }
    if matches!(
        candidate,
        DecodeBackend::Avx512Vbmi | DecodeBackend::Avx2 | DecodeBackend::Ssse3Sse41
    ) && input_len >= MIN_SIMD_INPUT
        && admit(DecodeBackend::Ssse3Sse41)
    {
        return DecodeBackend::Ssse3Sse41;
    }
    DecodeBackend::Scalar
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(super) fn select_neon(
    candidate: DecodeBackend,
    input_len: usize,
    mut admit: impl FnMut(DecodeBackend) -> bool,
) -> DecodeBackend {
    if candidate == DecodeBackend::Neon && input_len >= NEON_MIN_INPUT && admit(DecodeBackend::Neon)
    {
        DecodeBackend::Neon
    } else {
        DecodeBackend::Scalar
    }
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
pub(super) fn select_rvv(
    candidate: DecodeBackend,
    input_len: usize,
    mut admit: impl FnMut(DecodeBackend) -> bool,
) -> DecodeBackend {
    if candidate == DecodeBackend::Rvv && input_len >= RVV_MIN_INPUT && admit(DecodeBackend::Rvv) {
        DecodeBackend::Rvv
    } else {
        DecodeBackend::Scalar
    }
}

#[cfg(test)]
mod tests;
