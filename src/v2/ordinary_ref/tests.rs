extern crate std;
use super::*;
use crate::decode_validation::observation;
use crate::{
    CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED as CODEC, TrailingBits,
};

#[test]
fn borrowed_view_reuses_grammar_but_reference_policy_revalidates() {
    let _ = crate::initialize_backends();
    // Keep this scan-count test below SIMD admission: checked vector output
    // deliberately performs additional per-chunk reference work.
    let input = [b'A'; 64];
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let before = observation::calls();
        let fast = observation::fast_calls();
        let view = Base64Ref::parse_with_validation(CODEC, &input, policy).unwrap();
        if policy == DecodeValidation::ScalarReference {
            assert_eq!(observation::calls(), before + 1);
            assert_eq!(observation::fast_calls(), fast);
        } else {
            assert_eq!(observation::fast_calls(), fast + 1);
        }
        let after = observation::calls();
        let fast_after = observation::fast_calls();
        for _ in 0..3 {
            let mut output = [0xa5; 56];
            assert_eq!(view.decode_into(&mut output), Ok(48));
            assert_eq!(&output[..48], &[0; 48]);
            assert_eq!(&output[48..], &[0xa5; 8]);
        }
        assert_eq!(observation::fast_calls(), fast_after);
        assert_eq!(
            observation::calls() - after,
            if policy == DecodeValidation::ScalarReference {
                3
            } else {
                0
            }
        );
    }
}

#[test]
fn borrowed_view_owns_exact_runtime_settings_and_reuses_reference_fallback() {
    let mut alphabet = *CODEC.settings().alphabet().as_array();
    alphabet.swap(0, 1);
    let view = {
        let codec = CodecBuilder::from_table(alphabet).unwrap().build().unwrap();
        Base64Ref::parse(codec, b"AAAA").unwrap()
    };
    assert_eq!(view.settings().alphabet().as_array(), &alphabet);
    let before = observation::calls();
    let mut output = [0xa5; 5];
    for _ in 0..3 {
        assert_eq!(view.decode_into(&mut output), Ok(3));
        assert_eq!(output, [4, 16, 65, 0xa5, 0xa5]);
    }
    assert_eq!(observation::calls(), before);
    let relaxed = CodecBuilder::new(*CODEC.settings().alphabet())
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Indifferent)
        .trailing_bits(TrailingBits::AllowNonCanonical)
        .build()
        .unwrap();
    for input in [b"Zh".as_slice(), b"Zh=", b"Zh=="] {
        let view = Base64Ref::parse(relaxed, input).unwrap();
        assert_eq!(view.decoded_len(), 1);
        let mut output = [0xa5; 3];
        assert_eq!(view.decode_into(&mut output), Ok(1));
        assert_eq!(output, [b'f', 0xa5, 0xa5]);
    }
}

#[cfg(feature = "alloc")]
#[test]
fn borrowed_view_allocates_after_revalidation_and_limit_checks() {
    let view = Base64Ref::parse_with_validation(CODEC, b"Zm9v", DecodeValidation::ScalarReference)
        .unwrap();
    let before = observation::calls();
    assert_eq!(
        view.decode_with_reserver(2, |_, _| panic!("limit must precede allocation")),
        Err(OneShotError::AllocationLimitExceeded {
            required: 3,
            limit: 2
        })
    );
    assert_eq!(observation::calls(), before + 1);
    assert_eq!(
        view.decode_with_reserver(3, |_, required| {
            assert_eq!(required, 3);
            assert_eq!(observation::calls(), before + 2);
            Err(OneShotError::AllocationFailed {
                requested: required,
            })
        }),
        Err(OneShotError::AllocationFailed { requested: 3 })
    );
    assert_eq!(view.decode_to_vec_with_limit(3).unwrap(), b"foo");
}

#[cfg(feature = "alloc")]
#[test]
fn borrowed_view_from_owner_validates_and_borrows_without_copying() {
    let owner = crate::Base64String::encode(CODEC, b"hello").unwrap();
    let before = observation::fast_calls();
    let view = owner.as_base64_ref().unwrap();
    assert_eq!(observation::fast_calls(), before + 1);
    assert_eq!(view.as_bytes().as_ptr(), owner.as_bytes().as_ptr());
    assert_eq!(view.decode_to_vec().unwrap(), b"hello");
    let reference = owner
        .as_base64_ref_with_validation(DecodeValidation::ScalarReference)
        .unwrap();
    assert_eq!(reference.validation(), DecodeValidation::ScalarReference);
    let incompatible = super::super::specifications::runtime_codec(
        *CODEC.settings().alphabet(),
        EncodePadding::Padded,
        DecodePadding::Forbid,
        TrailingBits::RequireCanonical,
    );
    let encoded = crate::Base64String::encode(incompatible, b"f").unwrap();
    assert_eq!(encoded.as_str(), "Zg==");
    assert!(encoded.as_base64_ref().is_err());
}

#[cfg(all(feature = "std", not(miri)))]
#[test]
#[ignore = "opt-in parse-once/reuse measurement, not hardware admission"]
fn borrowed_view_same_process_benchmark() {
    use std::{hint::black_box, time::Instant};
    let _ = crate::initialize_backends();
    for size in [32, 4096, 1_048_576] {
        let plain = std::vec![0xa5; size];
        let input = CODEC.encode_to_string(&plain).unwrap();
        let view = Base64Ref::parse(CODEC, input.as_bytes()).unwrap();
        let mut output = std::vec![0; size];
        let rounds = if size > 4096 { 100 } else { 10_000 };
        for sample in 0..7 {
            for mode in if sample % 2 == 0 {
                [0, 1, 2]
            } else {
                [2, 1, 0]
            } {
                let start = Instant::now();
                for _ in 0..rounds {
                    match mode {
                        0 => {
                            black_box(
                                CODEC
                                    .decode_into(
                                        black_box(input.as_bytes()),
                                        black_box(&mut output),
                                    )
                                    .unwrap(),
                            );
                        }
                        1 => {
                            black_box(
                                black_box(&view)
                                    .decode_into(black_box(&mut output))
                                    .unwrap(),
                            );
                        }
                        _ => {
                            black_box(
                                Base64Ref::parse(CODEC, black_box(input.as_bytes())).unwrap(),
                            );
                        }
                    }
                }
                std::println!(
                    "borrowed-bench,{size},{sample},{mode},{rounds},{}",
                    start.elapsed().as_nanos()
                );
                assert_eq!(output, plain);
            }
        }
    }
}
