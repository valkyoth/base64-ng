#![cfg(feature = "secrets")]

use base64_ng::{
    STRICT_STANDARD_PADDED,
    secret::{SecretArrayFrame, SecretDecodeError, SecretFrame, SecretInput},
};

#[test]
fn public_stack_frame_releases_only_after_successful_finish() {
    let mut frame = SecretArrayFrame::<32>::new(&STRICT_STANDARD_PADDED).unwrap();
    let first = SecretInput::new(b"c2Vj");
    let second = SecretInput::new(b"cmV0");
    assert_eq!(frame.update(&first).unwrap().output_produced(), 0);
    assert_eq!(frame.update(&second).unwrap().output_produced(), 0);
    let secret = frame.finish().unwrap();
    assert_eq!(secret.expose_secret().as_bytes(), b"secret");
}

#[test]
fn public_borrowed_frame_wipes_rejected_staging_and_output() {
    let mut staging = [0xa5; 16];
    let mut output = [0xa5; 16];
    let rejected = {
        let mut frame =
            SecretFrame::new(&STRICT_STANDARD_PADDED, 16, &mut staging, &mut output).unwrap();
        frame.update(&SecretInput::new(b"c2Vj!mV0")).unwrap();
        matches!(frame.finish(), Err(SecretDecodeError::InvalidInput))
    };
    assert!(rejected);
    assert_eq!(staging, [0; 16]);
    assert_eq!(output, [0; 16]);
}

#[test]
fn public_macro_constructs_a_bounded_frame() {
    let frame = base64_ng::secret_array_frame!(STRICT_STANDARD_PADDED, 32).unwrap();
    assert_eq!(frame.state().maximum_decoded_len(), 32);
    assert_eq!(frame.state().maximum_encoded_len(), 44);
}

#[cfg(feature = "alloc")]
#[test]
fn public_vector_frame_is_bounded_before_updates() {
    use base64_ng::secret::SecretVecFrame;

    let mut frame = SecretVecFrame::new(&STRICT_STANDARD_PADDED, 16).unwrap();
    frame.update(&SecretInput::new(b"a2V5")).unwrap();
    let secret = frame.finish().unwrap();
    assert_eq!(secret.expose_secret().as_bytes(), b"key");
    assert!(secret.capacity() >= 16);
}
