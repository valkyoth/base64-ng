//! Differential checks against Python Base64 and OpenSSL SHA-crypt.

use std::io::Write;
use std::process::Command;

use base64_ng_password::{
    PasslibPbkdf2Algorithm, PasswordRecordLimits, decode_pbkdf2_field_into,
    decode_sha_crypt_checksum_into, generate_pbkdf2_record, generate_sha_crypt_record,
    parse_pbkdf2_record, parse_sha_crypt_record,
};

#[test]
fn adapted_fields_match_python_standard_base64_mapping() {
    let limits = PasswordRecordLimits::default();
    for length in 0_usize..=258 {
        let input: Vec<u8> = (0..length)
            .map(|index| {
                u8::try_from(index % 256)
                    .unwrap()
                    .wrapping_mul(73)
                    .wrapping_add(19)
            })
            .collect();
        let script = "import base64,sys;sys.stdout.buffer.write(base64.b64encode(sys.stdin.buffer.read()).rstrip(b'=') .replace(b'+',b'.'))";
        let mut child = Command::new("python3")
            .args(["-c", script])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&input).unwrap();
        let expected = child.wait_with_output().unwrap();
        assert!(expected.status.success());

        let checksum_len = if length <= 20 {
            20
        } else if length <= 32 {
            32
        } else {
            64
        };
        let algorithm = match checksum_len {
            20 => PasslibPbkdf2Algorithm::Sha1,
            32 => PasslibPbkdf2Algorithm::Sha256,
            _ => PasslibPbkdf2Algorithm::Sha512,
        };
        let mut checksum = [0_u8; 64];
        checksum[..checksum_len].fill(0x42);
        let record =
            generate_pbkdf2_record(algorithm, 1, &input, &checksum[..checksum_len], limits)
                .unwrap();
        let parsed = parse_pbkdf2_record(record.as_bytes(), limits).unwrap();
        assert_eq!(parsed.expose_encoded_salt(), expected.stdout);
        let mut decoded = vec![0_u8; input.len()];
        assert_eq!(
            decode_pbkdf2_field_into(parsed.expose_encoded_salt(), &mut decoded, limits),
            Ok(input.len())
        );
        assert_eq!(decoded, input);
    }
}

#[test]
fn openssl_sha_crypt_outputs_are_accepted_when_available() {
    for (flag, algorithm) in [
        ("-5", base64_ng_password::ShaCryptAlgorithm::Sha256),
        ("-6", base64_ng_password::ShaCryptAlgorithm::Sha512),
    ] {
        let Ok(result) = Command::new("openssl")
            .args(["passwd", flag, "-salt", "saltstring", "Hello world!"])
            .output()
        else {
            return;
        };
        if !result.status.success() {
            return;
        }
        let record = result.stdout.strip_suffix(b"\n").unwrap_or(&result.stdout);
        let parsed = parse_sha_crypt_record(record, PasswordRecordLimits::default()).unwrap();
        assert_eq!(parsed.algorithm(), algorithm);
        let mut digest = [0_u8; 64];
        let written = decode_sha_crypt_checksum_into(
            algorithm,
            parsed.expose_encoded_checksum(),
            &mut digest,
            PasswordRecordLimits::default(),
        )
        .unwrap();
        let regenerated = generate_sha_crypt_record(
            algorithm,
            parsed.rounds(),
            parsed.expose_salt(),
            &digest[..written],
            PasswordRecordLimits::default(),
        )
        .unwrap();
        assert_eq!(regenerated.as_bytes(), record);
    }
}
