use crate::runtime::{Backend, CandidateDetectionMode, SecurityPosture};
use crate::{Alphabet, Standard, UrlSafe, checked_encoded_len, scalar};

#[test]
fn qemu_runtime_detects_sve_but_keeps_public_dispatch_neon() {
    let vector_length = super::sve::vector_length_bytes().expect("SVE must be available");
    assert_eq!(super::sve::instruction_vector_length_bytes(), vector_length);
    eprintln!("SVE candidate vector length={} bits", vector_length * 8);
    assert_eq!(super::detected_candidate(), super::Candidate::Sve);

    let report = crate::runtime::backend_report();
    assert_eq!(report.candidate, Backend::Sve);
    assert_eq!(
        report.candidate_detection_mode,
        CandidateDetectionMode::RuntimeCpuFeatures
    );
    assert_eq!(report.active, Backend::Neon);
    assert_eq!(report.encode_backend.backend.as_str(), "neon");
    assert_eq!(report.strict_decode_backend.backend.as_str(), "neon");
    assert!(report.ordinary_acceleration_active);
    assert_eq!(report.security_posture, SecurityPosture::Accelerated);
}

#[test]
fn sve_candidate_matches_scalar_for_profiles_blocks_and_tails() {
    assert_candidate_round_trips::<Standard, true>();
    assert_candidate_round_trips::<Standard, false>();
    assert_candidate_round_trips::<UrlSafe, true>();
    assert_candidate_round_trips::<UrlSafe, false>();
}

#[test]
fn sve_candidate_rejects_before_writing_output() {
    let mut encoded = [b'A'; 64];
    encoded[47] = b'!';
    let mut output = [0x5a; 48];
    let before = output;

    assert!(super::sve::decode_slice::<Standard, false>(&encoded, &mut output).is_err());
    assert_eq!(output, before);
}

#[test]
fn sve_probe_fails_closed_for_missing_or_malformed_kernel_results() {
    use super::sve::probe_vector_length;

    assert_eq!(probe_vector_length(true, 16), Some(16));
    assert_eq!(probe_vector_length(true, 32), Some(32));
    assert_eq!(probe_vector_length(true, (1 << 17) | 64), Some(64));
    assert_eq!(probe_vector_length(false, 32), None);
    assert_eq!(probe_vector_length(true, -1), None);
    assert_eq!(probe_vector_length(true, 0), None);
    assert_eq!(probe_vector_length(true, 15), None);
    assert_eq!(probe_vector_length(true, 24), None);
    assert_eq!(probe_vector_length(true, 272), None);
}

#[cfg(target_os = "linux")]
#[test]
fn sve_candidate_observes_per_thread_vector_length_changes() {
    let original = super::sve::vector_length_bytes().expect("SVE must be available");
    if original == 16 {
        return;
    }

    let reduced = super::sve::set_vector_length_for_test(16);
    assert_eq!(reduced, Some(16));
    assert_eq!(super::sve::vector_length_bytes(), Some(16));
    assert_candidate_round_trips::<Standard, true>();
    let restored = super::sve::set_vector_length_for_test(original);
    assert_eq!(restored, Some(original));
    assert_eq!(super::sve::vector_length_bytes(), Some(original));
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
            super::sve::encode_slice::<A, PAD>(&input[..len], &mut candidate_encoded).unwrap();
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
        let candidate_decoded_len = super::sve::decode_slice::<A, PAD>(
            &candidate_encoded[..candidate_written],
            &mut candidate_decoded,
        )
        .unwrap();
        assert_eq!(candidate_decoded_len, expected_decoded_len);
        assert_eq!(&candidate_decoded[..candidate_decoded_len], &input[..len]);
    }
}
