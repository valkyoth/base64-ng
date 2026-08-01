#![cfg(feature = "secrets")]

extern crate std;

use std::{format, vec::Vec};

use super::{
    Base64, Codec, CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
    STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, TrailingBits,
    ValidatedAlphabet,
    rfc4648_oracle::{self, Profile},
    secret::{
        MAX_SECRET_STACK_ENCODED, SecretArrayEncoder, SecretEncodeError, SecretEncoder, SecretInput,
    },
    secret_encoder::{map_value_for_test, require_disjoint_ranges_for_test},
};

fn encode_array<const CAP: usize, S: Codec>(
    codec: &Base64<S>,
    chunks: &[&[u8]],
    maximum_input_len: usize,
) -> Result<Vec<u8>, SecretEncodeError> {
    let mut encoder = SecretArrayEncoder::<CAP>::new(codec, maximum_input_len)?;
    for chunk in chunks {
        let progress = encoder.update(&SecretInput::new(chunk))?;
        assert_eq!(progress.input_consumed(), chunk.len());
    }
    Ok(encoder.finish()?.declassify().as_bytes().to_vec())
}

#[test]
fn built_in_arithmetic_and_custom_scan_map_every_value() {
    const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let custom_table = *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let custom = CodecBuilder::new(ValidatedAlphabet::new(custom_table).unwrap())
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .trailing_bits(TrailingBits::RequireCanonical)
        .build()
        .unwrap();

    for value in 0u8..64 {
        let index = usize::from(value);
        assert_eq!(
            map_value_for_test(STRICT_STANDARD_PADDED.settings(), value),
            STANDARD[index]
        );
        assert_eq!(
            map_value_for_test(STRICT_URL_SAFE_PADDED.settings(), value),
            URL_SAFE[index]
        );
        assert_eq!(
            map_value_for_test(custom.settings(), value),
            custom_table[index]
        );
    }
}

#[test]
fn stack_encoder_accepts_the_documented_maximum_capacity() {
    let encoder =
        SecretArrayEncoder::<MAX_SECRET_STACK_ENCODED>::new(&STRICT_STANDARD_PADDED, 1_024)
            .unwrap();
    assert_eq!(
        encoder.state().maximum_encoded_len(),
        MAX_SECRET_STACK_ENCODED
    );
}

#[test]
fn secret_encoders_match_independent_oracle_for_profiles_and_chunks() {
    let mut input = [0u8; 97];
    let mut value = 19u8;
    for byte in &mut input {
        *byte = value;
        value = value.wrapping_add(73);
    }

    for len in 0..=input.len() {
        let bytes = &input[..len];
        let split = len / 2;
        let cases = [
            (
                &STRICT_STANDARD_PADDED as &dyn EncodeCase,
                Profile::StandardPadded,
            ),
            (&STRICT_STANDARD_UNPADDED, Profile::StandardUnpadded),
            (&STRICT_URL_SAFE_PADDED, Profile::UrlSafePadded),
            (&STRICT_URL_SAFE_UNPADDED, Profile::UrlSafeUnpadded),
        ];
        for (case, profile) in cases {
            let actual = case.encode(&[&bytes[..split], &bytes[split..]], len);
            assert_eq!(actual, rfc4648_oracle::encode(profile, bytes));
        }
    }
}

trait EncodeCase {
    fn encode(&self, chunks: &[&[u8]], maximum_input_len: usize) -> Vec<u8>;
}

impl<S: Codec> EncodeCase for Base64<S> {
    fn encode(&self, chunks: &[&[u8]], maximum_input_len: usize) -> Vec<u8> {
        encode_array::<132, _>(self, chunks, maximum_input_len).unwrap()
    }
}

#[test]
fn one_shot_array_and_borrowed_outputs_remain_secret_until_exposed() {
    let input = SecretInput::new(b"secret");
    let encoded = STRICT_STANDARD_PADDED
        .encode_secret_array::<16>(&input)
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");
    assert_eq!(
        format!("{encoded:?}"),
        "SecretArray { bytes: \"<redacted>\", len: 8, capacity: 16 }"
    );

    let mut output = [0xa5; 16];
    let guarded = STRICT_STANDARD_PADDED
        .encode_secret_into(&input, &mut output)
        .unwrap();
    assert_eq!(guarded.expose_secret().as_bytes(), b"c2VjcmV0");
    drop(guarded);
    assert_eq!(output, [0; 16]);
}

