use base64_ng::{
    Alphabet, Base64, Codec, CodecBuilder, DecodeValidation, Engine, OneShotError,
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, ValidatedAlphabet,
};

const POLICY: DecodeValidation = DecodeValidation::ScalarReference;
const CODEC: Base64<base64_ng::StrictStandardPadded> = Base64::new(base64_ng::StrictStandardPadded);

fn canonical<S: Codec>(codec: Base64<S>) {
    for input in [
        b"".as_slice(),
        b"Zg==",
        b"Zg",
        b"Zh==",
        b"Zh",
        b"Zg=",
        b"Zm9v",
        b"Zm9v!!!!",
        b"-_8=",
        b"+/8=",
        b" A A",
        b"====",
        b"\xffAAA",
    ] {
        for policy in [DecodeValidation::Auto, POLICY] {
            assert_eq!(
                codec.validate_with_validation(input, policy),
                codec.validate(input)
            );
            assert_eq!(
                codec.decoded_len_with_validation(input, policy),
                codec.decoded_len(input)
            );
            for capacity in 0..=12 {
                let mut expected = [0xa5; 16];
                let mut actual = expected;
                assert_eq!(
                    codec.decode_into_with_validation(input, &mut actual[..capacity], policy),
                    codec.decode_into(input, &mut expected[..capacity])
                );
                assert_eq!(actual, expected);
                if codec.validate(input).is_err() {
                    assert_eq!(actual, [0xa5; 16]);
                }
            }
        }
    }
    #[cfg(feature = "alloc")]
    for policy in [DecodeValidation::Auto, POLICY] {
        let input = codec.encode_to_string(b"hello").unwrap();
        assert_eq!(
            codec
                .decode_to_vec_with_validation(input.as_bytes(), policy)
                .unwrap(),
            b"hello"
        );
        assert!(matches!(
            codec.decode_to_vec_with_limit_and_validation(input.as_bytes(), 4, policy),
            Err(OneShotError::AllocationLimitExceeded {
                required: 5,
                limit: 4
            })
        ));
        assert!(matches!(
            codec.decode_to_vec_with_limit_and_validation(b"!!!!", 0, policy),
            Err(OneShotError::Input(_))
        ));
    }
}

fn historical<A: Alphabet, const PAD: bool>(engine: Engine<A, PAD>) {
    let source = [0xa5; 99];
    let mut encoded = [0; 132];
    engine.encode_slice(&source, &mut encoded).unwrap();
    for policy in [DecodeValidation::Auto, POLICY] {
        let mut output = [0xff; 104];
        assert_eq!(
            engine.decode_slice_with_validation(&encoded, &mut output, policy),
            Ok(99)
        );
        assert_eq!(&output[..99], &source);
        assert_eq!(&output[99..], &[0xff; 5]);
        assert_eq!(
            engine.validated_decoded_len_with_validation(&encoded, policy),
            Ok(99)
        );
        for position in 0..encoded.len() {
            for invalid in [b'!', b'=', b' ', 0xff] {
                let mut malformed = encoded;
                malformed[position] = invalid;
                for capacity in [0, 1, 98, 99, 104] {
                    let mut expected = [0xa5; 104];
                    let mut actual = expected;
                    assert_eq!(
                        engine.decode_slice_with_validation(
                            &malformed,
                            &mut actual[..capacity],
                            policy
                        ),
                        engine.decode_slice(&malformed, &mut expected[..capacity])
                    );
                    assert_eq!(actual, expected);
                    assert_eq!(
                        engine.decode_slice_clear_tail_with_validation(
                            &malformed,
                            &mut actual[..capacity],
                            policy
                        ),
                        engine.decode_slice_clear_tail(&malformed, &mut expected[..capacity])
                    );
                    assert_eq!(actual, expected);
                }
                assert_eq!(
                    engine.validate_result_with_validation(&malformed, policy),
                    engine.validate_result(&malformed)
                );
            }
        }
        #[cfg(feature = "alloc")]
        assert_eq!(
            engine.decode_vec_with_validation(&encoded, policy).unwrap(),
            source
        );
    }
}

#[test]
fn validation_policy_preserves_canonical_grammar_and_transactionality() {
    assert_eq!(DecodeValidation::default(), DecodeValidation::Auto);
    canonical(CODEC);
    canonical(STRICT_STANDARD_UNPADDED);
    canonical(STRICT_URL_SAFE_PADDED);
    canonical(STRICT_URL_SAFE_UNPADDED);
    canonical(
        CodecBuilder::new(
            ValidatedAlphabet::new(
                *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
            )
            .unwrap(),
        )
        .build()
        .unwrap(),
    );
    let mut output = [0x55; 3];
    assert!(matches!(
        STRICT_STANDARD_PADDED.decode_into_with_validation(b"!!!!", &mut output[..0], POLICY),
        Err(OneShotError::Input(_))
    ));
    assert_eq!(output, [0x55; 3]);
}

#[test]
fn validation_policy_preserves_historical_diagnostics_and_whole_buffers() {
    historical(base64_ng::STANDARD);
    historical(base64_ng::STANDARD_NO_PAD);
    historical(base64_ng::URL_SAFE);
    historical(base64_ng::URL_SAFE_NO_PAD);
    // Historical shape-only length remains intentionally weaker.
    assert_eq!(base64_ng::STANDARD.decoded_len(b"!!!!"), Ok(3));
    assert!(
        base64_ng::STANDARD
            .validated_decoded_len_with_validation(b"!!!!", POLICY)
            .is_err()
    );
    for policy in [DecodeValidation::Auto, POLICY] {
        let mut output = [0xa5; 6];
        assert!(matches!(
            base64_ng::STANDARD.decode_slice_with_validation(b"Zm9v!!!!", &mut output, policy),
            Err(base64_ng::DecodeError::InvalidByte { index: 4, .. })
        ));
        assert_eq!(&output[..3], b"foo");
        assert_eq!(&output[3..], &[0xa5; 3]);
        assert_eq!(
            STRICT_STANDARD_PADDED.decode_into_with_validation(b"Zm9v", &mut output, policy),
            Ok(3)
        );
        assert_eq!(&output[..3], b"foo");
    }
}
