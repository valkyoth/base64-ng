extern crate std;

use super::*;
use crate::{DecodeValidation, decode_validation::observation};
use std::{cell::Cell, string::String, vec, vec::Vec};

fn one_fast_validation(action: impl FnOnce()) {
    let before = observation::fast_calls();
    action();
    assert_eq!(observation::fast_calls() - before, 1);
}

#[test]
fn forwarding_owners_bounded_and_append_validate_once_per_call() {
    let codec = STRICT_STANDARD_PADDED;
    for len in [4, 508, 512, 516, 4096, 4100] {
        let input = vec![b'A'; len];
        let expected = vec![0; len / 4 * 3];
        one_fast_validation(|| {
            assert_eq!(codec.decode_to_vec(&input).unwrap(), expected);
        });
        one_fast_validation(|| {
            let bounded = codec.decode_bounded::<3080>(&input).unwrap();
            assert_eq!(bounded.as_bytes(), expected);
            let (bytes, written) = bounded.into_parts();
            assert!(bytes[written..].iter().all(|&byte| byte == 0));
        });
        one_fast_validation(|| {
            let mut output = Vec::from(b"prefix");
            assert_eq!(codec.decode_append(&input, &mut output), Ok(expected.len()));
            assert_eq!(&output[..6], b"prefix");
            assert_eq!(&output[6..], expected);
        });
        let text = String::from_utf8(input).unwrap();
        let mut owner = None;
        one_fast_validation(|| {
            owner = Some(Base64String::parse(codec, &text).unwrap());
        });
        let owner = owner.unwrap();
        one_fast_validation(|| assert_eq!(owner.decode().unwrap(), expected));
        one_fast_validation(|| {
            assert_eq!(owner.decode_with_limit(expected.len()).unwrap(), expected);
        });
        one_fast_validation(|| {
            assert_eq!(
                crate::STANDARD.decode_vec(text.as_bytes()).unwrap(),
                expected
            );
        });
    }
}

#[test]
fn forwarding_append_retains_proof_across_reservation_and_rolls_back() {
    for len in [4, 512, 4096] {
        let input = vec![b'A'; len];
        for fail_reserve in [false, true] {
            let mut output = Vec::from(b"prefix");
            let before = observation::fast_calls();
            let wrote = Cell::new(false);
            let result = STRICT_STANDARD_PADDED.decode_append_with_hooks(
                &input,
                &mut output,
                |output, required| {
                    assert_eq!(observation::fast_calls(), before + 1);
                    assert_eq!(output, b"prefix");
                    assert_eq!(required, len / 4 * 3);
                    if fail_reserve {
                        Err(OneShotError::AllocationFailed {
                            requested: required,
                        })
                    } else {
                        output.try_reserve_exact(required).unwrap();
                        Ok(())
                    }
                },
                |output| {
                    wrote.set(true);
                    assert_eq!(&output[..6], b"prefix");
                    assert!(output[6..].iter().all(|&byte| byte == 0));
                    Err(OneShotError::Backend(BackendFault::ImpossibleState))
                },
            );
            assert_eq!(observation::fast_calls(), before + 1);
            assert_eq!(wrote.get(), !fail_reserve);
            assert!(result.is_err());
            assert_eq!(output, b"prefix");
        }
    }
}

#[test]
fn forwarding_late_malformed_input_never_reserves_or_mutates() {
    for len in [4, 512, 4096] {
        let mut input = vec![b'A'; len];
        input[len - 1] = b'!';
        let expected = STRICT_STANDARD_PADDED
            .decoded_len_with_validation(&input, DecodeValidation::ScalarReference)
            .unwrap_err();
        let mut destination = Vec::from(b"prefix");
        assert_eq!(
            STRICT_STANDARD_PADDED.decode_append_with_hooks(
                &input,
                &mut destination,
                |_, _| panic!("invalid input must not reserve"),
                |_| panic!("invalid input must not write"),
            ),
            Err(expected)
        );
        assert_eq!(destination, b"prefix");
        assert_eq!(
            STRICT_STANDARD_PADDED.decode_to_vec_with_injected_reserver(&input, 0, |_, _| panic!(
                "invalid input must precede limit and reservation"
            ),),
            Err(expected)
        );
        assert_eq!(
            STRICT_STANDARD_PADDED.decode_bounded::<0>(&input),
            Err(expected)
        );
        assert_eq!(
            Base64String::parse_with_injected_reserver(
                STRICT_STANDARD_PADDED,
                core::str::from_utf8(&input).unwrap(),
                |_, _| panic!("invalid owner must not reserve"),
            ),
            Err(expected)
        );
    }
}

