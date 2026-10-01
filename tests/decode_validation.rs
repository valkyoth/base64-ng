use base64_ng::{
    Alphabet, Base64, Codec, CodecBuilder, DecodeValidation, Engine, OneShotError,
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, ValidatedAlphabet,
};

const POLICY: DecodeValidation = DecodeValidation::ScalarReference;
const CODEC: Base64<base64_ng::StrictStandardPadded> = Base64::new(base64_ng::StrictStandardPadded);

fn initialize_backend_health() {
    #[cfg(all(feature = "std", feature = "simd"))]
    {
        // Paired historical calls must not straddle another test's startup KAT:
        // the nonblocking scalar fallback has its own legacy error precedence.
        static INITIALIZE: std::sync::Once = std::sync::Once::new();
        INITIALIZE.call_once(|| {
            let _ = base64_ng::initialize_backends();
        });
    }
}

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
    initialize_backend_health();
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
fn public_vector_boundaries_preserve_reference_results_and_whole_destinations() {
    initialize_backend_health();
    for settings in [
        STRICT_STANDARD_PADDED.settings(),
        STRICT_STANDARD_UNPADDED.settings(),
        STRICT_URL_SAFE_PADDED.settings(),
        STRICT_URL_SAFE_UNPADDED.settings(),
    ] {
        let codec = CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        for len in [
            380, 381, 382, 383, 384, 385, 767, 768, 769, 1537, 3070, 3071, 3072, 3073, 4095, 4096,
            4097,
        ] {
            let input = [0xa5; 4097];
            let mut encoded = [0; 5464];
            let size = codec.encode_into(&input[..len], &mut encoded).unwrap();
            for position in [0, 15, 16, 31, 32, 63, 64, size - 1] {
                for byte in [encoded[position], b'!', b'=', b' ', 0xff] {
                    let mut data = encoded;
                    data[position] = byte;
                    assert_eq!(
                        codec.decoded_len(&data[..size]),
                        codec.decoded_len_with_validation(&data[..size], POLICY)
                    );
                    for capacity in [0, len - 1, len, 4100] {
                        let mut auto = [0x55; 4100];
                        let mut reference = auto;
                        let expected = codec.decode_into_with_validation(
                            &data[..size],
                            &mut reference[..capacity],
                            POLICY,
                        );
                        assert_eq!(
                            codec.decode_into(&data[..size], &mut auto[..capacity]),
                            expected
                        );
                        assert_eq!(auto, reference);
                        if expected.is_err() {
                            assert_eq!(auto, [0x55; 4100]);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn validation_policy_preserves_historical_diagnostics_and_whole_buffers() {
    initialize_backend_health();
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

fn historical_bulk<A: Alphabet, const PAD: bool>(engine: Engine<A, PAD>) {
    let mut encoded = [0; 5464];
    let size = engine.encode_slice(&[0xa5; 4097], &mut encoded).unwrap();
    for position in [
        0,
        15,
        16,
        31,
        32,
        63,
        64,
        511,
        512,
        1023,
        1024,
        4095,
        4096,
        size - 1,
    ] {
        for byte in [encoded[position], b'!', b'=', b' ', 0xff] {
            let mut data = encoded;
            data[position] = byte;
            let input = &data[..size];
            assert_eq!(
                engine.validated_decoded_len_with_validation(input, DecodeValidation::Auto),
                engine.validated_decoded_len_with_validation(input, POLICY)
            );
            for capacity in [0, 4096, 4097, 4104] {
                let mut auto = [0x55; 4104];
                let mut reference = auto;
                assert_eq!(
                    engine.decode_slice_with_validation(
                        input,
                        &mut auto[..capacity],
                        DecodeValidation::Auto
                    ),
                    engine.decode_slice_with_validation(input, &mut reference[..capacity], POLICY)
                );
                assert_eq!(auto, reference);
                assert_eq!(
                    engine.decode_slice_clear_tail_with_validation(
                        input,
                        &mut auto[..capacity],
                        DecodeValidation::Auto
                    ),
                    engine.decode_slice_clear_tail_with_validation(
                        input,
                        &mut reference[..capacity],
                        POLICY
                    )
                );
                assert_eq!(auto, reference);
            }
        }
    }
}

#[test]
fn historical_bulk_errors_never_acquire_canonical_diagnostics_or_mutation_rules() {
    initialize_backend_health();
    historical_bulk(base64_ng::STANDARD);
    historical_bulk(base64_ng::STANDARD_NO_PAD);
    historical_bulk(base64_ng::URL_SAFE);
    historical_bulk(base64_ng::URL_SAFE_NO_PAD);
}

#[cfg(all(
    feature = "std",
    feature = "simd",
    any(
        all(target_arch = "aarch64", target_endian = "little"),
        target_arch = "wasm32",
        all(target_arch = "riscv64", target_os = "linux")
    )
))]
#[test]
fn production_16_lane_simd_rejects_every_invalid_lane_without_writing() {
    // This integration target links the non-cfg(test) library. Do not silently
    // pass on scalar fallback if the production classifier failed its KAT.
    initialize_backend_health();
    #[cfg(target_arch = "riscv64")]
    let rvv_expected = base64_ng::runtime::backend_report().active_decode_backend();
    #[cfg(target_arch = "riscv64")]
    if std::env::var_os("BASE64_NG_REQUIRE_X60").is_some() {
        assert_eq!(rvv_expected, base64_ng::runtime::Backend::Rvv);
    }
    let assert_backend = || {
        #[cfg(target_arch = "riscv64")]
        let expected = rvv_expected;
        #[cfg(target_arch = "aarch64")]
        let expected = base64_ng::runtime::Backend::Neon;
        #[cfg(target_arch = "wasm32")]
        let expected = if base64_ng::runtime::backend_report().wasm_artifact_posture
            == base64_ng::runtime::WasmArtifactPosture::Simd128Artifact
        {
            base64_ng::runtime::Backend::WasmSimd128
        } else {
            base64_ng::runtime::Backend::Scalar
        };
        assert_eq!(
            base64_ng::runtime::backend_report().active_decode_backend(),
            expected
        );
    };
    assert_backend();
    for settings in [
        STRICT_STANDARD_PADDED.settings(),
        STRICT_STANDARD_UNPADDED.settings(),
        STRICT_URL_SAFE_PADDED.settings(),
        STRICT_URL_SAFE_UNPADDED.settings(),
    ] {
        let codec = CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        let mut input = [b'A'; 4096];
        let mut output = [0xa5; 3072];
        assert_eq!(codec.validate(&input), Ok(()));
        assert_eq!(codec.decode_into(&input, &mut output), Ok(3072));
        assert_eq!(output, [0; 3072]);
        output.fill(0xa5);
        // First, interior and last vector blocks, before the reserved tail.
        for block in [0, 2048, 4048] {
            for lane in 0..32 {
                for byte in 0..=u8::MAX {
                    if settings.alphabet().as_array().contains(&byte) {
                        continue;
                    }
                    input[block + lane] = byte;
                    assert!(matches!(
                        codec.validate(&input),
                        Err(OneShotError::Input(_))
                    ));
                    assert!(matches!(
                        codec.decode_into(&input, &mut output),
                        Err(OneShotError::Input(_))
                    ));
                    assert_eq!(output, [0xa5; 3072]);
                    assert_backend();
                }
                input[block + lane] = b'A';
            }
        }
    }
}
