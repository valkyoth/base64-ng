#![cfg(target_endian = "big")]

use base64_ng::{
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED,
    runtime::{Backend, OperationSecurityPosture, backend_report},
};

const SENTINEL: u8 = 0xa5;

#[test]
fn target_and_runtime_report_remain_big_endian_scalar() {
    assert!(cfg!(target_endian = "big"));
    let report = backend_report();
    assert_eq!(report.active, Backend::Scalar);
    assert_eq!(report.candidate, Backend::Scalar);
    assert_eq!(report.encode_backend.backend.as_str(), "scalar");
    assert_eq!(report.strict_decode_backend.backend.as_str(), "scalar");
    assert_eq!(
        report.encode_backend.security_posture,
        OperationSecurityPosture::OrdinaryScalar
    );
    assert_eq!(
        report.strict_decode_backend.security_posture,
        OperationSecurityPosture::OrdinaryScalar
    );
    assert!(!report.ordinary_acceleration_active);
}

#[test]
fn strict_profiles_round_trip_every_short_length_and_in_place_surface() {
    exercise_codec(&STRICT_STANDARD_PADDED);
    exercise_codec(&STRICT_STANDARD_UNPADDED);
    exercise_codec(&STRICT_URL_SAFE_PADDED);
    exercise_codec(&STRICT_URL_SAFE_UNPADDED);
}

#[test]
fn malformed_input_keeps_transactional_destination_unchanged() {
    for malformed in [
        b"A===".as_slice(),
        b"AB=C".as_slice(),
        b"Zh==".as_slice(),
        b"Zm9v!mFy".as_slice(),
    ] {
        let mut output = [SENTINEL; 32];
        assert!(
            STRICT_STANDARD_PADDED
                .decode_into(malformed, &mut output)
                .is_err()
        );
        assert_eq!(output, [SENTINEL; 32]);
    }
}

#[cfg(feature = "secrets")]
#[test]
fn rejected_secret_frame_wipes_staging_and_output() {
    use base64_ng::secret::{SecretDecodeError, SecretFrame, SecretInput};

    let mut staging = [SENTINEL; 32];
    let mut output = [SENTINEL; 32];
    {
        let mut frame =
            SecretFrame::new(&STRICT_STANDARD_PADDED, 32, &mut staging, &mut output).unwrap();
        frame.update(&SecretInput::new(b"c2Vj!mV0")).unwrap();
        assert!(matches!(
            frame.finish(),
            Err(SecretDecodeError::InvalidInput)
        ));
    }
    assert_eq!(staging, [0; 32]);
    assert_eq!(output, [0; 32]);
}

fn exercise_codec<S: base64_ng::Codec>(codec: &base64_ng::Base64<S>) {
    let mut input = [0u8; 129];
    fill_pattern(&mut input);

    for len in 0..=input.len() {
        let required = codec.encoded_len(len).unwrap();
        let mut encoded = [SENTINEL; 176];
        let encoded_len = codec.encode_into(&input[..len], &mut encoded).unwrap();
        assert_eq!(encoded_len, required);

        let mut decoded = [SENTINEL; 129];
        let decoded_len = codec
            .decode_into(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], &input[..len]);

        let mut in_place = [SENTINEL; 176];
        in_place[..len].copy_from_slice(&input[..len]);
        let in_place_encoded = codec.encode_in_place(&mut in_place, len).unwrap();
        assert_eq!(&in_place[..in_place_encoded], &encoded[..encoded_len]);
        let in_place_decoded = codec
            .decode_in_place(&mut in_place, in_place_encoded)
            .unwrap();
        assert_eq!(&in_place[..in_place_decoded], &input[..len]);
    }
}

fn fill_pattern(bytes: &mut [u8]) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap_or(0)
            .wrapping_mul(73)
            .wrapping_add(29);
    }
}
