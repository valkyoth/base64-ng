//! Pure encode dispatch policy and frozen automatic thresholds.

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
use super::EncodeBackend;

pub(super) const MIN_SIMD_INPUT: usize = 12;

#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) const X86_AVX2_MIN_INPUT: usize = 24;
#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) const X86_AVX512_MIN_INPUT: usize = 192;

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(super) const NEON_MIN_INPUT: usize = 192;

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
pub(super) const RVV_MIN_INPUT: usize = 192;

#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
pub(super) fn select_x86(
    candidate: EncodeBackend,
    input_len: usize,
    mut admit: impl FnMut(EncodeBackend) -> bool,
) -> EncodeBackend {
    if candidate == EncodeBackend::Avx512Vbmi
        && input_len >= X86_AVX512_MIN_INPUT
        && admit(EncodeBackend::Avx512Vbmi)
    {
        return EncodeBackend::Avx512Vbmi;
    }
    if matches!(candidate, EncodeBackend::Avx512Vbmi | EncodeBackend::Avx2)
        && input_len >= X86_AVX2_MIN_INPUT
        && admit(EncodeBackend::Avx2)
    {
        return EncodeBackend::Avx2;
    }
    if matches!(
        candidate,
        EncodeBackend::Avx512Vbmi | EncodeBackend::Avx2 | EncodeBackend::Ssse3Sse41
    ) && input_len >= MIN_SIMD_INPUT
        && admit(EncodeBackend::Ssse3Sse41)
    {
        return EncodeBackend::Ssse3Sse41;
    }
    EncodeBackend::Scalar
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
pub(super) fn select_neon(
    candidate: EncodeBackend,
    input_len: usize,
    mut admit: impl FnMut(EncodeBackend) -> bool,
) -> EncodeBackend {
    if candidate == EncodeBackend::Neon && input_len >= NEON_MIN_INPUT && admit(EncodeBackend::Neon)
    {
        EncodeBackend::Neon
    } else {
        EncodeBackend::Scalar
    }
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
pub(super) fn select_rvv(
    candidate: EncodeBackend,
    input_len: usize,
    mut admit: impl FnMut(EncodeBackend) -> bool,
) -> EncodeBackend {
    if candidate == EncodeBackend::Rvv && input_len >= RVV_MIN_INPUT && admit(EncodeBackend::Rvv) {
        EncodeBackend::Rvv
    } else {
        EncodeBackend::Scalar
    }
}

#[cfg(test)]
mod tests;
