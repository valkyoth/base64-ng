#![cfg(feature = "secrets")]

use base64_ng::{
    STRICT_STANDARD_PADDED,
    secret::{SecretArrayEncoder, SecretEncodeError, SecretEncoder, SecretInput},
};

#[test]
fn public_incremental_encoder_returns_only_secret_storage() {
    let mut encoder = SecretArrayEncoder::<32>::new(&STRICT_STANDARD_PADDED, 6).unwrap();
    encoder.update(&SecretInput::new(b"sec")).unwrap();
    encoder.update(&SecretInput::new(b"ret")).unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");
}

#[test]
fn public_borrowed_encoder_wipes_after_rejected_length() {
    let mut output = [0xa5; 16];
    {
        let mut encoder = SecretEncoder::new(&STRICT_STANDARD_PADDED, 3, &mut output).unwrap();
        assert!(matches!(
            encoder.update(&SecretInput::new(b"four")),
            Err(SecretEncodeError::InputTooLarge { .. })
        ));
    }
    assert_eq!(output, [0; 16]);
}

#[test]
fn public_one_shot_methods_require_classified_input() {
    let input = SecretInput::new(b"key");
    let encoded = STRICT_STANDARD_PADDED
        .encode_secret_array::<16>(&input)
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"a2V5");

    let mut output = [0; 16];
    let borrowed = STRICT_STANDARD_PADDED
        .encode_secret_into(&input, &mut output)
        .unwrap();
    assert_eq!(borrowed.expose_secret().as_bytes(), b"a2V5");
}

#[cfg(feature = "alloc")]
#[test]
fn public_heap_encoder_returns_secret_vec() {
    let encoded = STRICT_STANDARD_PADDED
        .encode_secret_vec(&SecretInput::new(b"key"))
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"a2V5");
}
