extern crate std;

#[cfg(feature = "alloc")]
use core::cell::Cell;
use std::vec;

use super::{
    ordinary::OneShotError,
    rfc4648_oracle::{self as oracle, Profile},
    specifications::{
        Base64, CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
        STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, TrailingBits,
    },
};

const VECTORS: &[(&[u8], &[u8])] = &[
    (b"", b""),
    (b"f", b"Zg=="),
    (b"fo", b"Zm8="),
    (b"foo", b"Zm9v"),
    (b"foob", b"Zm9vYg=="),
    (b"fooba", b"Zm9vYmE="),
    (b"foobar", b"Zm9vYmFy"),
];

#[test]
fn strict_one_shot_surfaces_match_rfc4648_oracle() {
    for &(plain, padded) in VECTORS {
        for profile in profiles() {
            let expected = if matches!(profile, Profile::StandardPadded | Profile::UrlSafePadded) {
                padded.to_vec()
            } else {
                oracle::encode(profile, plain)
            };
            assert_codec_matches(codec(profile), profile, plain, &expected);
        }
    }

    let mut input = [0u8; 8];
    for len in 0..=input.len() {
        fill_pattern(&mut input[..len], len);
        for profile in profiles() {
            let encoded = oracle::encode(profile, &input[..len]);
            assert_codec_matches(codec(profile), profile, &input[..len], &encoded);
        }
    }
}

#[test]
fn every_bounded_error_leaves_the_complete_destination_unchanged() {
    let malformed: &[(Base64Codec, &[u8])] = &[
        (Base64Codec::StandardPadded, b"Zm!v"),
        (Base64Codec::StandardPadded, b"AB=="),
        (Base64Codec::StandardPadded, b"Zg="),
        (Base64Codec::StandardPadded, b"Zg==A"),
        (Base64Codec::StandardUnpadded, b"Zg="),
        (Base64Codec::StandardUnpadded, b"A"),
        (Base64Codec::UrlSafePadded, b"AA+A"),
        (Base64Codec::UrlSafeUnpadded, b"AA/A"),
    ];

    for &(selected, input) in malformed {
        let mut destination = [0x5a; 16];
        let before = destination;
        assert!(selected.decode(input, &mut destination).is_err());
        assert_eq!(destination, before);
    }

    for available in 0..8 {
        let mut encoded = [0x5a; 12];
        let before = encoded;
        let error = STRICT_STANDARD_PADDED
            .encode_into(b"foobar", &mut encoded[..available])
            .unwrap_err();
        assert_eq!(
            error,
            OneShotError::OutputTooSmall {
                required: 8,
                available,
            }
        );
        assert_eq!(encoded, before);

        let mut decoded = [0x5a; 12];
        let before = decoded;
        let error = STRICT_STANDARD_PADDED
            .decode_into(b"Zm9vYmFy", &mut decoded[..available.min(5)])
            .unwrap_err();
        assert!(matches!(error, OneShotError::OutputTooSmall { .. }));
        assert_eq!(decoded, before);
    }
}

#[test]
fn every_single_quantum_byte_mutation_matches_the_independent_oracle() {
    for profile in profiles() {
        for position in 0..4 {
            for value in 0u16..=u16::from(u8::MAX) {
                let mut input = *b"AAAA";
                input[position] = u8::try_from(value).unwrap();
                let expected = oracle::decode(profile, &input);
                let mut output = [0xa5; 3];
                let before = output;
                let actual = codec(profile).decode(&input, &mut output);
                if let Ok(expected) = expected {
                    let written = actual.unwrap();
                    assert_eq!(&output[..written], expected);
                } else {
                    assert!(actual.is_err());
                    assert_eq!(output, before);
                }
            }
        }
    }
}

#[test]
fn malformed_input_takes_precedence_over_destination_size() {
    let mut destination = [];
    assert!(matches!(
        STRICT_STANDARD_PADDED.decode_into(b"!!!!", &mut destination),
        Err(OneShotError::Input(_))
    ));
}

