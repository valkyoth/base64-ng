#![no_main]

use base64_ng_pem::{
    PemDocumentParser, PemGenerationOptions, PemLabel, PemLimits, PemParsePolicy,
    encode_pem_block_to_string, parse_pem_document,
};
use libfuzzer_sys::fuzz_target;

const LIMITS: PemLimits = PemLimits::new(8192, 16_384, 8192, 2048, 128, 16, 4096, 8192);

fuzz_target!(|data: &[u8]| {
    let seed = data.first().copied().unwrap_or(1);
    let payload = data.get(1..).unwrap_or_default();
    let generated = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        payload,
        LIMITS,
        PemGenerationOptions::default(),
    )
    .unwrap();
    let parsed = parse_pem_document(generated.as_bytes(), LIMITS, PemParsePolicy::Strict).unwrap();
    assert_eq!(parsed.blocks()[0].contents(), payload);

    for policy in [PemParsePolicy::Strict, PemParsePolicy::Rfc7468Compatible] {
        let one_shot = parse_pem_document(data, LIMITS, policy);
        let mut incremental = PemDocumentParser::new(LIMITS, policy);
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
                (left, right) => panic!("PEM one-shot/chunk mismatch: {left:?} {right:?}"),
            }
        }
    }
});
