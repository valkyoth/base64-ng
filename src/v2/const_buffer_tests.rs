extern crate std;

use core::mem::{align_of, needs_drop, size_of};
use std::format;

use super::{
    bounded::{DecodedArray, EncodedArray, SecretArray},
    const_transforms::ConstTransformError,
    contracts::{Failure, InputError, OperationError},
    ordinary::OneShotError,
    rfc4648_oracle::{self as oracle, Profile},
    specifications::{
        Base64, Codec, CodecBuilder, CodecSettings, DecodePadding, EncodePadding, RuntimeSpec,
        STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
        STRICT_URL_SAFE_UNPADDED, TrailingBits,
    },
};

const CUSTOM_TABLE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const CUSTOM: Base64<RuntimeSpec> = match CodecBuilder::from_table(CUSTOM_TABLE) {
    Ok(builder) => match builder
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .build()
    {
        Ok(codec) => codec,
        Err(_) => panic!("valid custom codec rejected"),
    },
    Err(_) => panic!("valid custom alphabet rejected"),
};

const STANDARD_PADDED: [u8; 8] = match STRICT_STANDARD_PADDED.encode_array(b"hello") {
    Ok(output) => output,
    Err(_) => panic!("valid const encode rejected"),
};
const STANDARD_UNPADDED: [u8; 7] = match STRICT_STANDARD_UNPADDED.encode_array(b"hello") {
    Ok(output) => output,
    Err(_) => panic!("valid const encode rejected"),
};
const URL_SAFE_PADDED: [u8; 4] = match STRICT_URL_SAFE_PADDED.encode_array(b"\xfb\xff") {
    Ok(output) => output,
    Err(_) => panic!("valid const encode rejected"),
};
const URL_SAFE_UNPADDED: [u8; 3] = match STRICT_URL_SAFE_UNPADDED.encode_array(b"\xfb\xff") {
    Ok(output) => output,
    Err(_) => panic!("valid const encode rejected"),
};
const CUSTOM_ENCODED: [u8; 8] = match CUSTOM.encode_array(b"custom") {
    Ok(output) => output,
    Err(_) => panic!("valid custom const encode rejected"),
};
const CUSTOM_DECODED: [u8; 6] = match CUSTOM.decode_array(&CUSTOM_ENCODED) {
    Ok(output) => output,
    Err(_) => panic!("valid custom const decode rejected"),
};
const RELAXED: Base64<RuntimeSpec> = match CodecBuilder::from_table(
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
) {
    Ok(builder) => match builder
        .decode_padding(DecodePadding::Indifferent)
        .trailing_bits(TrailingBits::AllowNonCanonical)
        .build()
    {
        Ok(codec) => codec,
        Err(_) => panic!("valid relaxed codec rejected"),
    },
    Err(_) => panic!("valid relaxed alphabet rejected"),
};
const RELAXED_DECODED: [u8; 1] = match RELAXED.decode_array(b"Zh=") {
    Ok(output) => output,
    Err(_) => panic!("valid relaxed const decode rejected"),
};
const STANDARD_DECODED: [u8; 5] = match STRICT_STANDARD_PADDED.decode_array(&STANDARD_PADDED) {
    Ok(output) => output,
    Err(_) => panic!("valid const decode rejected"),
};
const MALFORMED: Result<[u8; 3], ConstTransformError> =
    STRICT_STANDARD_PADDED.decode_array(b"Zm!v");
const NONCANONICAL: Result<[u8; 1], ConstTransformError> =
    STRICT_STANDARD_PADDED.decode_array(b"Zh==");
const WRONG_ENCODE_SIZE: Result<[u8; 7], ConstTransformError> =
    STRICT_STANDARD_PADDED.encode_array(b"hello");
const WRONG_DECODE_SIZE: Result<[u8; 6], ConstTransformError> =
    STRICT_STANDARD_PADDED.decode_array(b"aGVsbG8=");

#[test]
fn built_in_and_custom_const_transforms_are_exact() {
    assert_eq!(STANDARD_PADDED, *b"aGVsbG8=");
    assert_eq!(STANDARD_UNPADDED, *b"aGVsbG8");
    assert_eq!(URL_SAFE_PADDED, *b"-_8=");
    assert_eq!(URL_SAFE_UNPADDED, *b"-_8");
    assert_eq!(STANDARD_DECODED, *b"hello");
    assert_eq!(CUSTOM_ENCODED, *b"W1TxbE7r");
    assert_eq!(CUSTOM_DECODED, *b"custom");
    assert_eq!(RELAXED_DECODED, *b"f");
}

