//! Caller-owned password-record operations without the `alloc` feature.

use base64_ng_password::{
    PasslibPbkdf2Algorithm, PasswordRecordLimits, ShaCryptAlgorithm, ShaCryptRounds,
    decode_pbkdf2_field_into, decode_sha_crypt_checksum_into, generate_pbkdf2_record_into,
    generate_sha_crypt_record_into, parse_pbkdf2_record, parse_sha_crypt_record,
};

const LIMITS: PasswordRecordLimits = PasswordRecordLimits::new(512, 128, 64, 64, 512, 512);

#[test]
fn no_alloc_pbkdf2_record_round_trip_uses_caller_owned_storage() {
    let checksum = [0x42_u8; 32];
    let mut record = [0_u8; 256];
    let written = generate_pbkdf2_record_into(
        PasslibPbkdf2Algorithm::Sha256,
        29_000,
        b"salt",
        &checksum,
        &mut record,
        LIMITS,
    )
    .unwrap();

    let parsed = parse_pbkdf2_record(&record[..written], LIMITS).unwrap();
    assert_eq!(parsed.rounds(), 29_000);
    let mut decoded = [0_u8; 32];
    assert_eq!(
        decode_pbkdf2_field_into(parsed.expose_encoded_checksum(), &mut decoded, LIMITS),
        Ok(32)
    );
    assert_eq!(decoded, checksum);
}

#[test]
fn no_alloc_sha_crypt_record_round_trip_uses_caller_owned_storage() {
    let digest = [0x24_u8; 32];
    let mut record = [0_u8; 128];
    let written = generate_sha_crypt_record_into(
        ShaCryptAlgorithm::Sha256,
        ShaCryptRounds::implicit(),
        b"salt",
        &digest,
        &mut record,
        LIMITS,
    )
    .unwrap();

    let parsed = parse_sha_crypt_record(&record[..written], LIMITS).unwrap();
    let mut decoded = [0_u8; 32];
    assert_eq!(
        decode_sha_crypt_checksum_into(
            ShaCryptAlgorithm::Sha256,
            parsed.expose_encoded_checksum(),
            &mut decoded,
            LIMITS,
        ),
        Ok(32)
    );
    assert_eq!(decoded, digest);
}

#[test]
fn no_alloc_work_failure_leaves_output_unchanged() {
    let mut output = [0xa5_u8; 32];
    let limits = PasswordRecordLimits::new(128, 128, 64, 64, 128, 85);
    assert!(
        decode_sha_crypt_checksum_into(
            ShaCryptAlgorithm::Sha256,
            b"5B8vYYiY.CVt1RlTTf8KbXBH3hsxY/GNooZaBBGWEc5",
            &mut output,
            limits,
        )
        .is_err()
    );
    assert_eq!(output, [0xa5; 32]);
}
