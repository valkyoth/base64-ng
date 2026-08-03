//! Exact format, limit, rollback, and redaction conformance tests.

use base64_ng_password::{
    PasslibPbkdf2Algorithm, PasswordRecordErrorKind, PasswordRecordLimits, ShaCryptAlgorithm,
    ShaCryptRounds, decode_pbkdf2_field_into, decode_sha_crypt_checksum_into,
    encode_pbkdf2_field_into, encode_sha_crypt_checksum_into, generate_pbkdf2_record,
    generate_pbkdf2_record_into, generate_sha_crypt_record, generate_sha_crypt_record_into,
    parse_pbkdf2_record, parse_sha_crypt_record,
};

const LIMITS: PasswordRecordLimits = PasswordRecordLimits::new(4096, 2048, 1024, 1024, 4096, 4096);

#[test]
fn passlib_documentation_vectors_parse_decode_and_regenerate() {
    let vectors = [
        (
            PasslibPbkdf2Algorithm::Sha256,
            b"$pbkdf2-sha256$6400$.6UI/S.nXIk8jcbdHx3Fhg$98jZicV16ODfEsEZeYPGHU3kbrUrvUEXOPimVSQDD44"
                .as_slice(),
        ),
        (
            PasslibPbkdf2Algorithm::Sha256,
            b"$pbkdf2-sha256$8000$XAuBMIYQQogxRg$tRRlz8hYn63B9LYiCd6PRo6FMiunY9ozmMMI3srxeRE"
                .as_slice(),
        ),
    ];

    for (algorithm, encoded) in vectors {
        let parsed = parse_pbkdf2_record(encoded, LIMITS).unwrap();
        assert_eq!(parsed.algorithm(), algorithm);
        let mut salt = [0_u8; 1024];
        let salt_len =
            decode_pbkdf2_field_into(parsed.expose_encoded_salt(), &mut salt, LIMITS).unwrap();
        let mut checksum = [0_u8; 64];
        let checksum_len =
            decode_pbkdf2_field_into(parsed.expose_encoded_checksum(), &mut checksum, LIMITS)
                .unwrap();
        assert_eq!(checksum_len, algorithm.checksum_len());
        let regenerated = generate_pbkdf2_record(
            algorithm,
            parsed.rounds(),
            &salt[..salt_len],
            &checksum[..checksum_len],
            LIMITS,
        )
        .unwrap();
        assert_eq!(regenerated.as_bytes(), encoded);
    }
}

#[test]
fn every_pbkdf2_algorithm_enforces_exact_checksum_size() {
    for algorithm in [
        PasslibPbkdf2Algorithm::Sha1,
        PasslibPbkdf2Algorithm::Sha256,
        PasslibPbkdf2Algorithm::Sha512,
    ] {
        let checksum = [0x5a_u8; 64];
        let record = generate_pbkdf2_record(
            algorithm,
            u32::MAX,
            b"salt\0bytes",
            &checksum[..algorithm.checksum_len()],
            LIMITS,
        )
        .unwrap();
        let parsed = parse_pbkdf2_record(record.as_bytes(), LIMITS).unwrap();
        assert_eq!(parsed.algorithm(), algorithm);
        assert_eq!(parsed.rounds(), u32::MAX);

        let error = generate_pbkdf2_record_into(
            algorithm,
            1,
            b"salt",
            &checksum[..algorithm.checksum_len() - 1],
            &mut [0_u8; 256],
            LIMITS,
        )
        .unwrap_err();
        assert_eq!(error.kind(), PasswordRecordErrorKind::InvalidChecksum);
    }
}

#[test]
fn adapted_fields_match_expected_mapping_and_canonicality() {
    let mut encoded = [0_u8; 32];
    let written = encode_pbkdf2_field_into(&[0xfb, 0xff, 0xff], &mut encoded, LIMITS).unwrap();
    assert_eq!(&encoded[..written], b".///");
    let mut decoded = [0_u8; 3];
    assert_eq!(
        decode_pbkdf2_field_into(&encoded[..written], &mut decoded, LIMITS),
        Ok(3)
    );
    assert_eq!(decoded, [0xfb, 0xff, 0xff]);

    for malformed in [b"+///".as_slice(), b".///=".as_slice(), b"AB".as_slice()] {
        assert_eq!(
            decode_pbkdf2_field_into(malformed, &mut decoded, LIMITS)
                .unwrap_err()
                .kind(),
            PasswordRecordErrorKind::InvalidField
        );
    }
}