#[test]
fn forwarding_string_uses_encode_padding_and_preserves_builder_guards() {
    let table = *STRICT_STANDARD_PADDED.settings().alphabet().as_array();
    for (encode, decode, error) in [
        (
            EncodePadding::Padded,
            DecodePadding::Forbid,
            CodecBuilderError::EncodedPaddingRejected,
        ),
        (
            EncodePadding::Unpadded,
            DecodePadding::RequireCanonical,
            CodecBuilderError::EncodedPaddingRequired,
        ),
    ] {
        assert_eq!(
            CodecBuilder::from_table(table)
                .unwrap()
                .encode_padding(encode)
                .decode_padding(decode)
                .build(),
            Err(error)
        );
    }
    for encode in [EncodePadding::Padded, EncodePadding::Unpadded] {
        let codec = CodecBuilder::from_table(table)
            .unwrap()
            .encode_padding(encode)
            .decode_padding(DecodePadding::Indifferent)
            .build()
            .unwrap();
        for input in [b"f".as_slice(), b"fo", b"foo"] {
            let owner = Base64String::encode(codec, input).unwrap();
            assert_eq!(
                owner.as_bytes().ends_with(b"="),
                encode == EncodePadding::Padded && input.len() % 3 != 0
            );
            assert_eq!(owner.decode().unwrap(), input);
            assert_eq!(owner.clone().decode().unwrap(), input);
            assert_eq!(Base64String::parse(codec, owner.as_str()).unwrap(), owner);
            assert_eq!(
                Base64String::from_string(codec, owner.clone().into_string()).unwrap(),
                owner
            );
            assert!(matches!(
                owner.decode_with_limit(0),
                Err(OneShotError::AllocationLimitExceeded { .. })
            ));
        }
    }
}

#[test]
fn forwarding_owners_keep_custom_and_relaxed_acceptance() {
    let mut custom = *STRICT_STANDARD_PADDED.settings().alphabet().as_array();
    custom.rotate_left(7);
    let codec = CodecBuilder::from_table(custom).unwrap().build().unwrap();
    let owner = Base64String::encode(codec, b"ordinary owned bytes").unwrap();
    let before = observation::calls();
    assert_eq!(owner.decode().unwrap(), b"ordinary owned bytes");
    assert_eq!(observation::calls(), before + 1);
    let relaxed = CodecBuilder::new(*STRICT_STANDARD_PADDED.settings().alphabet())
        .decode_padding(DecodePadding::Indifferent)
        .trailing_bits(TrailingBits::AllowNonCanonical)
        .build()
        .unwrap();
    for text in ["Zh", "Zh=", "Zh=="] {
        let owner = Base64String::parse(relaxed, text).unwrap();
        assert_eq!(owner.decode().unwrap(), b"f");
        let mut output = Vec::from(b"prefix");
        let before = observation::calls();
        assert_eq!(relaxed.decode_append(text.as_bytes(), &mut output), Ok(1));
        assert_eq!(observation::calls(), before + 1);
        assert_eq!(output, b"prefixf");
    }
}

#[test]
fn forwarding_historical_owned_preserves_reference_results() {
    fn check<A: crate::Alphabet, const PAD: bool>(engine: crate::Engine<A, PAD>) {
        for len in [0, 1, 2, 3, 383, 384, 385, 3071, 3072, 3073] {
            let plain: Vec<_> = (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect();
            let encoded = engine.encode_vec(&plain).unwrap();
            assert_eq!(engine.decode_vec(&encoded).unwrap(), plain);
            for position in [0, encoded.len() / 2, encoded.len().saturating_sub(1)] {
                if encoded.is_empty() {
                    continue;
                }
                for byte in [b'!', b'=', b' ', 0xff] {
                    let mut malformed = encoded.clone();
                    malformed[position] = byte;
                    assert_eq!(
                        engine.decode_vec_with_validation(&malformed, DecodeValidation::Auto),
                        engine.decode_vec_with_validation(
                            &malformed,
                            DecodeValidation::ScalarReference
                        ),
                    );
                }
            }
        }
        for input in [b"AB==".as_slice(), b"AAB=", b"Zg=", b"Zg==A", b"A"] {
            assert_eq!(
                engine.decode_vec_with_validation(input, DecodeValidation::Auto),
                engine.decode_vec_with_validation(input, DecodeValidation::ScalarReference),
            );
        }
    }
    check(crate::STANDARD);
    check(crate::STANDARD_NO_PAD);
    check(crate::URL_SAFE);
    check(crate::URL_SAFE_NO_PAD);
    check(crate::Engine::<crate::Bcrypt, false>::new());
}
