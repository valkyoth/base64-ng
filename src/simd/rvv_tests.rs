use crate::runtime::{Backend, CandidateDetectionMode, SecurityPosture};
use crate::{Alphabet, Standard, UrlSafe, checked_encoded_len, scalar};

#[test]
fn qemu_runtime_detects_rvv_but_keeps_public_dispatch_scalar() {
    assert!(super::rvv::available());
    let vlenb = super::rvv::vector_length_bytes();
    assert!(vlenb >= 16 && vlenb.is_power_of_two());
    eprintln!("RVV candidate VLEN={} bits", vlenb * 8);
    assert_eq!(super::detected_candidate(), super::Candidate::Rvv);

    let report = crate::runtime::backend_report();
    assert_eq!(report.candidate, Backend::Rvv);
    assert_eq!(
        report.candidate_detection_mode,
        CandidateDetectionMode::RuntimeCpuFeatures
    );
    assert_eq!(report.active, Backend::Scalar);
    assert_eq!(report.encode_backend.backend.as_str(), "scalar");
    assert_eq!(report.strict_decode_backend.backend.as_str(), "scalar");
    assert!(!report.ordinary_acceleration_active);
    assert_eq!(
        report.security_posture,
        SecurityPosture::SimdCandidateScalarActive
    );
}

#[test]
fn rvv_candidate_matches_scalar_for_profiles_blocks_and_tails() {
    assert_candidate_round_trips::<Standard, true>();
    assert_candidate_round_trips::<Standard, false>();
    assert_candidate_round_trips::<UrlSafe, true>();
    assert_candidate_round_trips::<UrlSafe, false>();
}

#[test]
fn rvv_candidate_rejects_before_writing_output() {
    let mut encoded = [b'A'; 64];
    encoded[47] = b'!';
    let mut output = [0x5a; 48];
    let before = output;

    assert!(super::rvv::decode_slice::<Standard, false>(&encoded, &mut output).is_err());
    assert_eq!(output, before);
}

#[test]
fn rvv_probe_fails_closed_across_kernel_and_vector_state_results() {
    use super::rvv::probe_allows_rvv;

    assert!(probe_allows_rvv(true, true, true, 2));
    assert!(!probe_allows_rvv(true, false, true, 2));
    assert!(!probe_allows_rvv(true, true, true, 0));
    assert!(probe_allows_rvv(false, false, true, -1));
    assert!(!probe_allows_rvv(false, false, false, -1));
    assert!(!probe_allows_rvv(false, false, true, 0));
}

#[test]
fn unavailable_rvv_candidate_falls_back_to_scalar_without_assembly_entry() {
    let mut input = [0u8; 48];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(29).wrapping_add(7);
    }

    let mut expected_encoded = [0u8; 64];
    let mut fallback_encoded = [0u8; 64];
    let expected_written =
        scalar::encode_slice::<Standard, true>(&input, &mut expected_encoded).unwrap();
    let fallback_written = super::rvv::encode_slice_unavailable_for_test::<Standard, true>(
        &input,
        &mut fallback_encoded,
    )
    .unwrap();
    assert_eq!(fallback_written, expected_written);
    assert_eq!(fallback_encoded, expected_encoded);

    let mut expected_decoded = [0u8; 48];
    let mut fallback_decoded = [0u8; 48];
    let expected_decoded_len = scalar::decode_slice::<Standard, true>(
        &expected_encoded[..expected_written],
        &mut expected_decoded,
    )
    .unwrap();
    let fallback_decoded_len = super::rvv::decode_slice_unavailable_for_test::<Standard, true>(
        &fallback_encoded[..fallback_written],
        &mut fallback_decoded,
    )
    .unwrap();
    assert_eq!(fallback_decoded_len, expected_decoded_len);
    assert_eq!(fallback_decoded, expected_decoded);
}

fn assert_candidate_round_trips<A: Alphabet, const PAD: bool>() {
    let mut input = [0u8; 513];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(73).wrapping_add(19);
    }

    for len in 0..=input.len() {
        let required = checked_encoded_len(len, PAD).unwrap();
        let mut expected_encoded = [0u8; 684];
        let mut candidate_encoded = [0u8; 684];
        let expected_written =
            scalar::encode_slice::<A, PAD>(&input[..len], &mut expected_encoded).unwrap();
        let candidate_written =
            super::rvv::encode_slice::<A, PAD>(&input[..len], &mut candidate_encoded).unwrap();
        assert_eq!(expected_written, required);
        assert_eq!(candidate_written, expected_written);
        assert_eq!(
            &candidate_encoded[..candidate_written],
            &expected_encoded[..expected_written]
        );

        let mut expected_decoded = [0u8; 513];
        let mut candidate_decoded = [0u8; 513];
        let expected_decoded_len = scalar::decode_slice::<A, PAD>(
            &expected_encoded[..expected_written],
            &mut expected_decoded,
        )
        .unwrap();
        let candidate_decoded_len = super::rvv::decode_slice::<A, PAD>(
            &candidate_encoded[..candidate_written],
            &mut candidate_decoded,
        )
        .unwrap();
        assert_eq!(candidate_decoded_len, expected_decoded_len);
        assert_eq!(&candidate_decoded[..candidate_decoded_len], &input[..len]);
    }
}
