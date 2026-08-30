#![no_main]

use base64_ng_password::{
    PasswordRecordLimits, decode_pbkdf2_field_into, decode_sha_crypt_checksum_into,
    generate_pbkdf2_record_into, generate_sha_crypt_record_into, parse_pbkdf2_record,
    parse_sha_crypt_record,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let limits = PasswordRecordLimits::new(4096, 2048, 1024, 1024, 4096, 4096);
    let mut decoded =
        core::array::from_fn::<_, 1024, _>(|index| u8::try_from(index % 251).unwrap());
    let mut regenerated =
        core::array::from_fn::<_, 4096, _>(|index| u8::try_from(index % 251).unwrap());

    if let Ok(record) = parse_pbkdf2_record(input, limits) {
        let salt_len = decode_pbkdf2_field_into(record.expose_encoded_salt(), &mut decoded, limits)
            .expect("validated PBKDF2 salt must decode");
        let mut checksum =
            core::array::from_fn::<_, 64, _>(|index| u8::try_from(index % 251).unwrap());
        let checksum_len =
            decode_pbkdf2_field_into(record.expose_encoded_checksum(), &mut checksum, limits)
                .expect("validated PBKDF2 checksum must decode");
        let written = generate_pbkdf2_record_into(
            record.algorithm(),
            record.rounds(),
            &decoded[..salt_len],
            &checksum[..checksum_len],
            &mut regenerated,
            limits,
        )
        .expect("validated PBKDF2 record must regenerate");
        assert_eq!(&regenerated[..written], input);
    }

    if let Ok(record) = parse_sha_crypt_record(input, limits) {
        let digest_len = decode_sha_crypt_checksum_into(
            record.algorithm(),
            record.expose_encoded_checksum(),
            &mut decoded,
            limits,
        )
        .expect("validated SHA-crypt checksum must decode");
        let written = generate_sha_crypt_record_into(
            record.algorithm(),
            record.rounds(),
            record.expose_salt(),
            &decoded[..digest_len],
            &mut regenerated,
            limits,
        )
        .expect("validated SHA-crypt record must regenerate");
        assert_eq!(&regenerated[..written], input);
    }
});
