use super::*;
use crate::{Alphabet, scalar};

mod benchmark;
#[cfg(base64_ng_rvv_candidate)]
mod guards;

fn executable() -> bool {
    let enabled = super::super::execution_available();
    if !enabled {
        std::eprintln!("RVV validation: SKIP direct execution (RVV unavailable)");
    }
    enabled
}

#[test]
#[ignore = "requires the exact native Linux X60 profile; not QEMU admission"]
fn rvv_native_exact_profile_is_required() {
    assert_eq!(width(Backend::Rvv), Some(16));
    let _ = crate::initialize_backends();
    assert_eq!(
        crate::runtime::backend_report().active_decode_backend(),
        Backend::Rvv
    );
    std::eprintln!("RVV validation: exact native Linux X60 execution");
}

#[test]
fn rvv_public_boundary_keeps_exact_profile_and_geometry() {
    let exact = super::super::available();
    assert_eq!(width(Backend::Rvv), exact.then_some(16));
    for backend in [Backend::Scalar, Backend::Neon, Backend::Avx2] {
        assert_eq!(width(backend), None);
        assert!(!validate(backend, &[b'A'; 16], false));
    }
    for len in [0, 1, 15, 16, 17, 32, 48, 64] {
        let input = std::vec![b'A'; len];
        let mut output = std::vec![0xa5; len / 4 * 3];
        let accepted = exact && len % 16 == 0;
        assert_eq!(validate(Backend::Rvv, &input, false), accepted);
        assert_eq!(decode(Backend::Rvv, &input, &mut output, false), accepted);
        assert!(
            output
                .iter()
                .all(|&byte| byte == if accepted { 0 } else { 0xa5 })
        );
        let mut oversized = std::vec![0xa5; output.len() + 1];
        assert!(!decode(Backend::Rvv, &input, &mut oversized, false));
        assert!(oversized.iter().all(|&byte| byte == 0xa5));
    }
}

#[test]
fn rvv_classifier_all_bytes_lanes_offsets_and_partial_vl() {
    if !executable() {
        return;
    }
    // Cross 16/32-byte VLENs repeatedly, with non-full final VLs and unaligned
    // starts. The leaf accepts arbitrary byte lengths; the writer does not.
    for (alphabet, url) in [(Standard::ENCODE, false), (UrlSafe::ENCODE, true)] {
        for offset in [0, 1, 15] {
            let mut storage = [b'A'; 144];
            let input = &mut storage[offset..offset + 129];
            for lane in 0..input.len() {
                for byte in 0..=u8::MAX {
                    input[lane] = byte;
                    assert_eq!(classify(input, url), alphabet.contains(&byte));
                }
                input[lane] = b'A';
            }
        }
        for len in 0..=129 {
            let mut input = std::vec![b'A'; len];
            assert!(classify(&input, url));
            if len != 0 {
                input[len - 1] = b'=';
                assert!(!classify(&input, url));
                input[0] = 0xff;
                assert!(!classify(&input, url));
            }
        }
    }
}

#[test]
fn rvv_writer_matches_scalar_and_rejects_without_stores() {
    if !executable() {
        return;
    }
    for (alphabet, url) in [(Standard::ENCODE, false), (UrlSafe::ENCODE, true)] {
        for len in [0, 16, 32, 48, 64, 80, 128, 144, 1024, 4096] {
            let mut input: std::vec::Vec<_> = (0..len).map(|i| alphabet[i % 64]).collect();
            let mut reference = std::vec![0xa5; len / 4 * 3 + 16];
            let mut output = reference.clone();
            let written = if url {
                scalar::decode_slice::<UrlSafe, false>(&input, &mut reference[1..]).unwrap()
            } else {
                scalar::decode_slice::<Standard, false>(&input, &mut reference[1..]).unwrap()
            };
            assert!(decode_available(&input, &mut output[1..][..written], url));
            assert_eq!(output, reference);
            if len != 0 {
                for position in [0, len / 2, len - 1] {
                    let saved = input[position];
                    input[position] = 0xff;
                    output.fill(0xa5);
                    assert!(!decode_available(&input, &mut output[1..][..written], url));
                    assert!(output.iter().all(|&b| b == 0xa5));
                    input[position] = saved;
                }
            }
        }
    }
}
