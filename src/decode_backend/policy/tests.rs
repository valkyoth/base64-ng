#[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
#[test]
fn every_x86_decode_threshold_and_downgrade_edge_is_forced() {
    use super::{DecodeBackend, X86_AVX2_MIN_INPUT, select_x86};
    use DecodeBackend::{Avx2, Avx512Vbmi, Scalar, Ssse3Sse41};

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

    assert_eq!(select(Avx512Vbmi, 15, None), (Scalar, [None, None]));
    assert_eq!(
        select(Avx512Vbmi, 16, None),
        (Ssse3Sse41, [Some(Ssse3Sse41), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX2_MIN_INPUT, None),
        (Avx2, [Some(Avx2), None])
    );
    assert_eq!(
        select(Avx512Vbmi, usize::MAX, None),
        (Avx2, [Some(Avx2), None])
    );
    assert_eq!(
        select(Avx512Vbmi, X86_AVX2_MIN_INPUT, Some(Avx2)),
        (Ssse3Sse41, [Some(Avx2), Some(Ssse3Sse41)])
    );
    assert_eq!(
        select(Ssse3Sse41, 16, Some(Ssse3Sse41)),
        (Scalar, [Some(Ssse3Sse41), None])
    );
}

#[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
#[test]
fn every_neon_decode_threshold_and_downgrade_edge_is_forced() {
    use super::{DecodeBackend, NEON_MIN_INPUT, select_neon};

    assert_eq!(
        select_neon(DecodeBackend::Neon, NEON_MIN_INPUT - 1, |_| true),
        DecodeBackend::Scalar
    );
    assert_eq!(
        select_neon(DecodeBackend::Neon, NEON_MIN_INPUT, |_| true),
        DecodeBackend::Neon
    );
    assert_eq!(
        select_neon(DecodeBackend::Neon, NEON_MIN_INPUT, |_| false),
        DecodeBackend::Scalar
    );
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    target_arch = "riscv64",
    target_os = "linux"
))]
#[test]
fn every_rvv_decode_threshold_and_downgrade_edge_is_forced() {
    use super::{DecodeBackend, RVV_MIN_INPUT, select_rvv};

    assert_eq!(
        select_rvv(DecodeBackend::Rvv, RVV_MIN_INPUT - 1, |_| true),
        DecodeBackend::Scalar
    );
    assert_eq!(
        select_rvv(DecodeBackend::Rvv, RVV_MIN_INPUT, |_| true),
        DecodeBackend::Rvv
    );
    assert_eq!(
        select_rvv(DecodeBackend::Rvv, RVV_MIN_INPUT, |_| false),
        DecodeBackend::Scalar
    );
}
