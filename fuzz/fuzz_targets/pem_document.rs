#![no_main]

use base64_ng_pem::{
    PemDocumentParser, PemErrorKind, PemGenerationOptions, PemLabel, PemLimits, PemParsePolicy,
    encode_pem_block_to_string, parse_pem_document,
};
use libfuzzer_sys::fuzz_target;

const MAX_PAYLOAD: usize = 8192;
const MAX_DOCUMENT: usize = 16_384;
const GENERATION_LIMITS: PemLimits = PemLimits::new(
    MAX_PAYLOAD,
    MAX_DOCUMENT,
    MAX_PAYLOAD,
    2048,
    128,
    16,
    4096,
    MAX_PAYLOAD,
);
const PARSE_LIMITS: PemLimits = PemLimits::new(
    MAX_DOCUMENT,
    MAX_DOCUMENT,
    MAX_PAYLOAD,
    2048,
    128,
    16,
    4096,
    12 * MAX_DOCUMENT,
);

fuzz_target!(|data: &[u8]| {
    let seed = data.first().copied().unwrap_or(1);
    let payload = data.get(1..).unwrap_or_default();
    let generated = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        payload,
        GENERATION_LIMITS,
        PemGenerationOptions::default(),
    );
    if payload.is_empty() {
        assert_eq!(generated.unwrap_err().kind(), PemErrorKind::InvalidBody);
    } else {
        let generated = generated.unwrap();
        let parsed = parse_pem_document(
            generated.as_bytes(),
            PARSE_LIMITS,
            PemParsePolicy::Strict,
        )
        .unwrap();
        assert_eq!(parsed.blocks()[0].contents(), payload);
    }

    for policy in [PemParsePolicy::Strict, PemParsePolicy::Rfc7468Compatible] {
        let one_shot = parse_pem_document(data, PARSE_LIMITS, policy);
        let mut incremental = PemDocumentParser::new(PARSE_LIMITS, policy);
        let mut update_failed = false;
        for chunk in data.chunks(usize::from(seed % 31) + 1) {
            if incremental.update(chunk).is_err() {
                update_failed = true;
                break;
            }
        }
        if update_failed {
            assert!(data.len() > PARSE_LIMITS.max_input_bytes());
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
