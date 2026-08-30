#![no_main]

use base64_ng_fuzz::pem_document::{PARSE_LIMITS, assert_generation_round_trip};
use base64_ng_pem::{PemDocumentParser, PemParsePolicy, parse_pem_document};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let seed = data.first().copied().unwrap_or(1);
    let payload = data.get(1..).unwrap_or_default();
    assert_generation_round_trip(payload);

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
                (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
                    panic!("PEM one-shot/chunk result classification mismatch")
                }
            }
        }
    }
});
