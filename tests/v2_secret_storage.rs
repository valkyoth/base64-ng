#![cfg(feature = "secrets")]

#[cfg(feature = "alloc")]
use base64_ng::secret::SecretVec;

#[cfg(feature = "alloc")]
fn assert_initialized_capacity_is_zero(mut bytes: Vec<u8>) {
    let visible = bytes.len();
    let capacity = bytes.capacity();
    // SAFETY: `SecretVec` initializes spare capacity with volatile zero writes
    // before this vector is returned by explicit declassification.
    unsafe { bytes.set_len(capacity) };
    assert!(bytes[visible..].iter().all(|byte| *byte == 0));
    bytes.truncate(visible);
}

#[cfg(feature = "alloc")]
#[test]
fn secret_vec_wipes_spare_capacity_before_explicit_declassification() {
    let mut source = Vec::with_capacity(32);
    source.extend_from_slice(b"secret");
    let secret = SecretVec::from_vec(source);
    assert_eq!(secret.expose_secret().as_bytes(), b"secret");
    assert_eq!(
        format!("{secret:?}"),
        "SecretVec { bytes: \"<redacted>\", len: 6 }"
    );
    assert_eq!(format!("{secret}"), "<redacted secret>");
    assert_initialized_capacity_is_zero(secret.declassify_into_unprotected_vec());
}

#[cfg(feature = "alloc")]
#[test]
fn secret_vec_clear_wipes_initialized_and_spare_capacity() {
    let mut source = Vec::with_capacity(32);
    source.extend_from_slice(b"secret");
    let mut secret = SecretVec::from_vec(source);
    secret.clear();
    let ordinary = secret.declassify_into_unprotected_vec();
    assert!(ordinary.is_empty());
    assert_initialized_capacity_is_zero(ordinary);
}
