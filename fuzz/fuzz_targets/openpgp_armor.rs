#![no_main]

use base64_ng_openpgp::{
    ArmorDocumentParser, ArmorType, ChecksumGeneration, ChecksumPolicy, GenerationOptions,
    LineEnding, OpenPgpLimits, encode_armor_to_string, parse_armor_document,
};
use libfuzzer_sys::fuzz_target;

const LIMITS: OpenPgpLimits =
    OpenPgpLimits::new(8192, 16_384, 8192, 2048, 32, 4096, 64, 16, 2048, 8192);

fuzz_target!(|data: &[u8]| {
    let seed = data.first().copied().unwrap_or(1);
    let payload = data.get(1..).unwrap_or_default();
    let kind = match seed & 3 {
        0 => ArmorType::Message,
        1 => ArmorType::PublicKey,
        2 => ArmorType::PrivateKey,
        _ => ArmorType::Signature,
    };
    let checksum = if seed & 4 == 0 {
        ChecksumGeneration::Omit
    } else {
        ChecksumGeneration::LegacyCrc24
    };
    if payload.len() <= LIMITS.max_decoded_output_bytes() {
        let generated = encode_armor_to_string(
            kind,
            &[],
            payload,
            LIMITS,
            GenerationOptions::new(checksum).with_line_ending(LineEnding::Lf),
        )
        .expect("bounded payload must armor");
        let parsed = parse_armor_document(generated.as_bytes(), LIMITS, ChecksumPolicy::Rfc9580)
            .expect("generated armor must parse");
        assert_eq!(parsed.blocks()[0].contents(), payload);
    }

    for policy in [ChecksumPolicy::Rfc9580, ChecksumPolicy::RequireValidCrc24] {
        let one_shot = parse_armor_document(data, LIMITS, policy);
        let mut incremental = ArmorDocumentParser::new(LIMITS, policy);
        let mut update_failed = false;
        for chunk in data.chunks(usize::from(seed % 31) + 1) {
            if incremental.update(chunk).is_err() {
                update_failed = true;
                break;
            }
        }
        if update_failed {
            assert!(data.len() > LIMITS.max_input_bytes());
        } else {
            let chunked = incremental.finish();
            match (one_shot, chunked) {
                (Ok(expected), Ok(actual)) => assert_eq!(actual, expected),
                (Err(expected), Err(actual)) => assert_eq!(actual.kind(), expected.kind()),
                (left, right) => panic!("OpenPGP one-shot/chunk mismatch: {left:?} {right:?}"),
            }
        }
    }
});
