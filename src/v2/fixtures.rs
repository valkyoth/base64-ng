use super::{
    ordinary,
    rfc4648_oracle::{self as oracle, DecodeFailure, ErrorClass, Profile},
};
use crate::{DecodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

const RFC_VECTORS: &[(&[u8], &[u8])] = &[
    (b"", b""),
    (b"f", b"Zg=="),
    (b"fo", b"Zm8="),
    (b"foo", b"Zm9v"),
    (b"foob", b"Zm9vYg=="),
    (b"fooba", b"Zm9vYmE="),
    (b"foobar", b"Zm9vYmFy"),
];

#[test]
fn oracle_self_tests_against_rfc4648_vectors() {
    for &(plain, encoded) in RFC_VECTORS {
        assert_eq!(oracle::encode(Profile::StandardPadded, plain), encoded);
        assert_eq!(
            oracle::decode(Profile::StandardPadded, encoded),
            Ok(plain.to_vec())
        );
    }
}

#[test]
fn oracle_is_strict_about_alphabet_padding_and_canonical_bits() {
    assert_eq!(
        oracle::decode(Profile::StandardPadded, b"Zm9v\n"),
        Err(DecodeFailure {
            class: ErrorClass::Length,
            offset: None,
        })
    );
    assert_eq!(
        oracle::decode(Profile::StandardPadded, b"Zh=="),
        Err(DecodeFailure {
            class: ErrorClass::Padding,
            offset: Some(1),
        })
    );
    assert_eq!(
        oracle::decode(Profile::UrlSafeUnpadded, b"AA/A"),
        Err(DecodeFailure {
            class: ErrorClass::Byte,
            offset: Some(2),
        })
    );
}

#[test]
fn legacy_and_v2_compatibility_boundary_match_independent_oracle() {
    let profiles = [
        Profile::StandardPadded,
        Profile::StandardUnpadded,
        Profile::UrlSafePadded,
        Profile::UrlSafeUnpadded,
    ];
    let mut input = [0u8; 97];

    for len in 0..=input.len() {
        for (index, byte) in input[..len].iter_mut().enumerate() {
            *byte = u8::try_from((index * 53 + len * 29) % 256).unwrap();
        }

        for profile in profiles {
            let expected = oracle::encode(profile, &input[..len]);
            let mut legacy = [0u8; 132];
            let legacy_len = legacy_encode(profile, &input[..len], &mut legacy);
            let mut emerging = [0u8; 132];
            let emerging_len = ordinary::encode(profile, &input[..len], &mut emerging).unwrap();

            assert_eq!(&legacy[..legacy_len], expected);
            assert_eq!(&emerging[..emerging_len], expected);

            let decoded = oracle::decode(profile, &expected).unwrap();
            let mut legacy_plain = [0u8; 97];
            let legacy_plain_len = legacy_decode(profile, &expected, &mut legacy_plain).unwrap();
            let mut emerging_plain = [0u8; 97];
            let emerging_plain_len =
                ordinary::decode(profile, &expected, &mut emerging_plain).unwrap();

            assert_eq!(&legacy_plain[..legacy_plain_len], decoded);
            assert_eq!(&emerging_plain[..emerging_plain_len], decoded);
        }
    }
}

#[test]
fn malformed_diagnostics_match_where_compatibility_is_intentional() {
    for (profile, input) in [
        (Profile::StandardPadded, &b"Zm!v"[..]),
        (Profile::StandardPadded, b"Zh=="),
        (Profile::StandardPadded, b"Zg="),
        (Profile::StandardUnpadded, b"Z"),
        (Profile::StandardUnpadded, b"Zh"),
        (Profile::UrlSafePadded, b"AA/A"),
        (Profile::UrlSafeUnpadded, b"AA+A"),
    ] {
        let expected = oracle::decode(profile, input).unwrap_err();
        let mut legacy = [0x55; 16];
        let legacy_error = legacy_decode(profile, input, &mut legacy).unwrap_err();
        let mut emerging = [0xaa; 16];
        let emerging_error = ordinary::decode(profile, input, &mut emerging).unwrap_err();

        assert_eq!(normalized(legacy_error), expected);
        assert_eq!(normalized(emerging_error), expected);
    }
}

fn legacy_encode(profile: Profile, input: &[u8], output: &mut [u8]) -> usize {
    match profile {
        Profile::StandardPadded => STANDARD.encode_slice(input, output),
        Profile::StandardUnpadded => STANDARD_NO_PAD.encode_slice(input, output),
        Profile::UrlSafePadded => URL_SAFE.encode_slice(input, output),
        Profile::UrlSafeUnpadded => URL_SAFE_NO_PAD.encode_slice(input, output),
    }
    .unwrap()
}

fn legacy_decode(profile: Profile, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    match profile {
        Profile::StandardPadded => STANDARD.decode_slice(input, output),
        Profile::StandardUnpadded => STANDARD_NO_PAD.decode_slice(input, output),
        Profile::UrlSafePadded => URL_SAFE.decode_slice(input, output),
        Profile::UrlSafeUnpadded => URL_SAFE_NO_PAD.decode_slice(input, output),
    }
}

fn normalized(error: DecodeError) -> DecodeFailure {
    match error {
        DecodeError::InvalidLength => DecodeFailure {
            class: ErrorClass::Length,
            offset: None,
        },
        DecodeError::InvalidByte { index, .. } => DecodeFailure {
            class: ErrorClass::Byte,
            offset: Some(index),
        },
        DecodeError::InvalidPadding { index } => DecodeFailure {
            class: ErrorClass::Padding,
            offset: Some(index),
        },
        other => panic!("unexpected compatibility error: {other:?}"),
    }
}
