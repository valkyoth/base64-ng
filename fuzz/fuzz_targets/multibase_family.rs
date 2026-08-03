#![no_main]

use base64_ng_multibase::{
    Base64MultibaseEncoding, Base64MultibaseLimits, decode_base64_multibase_to_vec,
    encode_base64_multibase_to_string,
};
use libfuzzer_sys::fuzz_target;

const LIMITS: Base64MultibaseLimits = Base64MultibaseLimits::new(8_192, 10_925, 8_192);

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or_default();
    let payload = data.get(1..).unwrap_or_default();
    if payload.len() <= LIMITS.max_input_bytes() {
        let encoding = Base64MultibaseEncoding::ALL[usize::from(selector & 3)];
        let encoded = encode_base64_multibase_to_string(encoding, payload, LIMITS).unwrap();
        let decoded = decode_base64_multibase_to_vec(encoded.as_bytes(), LIMITS).unwrap();
        assert_eq!(decoded.encoding(), encoding);
        assert_eq!(decoded.as_bytes(), payload);
    }

    if let Ok(decoded) = decode_base64_multibase_to_vec(data, LIMITS) {
        let canonical =
            encode_base64_multibase_to_string(decoded.encoding(), decoded.as_bytes(), LIMITS)
                .unwrap();
        assert_eq!(canonical.as_bytes(), data);
    }
});
