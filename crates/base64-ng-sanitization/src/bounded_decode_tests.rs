use crate::{
    CtDecodeSanitizationBoundedExt, CtDecodeSanitizationExt, DEFAULT_SECRET_VEC_DECODE_MAX_LEN,
    SecretVecDecodeError,
};
use base64_ng::{DecodeError, ct};

#[test]
fn bounded_secret_vec_decode_enforces_public_capacity_before_output() {
    assert!(matches!(
        ct::STANDARD.decode_secret_vec_bounded::<4>(b"aGVsbG8="),
        Err(SecretVecDecodeError::CapacityLimit {
            maximum: 4,
            actual: 5
        })
    ));

    let secret = ct::STANDARD
        .decode_secret_vec_bounded::<5>(b"aGVsbG8=")
        .unwrap();
    secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
}

#[test]
fn bounded_secret_vec_decode_rejects_oversized_input_before_validation() {
    assert!(matches!(
        ct::STANDARD.decode_secret_vec_bounded::<4>(b"!!!!!!!!!!!!"),
        Err(SecretVecDecodeError::EncodedInputLimit {
            maximum: 8,
            actual: 12
        })
    ));
}

#[test]
fn staged_secret_vec_decode_bounds_validation_by_staging_capacity() {
    assert!(matches!(
        ct::STANDARD.decode_secret_vec_staged_bounded::<64, 4>(b"!!!!!!!!!!!!!!!!"),
        Err(SecretVecDecodeError::EncodedInputLimit {
            maximum: 8,
            actual: 16
        })
    ));
}

#[test]
fn staged_secret_vec_decode_rejects_stage_before_output_allocation() {
    assert!(matches!(
        ct::STANDARD.decode_secret_vec_staged_bounded::<64, 4>(b"aGVsbG8="),
        Err(SecretVecDecodeError::Decode(DecodeError::StagingTooSmall {
            required: 5,
            available: 4
        }))
    ));
}

#[test]
fn default_secret_vec_decode_rejects_output_above_one_mibibyte() {
    let quantum_count = (DEFAULT_SECRET_VEC_DECODE_MAX_LEN / 3) + 1;
    let input = alloc::vec![b'A'; quantum_count * 4];
    assert!(matches!(
        ct::STANDARD.decode_secret_vec(&input),
        Err(SecretVecDecodeError::CapacityLimit {
            maximum: DEFAULT_SECRET_VEC_DECODE_MAX_LEN,
            actual
        }) if actual > DEFAULT_SECRET_VEC_DECODE_MAX_LEN
    ));
}