#[test]
fn runtime_custom_and_relaxed_policies_are_owned_and_explicit() {
    const CUSTOM: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let custom = CodecBuilder::from_table(CUSTOM)
        .unwrap()
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .build()
        .unwrap();
    let mut encoded = [0u8; 8];
    assert_eq!(custom.encode_into(b"custom", &mut encoded).unwrap(), 8);
    assert_eq!(&encoded, b"W1TxbE7r");
    let mut decoded = [0u8; 6];
    assert_eq!(custom.decode_into(&encoded, &mut decoded).unwrap(), 6);
    assert_eq!(&decoded, b"custom");

    let relaxed = CodecBuilder::from_table(
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .unwrap()
    .decode_padding(DecodePadding::Indifferent)
    .trailing_bits(TrailingBits::AllowNonCanonical)
    .build()
    .unwrap();
    for encoded in [b"Zg".as_slice(), b"Zg=", b"Zg=="] {
        let mut output = [0u8; 1];
        assert_eq!(relaxed.decode_into(encoded, &mut output).unwrap(), 1);
        assert_eq!(&output, b"f");
    }
    let mut output = [0u8; 1];
    assert_eq!(relaxed.decode_into(b"Zh==", &mut output).unwrap(), 1);
    assert_eq!(&output, b"f");
}

#[cfg(feature = "alloc")]
#[test]
fn allocation_limits_and_reservation_failure_precede_materialization() {
    assert_eq!(
        STRICT_STANDARD_PADDED
            .encode_to_string_with_limit(b"foo", 3)
            .unwrap_err(),
        OneShotError::AllocationLimitExceeded {
            required: 4,
            limit: 3,
        }
    );
    assert_eq!(
        STRICT_STANDARD_PADDED
            .decode_to_vec_with_limit(b"Zm9v", 2)
            .unwrap_err(),
        OneShotError::AllocationLimitExceeded {
            required: 3,
            limit: 2,
        }
    );

    let called = Cell::new(false);
    let error = STRICT_STANDARD_PADDED
        .decode_to_vec_with_injected_reserver(b"Zm9v", 3, |output, required| {
            called.set(true);
            assert!(output.is_empty());
            assert_eq!(required, 3);
            Err(OneShotError::AllocationFailed {
                requested: required,
            })
        })
        .unwrap_err();
    assert!(called.get());
    assert_eq!(error, OneShotError::AllocationFailed { requested: 3 });

    let encode_called = Cell::new(false);
    let error = STRICT_STANDARD_PADDED
        .encode_to_string_with_injected_reserver(b"foo", 4, |output, required| {
            encode_called.set(true);
            assert!(output.is_empty());
            assert_eq!(required, 4);
            Err(OneShotError::AllocationFailed {
                requested: required,
            })
        })
        .unwrap_err();
    assert!(encode_called.get());
    assert_eq!(error, OneShotError::AllocationFailed { requested: 4 });

    let reserve_called = Cell::new(false);
    let error = STRICT_STANDARD_PADDED
        .decode_to_vec_with_injected_reserver(b"!!!!", usize::MAX, |_, _| {
            reserve_called.set(true);
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, OneShotError::Input(_)));
    assert!(!reserve_called.get());
}

#[test]
fn checked_lengths_cover_overflow_and_exact_boundaries() {
    assert_eq!(STRICT_STANDARD_PADDED.encoded_len(0), Ok(0));
    assert_eq!(STRICT_STANDARD_PADDED.encoded_len(1), Ok(4));
    assert_eq!(STRICT_STANDARD_UNPADDED.encoded_len(1), Ok(2));
    assert_eq!(
        STRICT_STANDARD_PADDED.encoded_len(usize::MAX),
        Err(OneShotError::LengthOverflow)
    );
    assert_eq!(STRICT_STANDARD_PADDED.decoded_len(b"Zm8="), Ok(2));
    assert_eq!(STRICT_STANDARD_UNPADDED.decoded_len(b"Zm8"), Ok(2));
}

#[cfg(feature = "alloc")]
#[test]
fn allocating_results_have_exact_lengths_and_no_spare_visible_bytes() {
    let encoded = STRICT_URL_SAFE_UNPADDED
        .encode_to_string(b"\xfb\xff")
        .unwrap();
    assert_eq!(encoded, "-_8");
    let decoded = STRICT_URL_SAFE_UNPADDED
        .decode_to_vec(encoded.as_bytes())
        .unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

fn assert_codec_matches(codec: Base64Codec, profile: Profile, plain: &[u8], encoded: &[u8]) {
    let mut encoded_output = vec![0xa5; encoded.len() + 3];
    let encoded_before_tail = encoded_output[encoded.len()..].to_vec();
    let written = codec.encode(plain, &mut encoded_output).unwrap();
    assert_eq!(written, encoded.len());
    assert_eq!(&encoded_output[..written], encoded);
    assert_eq!(&encoded_output[written..], encoded_before_tail);

    let expected = oracle::decode(profile, encoded).unwrap();
    let mut decoded_output = vec![0xa5; expected.len() + 3];
    let decoded_before_tail = decoded_output[expected.len()..].to_vec();
    let written = codec.decode(encoded, &mut decoded_output).unwrap();
    assert_eq!(&decoded_output[..written], expected);
    assert_eq!(&decoded_output[written..], decoded_before_tail);
}

#[derive(Clone, Copy)]
enum Base64Codec {
    StandardPadded,
    StandardUnpadded,
    UrlSafePadded,
    UrlSafeUnpadded,
}

impl Base64Codec {
    fn encode(self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        dispatch(self).encode(input, output)
    }

    fn decode(self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        dispatch(self).decode(input, output)
    }
}

enum CodecDispatch {
    Padded(Base64<super::specifications::StrictStandardPadded>),
    Unpadded(Base64<super::specifications::StrictStandardUnpadded>),
    UrlPadded(Base64<super::specifications::StrictUrlSafePadded>),
    UrlUnpadded(Base64<super::specifications::StrictUrlSafeUnpadded>),
}

impl CodecDispatch {
    fn encode(&self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        match self {
            Self::Padded(codec) => codec.encode_into(input, output),
            Self::Unpadded(codec) => codec.encode_into(input, output),
            Self::UrlPadded(codec) => codec.encode_into(input, output),
            Self::UrlUnpadded(codec) => codec.encode_into(input, output),
        }
    }

    fn decode(&self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        match self {
            Self::Padded(codec) => codec.decode_into(input, output),
            Self::Unpadded(codec) => codec.decode_into(input, output),
            Self::UrlPadded(codec) => codec.decode_into(input, output),
            Self::UrlUnpadded(codec) => codec.decode_into(input, output),
        }
    }
}

fn dispatch(codec: Base64Codec) -> CodecDispatch {
    match codec {
        Base64Codec::StandardPadded => CodecDispatch::Padded(STRICT_STANDARD_PADDED),
        Base64Codec::StandardUnpadded => CodecDispatch::Unpadded(STRICT_STANDARD_UNPADDED),
        Base64Codec::UrlSafePadded => CodecDispatch::UrlPadded(STRICT_URL_SAFE_PADDED),
        Base64Codec::UrlSafeUnpadded => CodecDispatch::UrlUnpadded(STRICT_URL_SAFE_UNPADDED),
    }
}

fn codec(profile: Profile) -> Base64Codec {
    match profile {
        Profile::StandardPadded => Base64Codec::StandardPadded,
        Profile::StandardUnpadded => Base64Codec::StandardUnpadded,
        Profile::UrlSafePadded => Base64Codec::UrlSafePadded,
        Profile::UrlSafeUnpadded => Base64Codec::UrlSafeUnpadded,
    }
}

fn profiles() -> [Profile; 4] {
    [
        Profile::StandardPadded,
        Profile::StandardUnpadded,
        Profile::UrlSafePadded,
        Profile::UrlSafeUnpadded,
    ]
}

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::try_from((index * 73 + seed * 29) % 256).unwrap();
    }
}