#[test]
fn const_diagnostics_preserve_input_and_exact_size_classes() {
    assert!(matches!(
        MALFORMED,
        Err(ConstTransformError::Input(InputError::InvalidByte {
            index: 2,
            byte: b'!'
        }))
    ));
    assert!(matches!(
        NONCANONICAL,
        Err(ConstTransformError::Input(
            InputError::NonCanonicalTrailingBits { index: 1 }
        ))
    ));
    assert_eq!(
        WRONG_ENCODE_SIZE,
        Err(ConstTransformError::OutputLengthMismatch {
            required: 8,
            actual: 7,
        })
    );
    assert_eq!(
        WRONG_DECODE_SIZE,
        Err(ConstTransformError::OutputLengthMismatch {
            required: 5,
            actual: 6,
        })
    );
    assert!(matches!(
        STRICT_STANDARD_UNPADDED.const_settings().decoded_len(b"!!"),
        Err(ConstTransformError::Input(InputError::InvalidByte {
            index: 0,
            byte: b'!'
        }))
    ));
}

#[test]
fn malformed_padding_errors_match_const_one_shot_and_incremental_surfaces() {
    for (input, expected) in [
        (b"=AAA".as_slice(), InputError::InvalidPadding { index: 0 }),
        (b"A=AA".as_slice(), InputError::InvalidPadding { index: 1 }),
        (b"AA=A".as_slice(), InputError::InvalidPadding { index: 2 }),
        (
            b"AAAA=AAA".as_slice(),
            InputError::InvalidPadding { index: 4 },
        ),
        (
            b"AAAAA=AA".as_slice(),
            InputError::InvalidPadding { index: 5 },
        ),
    ] {
        assert_strict_decode_error_consistency(&STRICT_STANDARD_PADDED, input, expected);
        assert_strict_decode_error_consistency(&STRICT_STANDARD_UNPADDED, input, expected);
        assert_strict_decode_error_consistency(&STRICT_URL_SAFE_PADDED, input, expected);
        assert_strict_decode_error_consistency(&STRICT_URL_SAFE_UNPADDED, input, expected);
    }
}

#[test]
fn const_transforms_match_independent_oracle_at_boundaries() {
    const EMPTY_ENCODED: [u8; 0] = match STRICT_STANDARD_PADDED.encode_array(b"") {
        Ok(output) => output,
        Err(_) => panic!("empty const encode rejected"),
    };
    const EMPTY_DECODED: [u8; 0] = match STRICT_STANDARD_PADDED.decode_array(b"") {
        Ok(output) => output,
        Err(_) => panic!("empty const decode rejected"),
    };
    assert_eq!(EMPTY_ENCODED, []);
    assert_eq!(EMPTY_DECODED, []);

    for (profile, actual, plain) in [
        (
            Profile::StandardPadded,
            STANDARD_PADDED.as_slice(),
            b"hello".as_slice(),
        ),
        (
            Profile::StandardUnpadded,
            STANDARD_UNPADDED.as_slice(),
            b"hello".as_slice(),
        ),
        (
            Profile::UrlSafePadded,
            URL_SAFE_PADDED.as_slice(),
            b"\xfb\xff".as_slice(),
        ),
        (
            Profile::UrlSafeUnpadded,
            URL_SAFE_UNPADDED.as_slice(),
            b"\xfb\xff".as_slice(),
        ),
    ] {
        assert_eq!(actual, oracle::encode(profile, plain));
        assert_eq!(oracle::decode(profile, actual).unwrap(), plain);
    }
}

#[test]
fn const_decoder_matches_oracle_for_every_single_quantum_mutation() {
    for profile in [
        Profile::StandardPadded,
        Profile::StandardUnpadded,
        Profile::UrlSafePadded,
        Profile::UrlSafeUnpadded,
    ] {
        let settings = settings(profile);
        for position in 0..4 {
            for value in 0u16..=u16::from(u8::MAX) {
                let mut input = *b"AAAA";
                input[position] = u8::try_from(value).unwrap();
                let expected = oracle::decode(profile, &input);
                let measured = settings.decoded_len(&input);
                match expected {
                    Ok(expected) => {
                        assert_eq!(measured, Ok(expected.len()));
                        match expected.len() {
                            1 => assert_eq!(
                                settings.decode_array::<4, 1>(&input).unwrap().as_slice(),
                                expected
                            ),
                            2 => assert_eq!(
                                settings.decode_array::<4, 2>(&input).unwrap().as_slice(),
                                expected
                            ),
                            3 => assert_eq!(
                                settings.decode_array::<4, 3>(&input).unwrap().as_slice(),
                                expected
                            ),
                            _ => unreachable!(),
                        }
                    }
                    Err(_) => assert!(matches!(measured, Err(ConstTransformError::Input(_)))),
                }
            }
        }
    }
}