#[test]
fn sha_crypt_permutations_match_independent_known_answers() {
    let digest256 = core::array::from_fn::<_, 32, _>(|index| u8::try_from(index).unwrap());
    let digest512 = core::array::from_fn::<_, 64, _>(|index| u8::try_from(index).unwrap());
    let cases = [
        (
            ShaCryptAlgorithm::Sha256,
            digest256.as_slice(),
            b"Ic..92E30M/1Lok.CE.43Yl1O.V/FQk46kV2RAF0Sw/".as_slice(),
        ),
        (
            ShaCryptAlgorithm::Sha512,
            digest512.as_slice(),
            b"eI/./gW3L6.9hUl.2sG4OIk9kgV/5215RUUAnsF08En5UgEBq201BQX6Xs.CtEm1EcH7a2lCwQW2Ho18dEVDz."
                .as_slice(),
        ),
    ];

    for (algorithm, digest, expected) in cases {
        let mut encoded = [0_u8; 86];
        let written =
            encode_sha_crypt_checksum_into(algorithm, digest, &mut encoded, LIMITS).unwrap();
        assert_eq!(&encoded[..written], expected);
        let mut decoded = [0_u8; 64];
        let decoded_len =
            decode_sha_crypt_checksum_into(algorithm, &encoded[..written], &mut decoded, LIMITS)
                .unwrap();
        assert_eq!(&decoded[..decoded_len], digest);
    }
}

#[test]
fn openssl_compatible_sha_crypt_records_parse_and_regenerate() {
    let vectors = [
        b"$5$saltstring$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5".as_slice(),
        b"$6$saltstring$svn8UoSVapNtMuq1ukKS4tPQd8iKwSMHWjl/O817G3uBnIFNjnQJuesI68u4OTLiBFdcbYEdFCoEOfaS35inz1"
            .as_slice(),
    ];
    for encoded in vectors {
        let parsed = parse_sha_crypt_record(encoded, LIMITS).unwrap();
        assert_eq!(parsed.rounds(), ShaCryptRounds::implicit());
        let mut digest = [0_u8; 64];
        let digest_len = decode_sha_crypt_checksum_into(
            parsed.algorithm(),
            parsed.expose_encoded_checksum(),
            &mut digest,
            LIMITS,
        )
        .unwrap();
        let regenerated = generate_sha_crypt_record(
            parsed.algorithm(),
            parsed.rounds(),
            parsed.expose_salt(),
            &digest[..digest_len],
            LIMITS,
        )
        .unwrap();
        assert_eq!(regenerated.as_bytes(), encoded);
    }
}

#[test]
fn rounds_salts_delimiters_and_unused_bits_are_strict() {
    let checksum = b"5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5";
    for malformed in [
        b"$5$rounds=0999$salt$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5".as_slice(),
        b"$5$rounds=999$salt$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5".as_slice(),
        b"$5$salt!$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5".as_slice(),
        b"$5$salt$short".as_slice(),
        b"$5$salt$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5$extra".as_slice(),
    ] {
        assert!(parse_sha_crypt_record(malformed, LIMITS).is_err());
    }
    let mut noncanonical = *checksum;
    noncanonical[42] = b'z';
    assert_eq!(
        decode_sha_crypt_checksum_into(
            ShaCryptAlgorithm::Sha256,
            &noncanonical,
            &mut [0_u8; 32],
            LIMITS,
        )
        .unwrap_err()
        .kind(),
        PasswordRecordErrorKind::InvalidChecksum
    );

    for malformed in [
        b"$pbkdf2-sha256$0$c2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".as_slice(),
        b"$pbkdf2-sha256$01$c2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".as_slice(),
        b"$pbkdf2-sha256$1$c2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA$extra".as_slice(),
    ] {
        assert!(parse_pbkdf2_record(malformed, LIMITS).is_err());
    }
}

