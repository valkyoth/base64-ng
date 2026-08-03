#![allow(missing_docs)]

#[cfg(feature = "alloc")]
use base64_ng::secret::SecretVec;
use base64_ng::secret::{SecretArray, SecretInput, SecretOutput};
use base64_ng_subtle::{SubtleSecretEq, subtle_ct_eq_fixed_width, subtle_ct_eq_public_len};

fn assert_public_length_cases(value: &impl SubtleSecretEq, expected: &[u8]) {
    assert!(bool::from(value.subtle_ct_eq_public_len(expected)));

    let mut unequal = expected.to_vec();
    if let Some(first) = unequal.first_mut() {
        *first ^= 1;
    }
    assert!(!bool::from(value.subtle_ct_eq_public_len(&unequal)));

    let shorter = &expected[..expected.len().saturating_sub(1)];
    assert!(!bool::from(value.subtle_ct_eq_public_len(shorter)));
}

#[test]
fn compares_every_no_alloc_secret_owner_and_view() {
    let input = SecretInput::new(b"secret");
    assert_public_length_cases(&input, b"secret");
    assert_public_length_cases(&input.expose_secret(), b"secret");

    let fixed = SecretArray::from_array(*b"secret00", 6).unwrap();
    assert_public_length_cases(&fixed, b"secret");

    let mut backing = *b"secret00";
    let mut output = SecretOutput::from_initialized(&mut backing, 6).unwrap();
    assert_public_length_cases(&output, b"secret");
    assert_public_length_cases(&output.expose_secret(), b"secret");
    assert_public_length_cases(&output.expose_secret_mut(), b"secret");
}

#[cfg(feature = "alloc")]
#[test]
fn compares_heap_secret_owner() {
    let secret = SecretVec::from_slice(b"secret");
    assert_public_length_cases(&secret, b"secret");
}

#[test]
fn raw_public_length_and_fixed_width_helpers_are_explicit() {
    assert!(bool::from(subtle_ct_eq_public_len(b"token", b"token")));
    assert!(!bool::from(subtle_ct_eq_public_len(b"token", b"Token")));
    assert!(!bool::from(subtle_ct_eq_public_len(b"token", b"token!")));

    assert!(bool::from(subtle_ct_eq_fixed_width(b"token", b"token")));
    assert!(!bool::from(subtle_ct_eq_fixed_width(b"token", b"Token")));
}
