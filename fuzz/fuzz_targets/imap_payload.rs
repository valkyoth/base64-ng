#![no_main]

use base64_ng_imap::{
    ImapPayloadLimits, decode_modified_utf7_payload_to_vec,
    encode_modified_utf7_payload_to_string,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = ImapPayloadLimits::new(8_192, 16_384, 8_192);

    if data.len() <= 8_192 && data.len().is_multiple_of(2) {
        let encoded = encode_modified_utf7_payload_to_string(data, limits).unwrap();
        assert!(!encoded.as_bytes().contains(&b'='));
        assert!(encoded.as_bytes().iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'+' | b',')
        }));
        let decoded = decode_modified_utf7_payload_to_vec(encoded.as_bytes(), limits).unwrap();
        assert_eq!(decoded, data);
    }

    if data.len() <= 8_192 {
        match decode_modified_utf7_payload_to_vec(data, limits) {
            Ok(decoded) => {
                assert!(decoded.len().is_multiple_of(2));
                let canonical = encode_modified_utf7_payload_to_string(&decoded, limits).unwrap();
                assert_eq!(canonical.as_bytes(), data);
            }
            Err(_) => {}
        }
    }
});
