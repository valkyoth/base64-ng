use base64_ng_pem::{
    PemErrorKind, PemGenerationOptions, PemLabel, PemLimits, PemParsePolicy,
    encode_pem_block_to_string, parse_pem_document,
};

pub const MAX_PAYLOAD: usize = 8192;
pub const MAX_DOCUMENT: usize = 16_384;
pub const GENERATION_LIMITS: PemLimits = PemLimits::new(
    MAX_PAYLOAD,
    MAX_DOCUMENT,
    MAX_PAYLOAD,
    2048,
    128,
    16,
    4096,
    MAX_PAYLOAD,
);
pub const PARSE_LIMITS: PemLimits = PemLimits::new(
    MAX_DOCUMENT,
    MAX_DOCUMENT,
    MAX_PAYLOAD,
    2048,
    128,
    16,
    4096,
    12 * MAX_DOCUMENT,
);

/// Exercises canonical generation and strict re-parsing for one fuzz payload.
pub fn assert_generation_round_trip(payload: &[u8]) {
    let generated = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        payload,
        GENERATION_LIMITS,
        PemGenerationOptions::default(),
    );
    if payload.is_empty() {
        assert_eq!(generated.unwrap_err().kind(), PemErrorKind::InvalidBody);
    } else if payload.len() > MAX_PAYLOAD {
        assert_eq!(
            generated.unwrap_err().kind(),
            PemErrorKind::InputLimitExceeded
        );
    } else {
        let generated = generated.unwrap();
        let parsed =
            parse_pem_document(generated.as_bytes(), PARSE_LIMITS, PemParsePolicy::Strict).unwrap();
        assert_eq!(parsed.blocks()[0].contents(), payload);
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PAYLOAD, assert_generation_round_trip};

    #[test]
    fn oversized_payload_is_an_expected_generation_rejection() {
        assert_generation_round_trip(&[0x5a; MAX_PAYLOAD + 1]);
    }
}