#[test]
fn one_shot_failures_are_transactional_and_limits_are_distinct() {
    let digest = [7_u8; 32];
    let mut output = [0xa5_u8; 128];
    let snapshot = output;
    let error = generate_sha_crypt_record_into(
        ShaCryptAlgorithm::Sha256,
        ShaCryptRounds::implicit(),
        b"salt",
        &digest,
        &mut output[..8],
        LIMITS,
    )
    .unwrap_err();
    assert_eq!(error.kind(), PasswordRecordErrorKind::OutputTooSmall);
    assert_eq!(output, snapshot);

    let tiny = PasswordRecordLimits::new(8, 8, 4, 4, 8, 8);
    assert_eq!(
        parse_sha_crypt_record(b"$5$salt$5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5", tiny,)
            .unwrap_err()
            .kind(),
        PasswordRecordErrorKind::InputLimitExceeded
    );

    let independent_fields = PasswordRecordLimits::new(256, 128, 4, 32, 256, 64);
    let generated = generate_pbkdf2_record(
        PasslibPbkdf2Algorithm::Sha256,
        1,
        b"salt",
        &[7_u8; 32],
        independent_fields,
    )
    .unwrap();
    assert!(generated.starts_with("$pbkdf2-sha256$1$c2FsdA$"));
}

#[test]
fn every_finite_limit_has_a_distinct_failure_class() {
    let checksum = [7_u8; 32];
    let record = b"$pbkdf2-sha256$1$c2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    let record_limit = PasswordRecordLimits::new(8, 128, 32, 64, 256, 256);
    assert_eq!(
        parse_pbkdf2_record(record, record_limit)
            .unwrap_err()
            .kind(),
        PasswordRecordErrorKind::InputLimitExceeded
    );

    let field_limit = PasswordRecordLimits::new(256, 3, 32, 64, 256, 256);
    assert_eq!(
        encode_pbkdf2_field_into(b"salt", &mut [0_u8; 16], field_limit)
            .unwrap_err()
            .kind(),
        PasswordRecordErrorKind::FieldLimitExceeded
    );

    let salt_limit = PasswordRecordLimits::new(256, 128, 3, 64, 256, 256);
    assert_eq!(
        generate_pbkdf2_record(
            PasslibPbkdf2Algorithm::Sha256,
            1,
            b"salt",
            &checksum,
            salt_limit,
        )
        .unwrap_err()
        .kind(),
        PasswordRecordErrorKind::InvalidSalt
    );

    let decoded_limit = PasswordRecordLimits::new(256, 128, 32, 31, 256, 256);
    assert_eq!(
        generate_pbkdf2_record(
            PasslibPbkdf2Algorithm::Sha256,
            1,
            b"salt",
            &checksum,
            decoded_limit,
        )
        .unwrap_err()
        .kind(),
        PasswordRecordErrorKind::DecodedOutputLimitExceeded
    );

    let generated_limit = PasswordRecordLimits::new(256, 128, 32, 64, 8, 256);
    assert_eq!(
        generate_pbkdf2_record(
            PasslibPbkdf2Algorithm::Sha256,
            1,
            b"salt",
            &checksum,
            generated_limit,
        )
        .unwrap_err()
        .kind(),
        PasswordRecordErrorKind::OutputLimitExceeded
    );

    let work_limit = PasswordRecordLimits::new(256, 128, 32, 64, 256, 3);
    assert_eq!(
        encode_pbkdf2_field_into(b"salt", &mut [0_u8; 16], work_limit)
            .unwrap_err()
            .kind(),
        PasswordRecordErrorKind::WorkLimitExceeded
    );
}

#[test]
fn debug_and_errors_never_emit_record_canaries() {
    let pbkdf2 = parse_pbkdf2_record(
        b"$pbkdf2-sha256$1$U0FMVF9DQU5BUlk$Q0hFQ0tTVU1fQ0FOQVJZX0NIRUNLU1VNX0NBTkFSWV8",
        LIMITS,
    )
    .unwrap();
    let rendered = format!("{pbkdf2:?}");
    assert!(!rendered.contains("U0FMVF9DQU5BUlk"));
    assert!(!rendered.contains("Q0hFQ0tTVU1"));
    assert!(rendered.contains("[REDACTED]"));

    let error =
        parse_pbkdf2_record(b"$pbkdf2-sha256$bad$RAW_CANARY$RAW_CHECKSUM", LIMITS).unwrap_err();
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("RAW_CANARY"));
    assert!(!rendered.contains("RAW_CHECKSUM"));
}

#[test]
fn explicit_sha_crypt_rounds_are_preserved() {
    let digest = [0x33_u8; 64];
    let rounds = ShaCryptRounds::explicit(123_456).unwrap();
    let record =
        generate_sha_crypt_record(ShaCryptAlgorithm::Sha512, rounds, b"salt", &digest, LIMITS)
            .unwrap();
    assert!(record.starts_with("$6$rounds=123456$salt$"));
    assert_eq!(
        parse_sha_crypt_record(record.as_bytes(), LIMITS)
            .unwrap()
            .rounds(),
        rounds
    );
}