fn assert_strict_decode_error_consistency<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    expected: InputError,
) {
    assert_eq!(
        codec.settings().decoded_len(input),
        Err(ConstTransformError::Input(expected))
    );
    assert_eq!(codec.decoded_len(input), Err(OneShotError::Input(expected)));

    let mut decoder = codec.decoder();
    let mut output = [0xa5; 6];
    assert_eq!(
        decoder.update(input, &mut output),
        Err(OperationError::Failed(Failure::Input(expected)))
    );
}

#[test]
fn ordinary_bounded_arrays_keep_length_private_and_tail_invisible() {
    let encoded = STRICT_STANDARD_PADDED
        .encode_bounded::<12>(b"hello")
        .unwrap();
    assert_eq!(encoded.as_bytes(), b"aGVsbG8=");
    assert_eq!(encoded.len(), 8);
    assert_eq!(encoded.capacity(), 12);
    assert_eq!(encoded.remaining_capacity(), 4);
    let (encoded_bytes, encoded_len) = encoded.into_parts();
    assert_eq!(&encoded_bytes[..encoded_len], b"aGVsbG8=");
    assert_eq!(&encoded_bytes[encoded_len..], &[0; 4]);

    let decoded = STRICT_STANDARD_PADDED
        .decode_bounded::<8>(b"aGVsbG8=")
        .unwrap();
    assert_eq!(decoded.as_bytes(), b"hello");
    assert_eq!(decoded.len(), 5);
    assert_eq!(decoded.remaining_capacity(), 3);
    let (decoded_bytes, decoded_len) = decoded.into_parts();
    assert_eq!(&decoded_bytes[..decoded_len], b"hello");
    assert_eq!(&decoded_bytes[decoded_len..], &[0; 3]);
}

#[test]
fn bounded_arrays_cover_zero_exact_capacity_and_rejection() {
    let empty = STRICT_STANDARD_PADDED.encode_bounded::<0>(b"").unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.capacity(), 0);

    let exact = STRICT_STANDARD_PADDED.decode_bounded::<3>(b"Zm9v").unwrap();
    assert_eq!(exact.as_bytes(), b"foo");
    assert_eq!(exact.remaining_capacity(), 0);

    let error = STRICT_STANDARD_PADDED
        .encode_bounded::<3>(b"foo")
        .unwrap_err();
    assert!(matches!(
        error,
        super::ordinary::OneShotError::OutputTooSmall {
            required: 4,
            available: 3
        }
    ));

    let rejected = EncodedArray::<2>::from_array([0; 2], 3).unwrap_err();
    assert_eq!(rejected.length(), 3);
    assert_eq!(rejected.capacity(), 2);
}

#[test]
fn ordinary_and_secret_storage_have_distinct_cleanup_contracts() {
    fn assert_copy<T: Copy>() {}
    assert_copy::<EncodedArray<64>>();
    assert_copy::<DecodedArray<64>>();
    assert!(!needs_drop::<EncodedArray<64>>());
    assert!(!needs_drop::<DecodedArray<64>>());
    assert!(needs_drop::<SecretArray<64>>());

    let mut bytes = [0xa5; 8];
    bytes[..3].copy_from_slice(b"key");
    let mut secret = SecretArray::from_array(bytes, 3).unwrap();
    assert_eq!(secret.expose_secret(), b"key");
    assert_eq!(&secret.backing_for_test()[3..], &[0; 5]);
    assert_eq!(
        format!("{secret:?}"),
        "SecretArray { bytes: \"<redacted>\", len: 3, capacity: 8 }"
    );
    assert_eq!(format!("{secret}"), "<redacted secret array>");

    secret.clear();
    assert!(secret.is_empty());
    assert_eq!(secret.backing_for_test(), &[0; 8]);
}

#[test]
fn representative_stack_object_sizes_are_capacity_plus_length_metadata() {
    assert_eq!(align_of::<EncodedArray<64>>(), align_of::<usize>());
    assert_eq!(size_of::<EncodedArray<64>>(), 64 + size_of::<usize>());
    assert_eq!(size_of::<DecodedArray<256>>(), 256 + size_of::<usize>());
    assert_eq!(size_of::<SecretArray<1024>>(), 1024 + size_of::<usize>());
}

const fn settings(profile: Profile) -> CodecSettings {
    match profile {
        Profile::StandardPadded => STRICT_STANDARD_PADDED.const_settings(),
        Profile::StandardUnpadded => STRICT_STANDARD_UNPADDED.const_settings(),
        Profile::UrlSafePadded => STRICT_URL_SAFE_PADDED.const_settings(),
        Profile::UrlSafeUnpadded => STRICT_URL_SAFE_UNPADDED.const_settings(),
    }
}