#[test]
fn public_bounds_fail_before_processing_and_failure_is_absorbing() {
    assert_eq!(
        SecretArrayEncoder::<7>::new(&STRICT_STANDARD_PADDED, 6).unwrap_err(),
        SecretEncodeError::OutputFull {
            required: 8,
            available: 7,
        }
    );

    let mut encoder = SecretArrayEncoder::<8>::new(&STRICT_STANDARD_PADDED, 3).unwrap();
    let error = encoder.update(&SecretInput::new(b"four")).unwrap_err();
    assert_eq!(
        error,
        SecretEncodeError::InputTooLarge {
            input_len: 4,
            maximum_input_len: 3,
        }
    );
    assert!(encoder.state().is_failed());
    assert_eq!(encoder.storage_for_test(), &[0; 8]);
    assert_eq!(
        encoder.update(&SecretInput::new(b"")),
        Err(SecretEncodeError::Failed)
    );
}

#[test]
fn borrowed_capacity_preflight_is_non_destructive() {
    let mut output = [0xa5; 7];
    assert!(matches!(
        SecretEncoder::new(&STRICT_STANDARD_PADDED, 6, &mut output),
        Err(SecretEncodeError::OutputFull {
            required: 8,
            available: 7,
        })
    ));
    assert_eq!(output, [0xa5; 7]);
}

#[test]
fn custom_scan_work_is_fixed_per_emitted_symbol() {
    let custom = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        )
        .unwrap(),
    )
    .encode_padding(EncodePadding::Padded)
    .decode_padding(DecodePadding::RequireCanonical)
    .trailing_bits(TrailingBits::RequireCanonical)
    .build()
    .unwrap();
    let mut custom_encoder = SecretArrayEncoder::<16>::new(&custom, 6).unwrap();
    custom_encoder.update(&SecretInput::new(b"secret")).unwrap();
    assert_eq!(custom_encoder.state().mapping_work_for_test(), 8 * 64);

    let mut builtin = SecretArrayEncoder::<16>::new(&STRICT_STANDARD_PADDED, 6).unwrap();
    builtin.update(&SecretInput::new(b"secret")).unwrap();
    assert_eq!(builtin.state().mapping_work_for_test(), 8);
}

#[test]
fn successful_output_tail_is_zero_and_only_prefix_declassifies() {
    let encoded = STRICT_STANDARD_PADDED
        .encode_secret_array::<32>(&SecretInput::new(b"key"))
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"a2V5");
    assert!(
        encoded.backing_for_test()[4..]
            .iter()
            .all(|byte| *byte == 0)
    );
    assert_eq!(encoded.declassify().as_bytes(), b"a2V5");
}

#[test]
fn range_arithmetic_rejects_overlap_and_overflow() {
    assert_eq!(
        require_disjoint_ranges_for_test(100, 8, 107, 4),
        Err(SecretEncodeError::OverlappingBuffers)
    );
    assert_eq!(require_disjoint_ranges_for_test(100, 8, 108, 4), Ok(()));
    assert_eq!(
        require_disjoint_ranges_for_test(usize::MAX, 2, 0, 0),
        Err(SecretEncodeError::AddressRangeOverflow)
    );
}

#[cfg(feature = "std")]
#[test]
fn borrowed_encoder_wipes_on_unwind() {
    let mut output = [0xa5; 32];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut encoder = SecretEncoder::new(&STRICT_STANDARD_PADDED, 16, &mut output).unwrap();
        encoder.update(&SecretInput::new(b"secret")).unwrap();
        panic!("reviewed secret encoder cleanup test");
    }));
    assert!(result.is_err());
    assert_eq!(output, [0; 32]);
}

#[cfg(feature = "alloc")]
#[test]
fn vector_encoder_preallocates_and_returns_wiping_storage() {
    let encoded = STRICT_URL_SAFE_UNPADDED
        .encode_secret_vec(&SecretInput::new(&[0xfb, 0xff]))
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"-_8");
    assert!(encoded.capacity() >= 3);
}
