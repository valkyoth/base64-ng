#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
#[test]
fn every_x86_encode_threshold_and_downgrade_edge_is_forced() {
    use super::{EncodeBackend, X86_AVX2_MIN_INPUT, X86_AVX512_MIN_INPUT, select_x86};
    use EncodeBackend::{Avx2, Avx512Vbmi, Scalar, Ssse3Sse41};

    let select = |candidate, input_len, rejected| {
        let mut visited = [None; 2];
        let selected = select_x86(candidate, input_len, |backend| {
            if let Some(slot) = visited.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some(backend);
            }
            Some(backend) != rejected
        });
        (selected, visited)
    };

    assert_eq!(select(Avx512Vbmi, 11, None), (Scalar, [None, None]));
    assert_eq!(
        select(Avx512Vbmi, 12, None),
        (Ssse3Sse41, [Some(Ssse3Sse41), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX2_MIN_INPUT, None),
        (Avx2, [Some(Avx2), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX512_MIN_INPUT - 1, None),
        (Avx2, [Some(Avx2), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX512_MIN_INPUT, None),
        (Avx512Vbmi, [Some(Avx512Vbmi), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX512_MIN_INPUT, Some(Avx512Vbmi)),
        (Avx2, [Some(Avx512Vbmi), Some(Avx2)])
    );
    assert_eq!(
        select(Avx2, X86_AVX2_MIN_INPUT, Some(Avx2)),
        (Ssse3Sse41, [Some(Avx2), Some(Ssse3Sse41)])
    );
    assert_eq!(
        select(Ssse3Sse41, 12, Some(Ssse3Sse41)),
        (Scalar, [Some(Ssse3Sse41), None])
    );
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
#[test]
fn every_neon_encode_threshold_and_downgrade_edge_is_forced() {
    use super::{EncodeBackend, NEON_MIN_INPUT, select_neon};

    assert_eq!(
        select_neon(EncodeBackend::Neon, NEON_MIN_INPUT - 1, |_| true),
        EncodeBackend::Scalar
    );
    assert_eq!(
        select_neon(EncodeBackend::Neon, NEON_MIN_INPUT, |_| true),
        EncodeBackend::Neon
    );
    assert_eq!(
        select_neon(EncodeBackend::Neon, NEON_MIN_INPUT, |_| false),
        EncodeBackend::Scalar
    );
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
#[test]
fn every_rvv_encode_threshold_and_downgrade_edge_is_forced() {
    use super::{EncodeBackend, RVV_MIN_INPUT, select_rvv};

    assert_eq!(
        select_rvv(EncodeBackend::Rvv, RVV_MIN_INPUT - 1, |_| true),
        EncodeBackend::Scalar
    );
    assert_eq!(
        select_rvv(EncodeBackend::Rvv, RVV_MIN_INPUT, |_| true),
        EncodeBackend::Rvv
    );
    assert_eq!(
        select_rvv(EncodeBackend::Rvv, RVV_MIN_INPUT, |_| false),
        EncodeBackend::Scalar
    );
}
