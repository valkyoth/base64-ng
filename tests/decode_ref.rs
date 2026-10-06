use base64_ng::{
    Base64Ref, Codec, DecodeValidation, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
};

fn check<S: Codec + Clone>(codec: base64_ng::Base64<S>) {
    let mut source = [0; 4097];
    for (i, byte) in source.iter_mut().enumerate() {
        *byte = (i % 256) as u8;
    }
    for size in [
        0, 1, 2, 3, 11, 12, 13, 23, 24, 25, 31, 32, 33, 127, 128, 129, 3071, 3072, 3073, 4097,
    ] {
        let mut encoded = [0; 5464];
        let n = codec.encode_into(&source[..size], &mut encoded).unwrap();
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            let view =
                Base64Ref::parse_with_validation(codec.clone(), &encoded[..n], policy).unwrap();
            assert_eq!(view.as_bytes(), &encoded[..n]);
            assert_eq!(view.as_bytes().as_ptr(), encoded.as_ptr());
            assert_eq!(view.len(), n);
            assert_eq!(view.is_empty(), n == 0);
            assert_eq!(view.decoded_len(), size);
            assert_eq!(view.settings(), codec.settings());
            assert_eq!(view.codec().settings(), codec.settings());
            assert_eq!(view.validation(), policy);
            for capacity in [0, size.saturating_sub(1), size, size + 5] {
                let mut output = [0xa5; 4102];
                let reference = codec.decode_into_with_validation(
                    &encoded[..n],
                    &mut output[..capacity],
                    policy,
                );
                let mut actual = [0xa5; 4102];
                for _ in 0..2 {
                    assert_eq!(view.decode_into(&mut actual[..capacity]), reference);
                    assert_eq!(actual, output);
                }
            }
        }
    }
}

#[test]
fn borrowed_view_profiles_boundaries_and_repeated_destinations() {
    check(STRICT_STANDARD_PADDED);
    check(STRICT_STANDARD_UNPADDED);
    check(STRICT_URL_SAFE_PADDED);
    check(STRICT_URL_SAFE_UNPADDED);
}

#[test]
fn borrowed_view_rejects_invalid_construction_with_exact_errors() {
    for input in [
        b"A".as_slice(),
        b"!!!!",
        b"Zh==",
        b"Zm9=",
        b"Zm=9",
        b"AA A",
        b"AA\xffA",
        b"____",
        b"Zg===",
        b"Zg",
    ] {
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            assert_eq!(
                Base64Ref::parse_with_validation(STRICT_STANDARD_PADDED, input, policy)
                    .unwrap_err(),
                STRICT_STANDARD_PADDED
                    .decoded_len_with_validation(input, policy)
                    .unwrap_err()
            );
        }
    }
    assert!(Base64Ref::parse(STRICT_URL_SAFE_PADDED, b"////").is_err());
    assert!(Base64Ref::parse(STRICT_STANDARD_UNPADDED, b"Zg==").is_err());
}

#[test]
fn borrowed_view_security_retry_keeps_proof_and_entire_destination() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let input = *b"AAECAwQFBgcICQoLDA0ODw==";
        let view =
            Base64Ref::parse_with_validation(STRICT_STANDARD_PADDED, &input, policy).unwrap();
        for capacity in [0, 15, 16, 19, 1, 16] {
            let mut storage = [0xa5; 24];
            let result = view.decode_into(&mut storage[2..2 + capacity]);
            if capacity < 16 {
                assert_eq!(
                    result,
                    Err(base64_ng::OneShotError::OutputTooSmall {
                        required: 16,
                        available: capacity,
                    })
                );
                assert_eq!(storage, [0xa5; 24]);
            } else {
                assert_eq!(result, Ok(16));
                assert_eq!(
                    &storage[2..18],
                    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
                );
                assert_eq!(&storage[..2], &[0xa5; 2]);
                assert_eq!(&storage[18..], &[0xa5; 6]);
            }
            assert_eq!(view.as_bytes(), input);
        }
    }
}

#[test]
fn security_malformed_blocks_and_tails_never_commit() {
    let _ = base64_ng::initialize_backends();
    let mut canonical = [b'A'; 4100];
    canonical[4096..].copy_from_slice(b"Zg==");
    // Interior padding, high-bit lanes, and noncanonical final sextets must
    // fail even when the destination is too short. No prefix may be committed.
    for (index, byte) in [
        (0, b'='),
        (15, 0x80),
        (16, b'='),
        (2047, 0xff),
        (4095, b'='),
        (4097, b'h'),
        (4098, b'9'),
    ] {
        let mut input = canonical;
        input[index] = byte;
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            let error = Base64Ref::parse_with_validation(STRICT_STANDARD_PADDED, &input, policy)
                .unwrap_err();
            assert!(matches!(error, base64_ng::OneShotError::Input(_)));
            for capacity in [0, 8, 3073, 4100] {
                let mut output = [0xa5; 4104];
                assert_eq!(
                    STRICT_STANDARD_PADDED.decode_into_with_validation(
                        &input,
                        &mut output[2..2 + capacity],
                        policy,
                    ),
                    Err(error)
                );
                assert_eq!(output, [0xa5; 4104]);
            }
        }
    }
}
