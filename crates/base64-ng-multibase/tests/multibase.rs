//! Base64-family multibase conformance and boundary tests.

use base64_ng_multibase::{
    Base64MultibaseDecoder, Base64MultibaseEncoder, Base64MultibaseEncoding,
    Base64MultibaseErrorKind, Base64MultibaseLimits, Base64MultibaseStatus,
    MultibaseRegistryStatus, base64_multibase_encoded_len, decode_base64_multibase_into,
    encode_base64_multibase_into, validate_base64_multibase,
};

const GENEROUS: Base64MultibaseLimits = Base64MultibaseLimits::new(4_096, 4_096, 4_096);

#[test]
fn registry_metadata_is_exact_and_case_sensitive() {
    let expected = [
        (
            Base64MultibaseEncoding::Base64,
            b'm',
            "base64",
            MultibaseRegistryStatus::Final,
        ),
        (
            Base64MultibaseEncoding::Base64Pad,
            b'M',
            "base64pad",
            MultibaseRegistryStatus::Experimental,
        ),
        (
            Base64MultibaseEncoding::Base64Url,
            b'u',
            "base64url",
            MultibaseRegistryStatus::Final,
        ),
        (
            Base64MultibaseEncoding::Base64UrlPad,
            b'U',
            "base64urlpad",
            MultibaseRegistryStatus::Final,
        ),
    ];
    for (encoding, prefix, name, status) in expected {
        assert_eq!(encoding.prefix(), prefix);
        assert_eq!(encoding.registry_name(), name);
        assert_eq!(encoding.registry_status(), status);
        assert_eq!(Base64MultibaseEncoding::from_prefix(prefix), Some(encoding));
    }
    for prefix in u8::MIN..=u8::MAX {
        let supported = matches!(prefix, b'm' | b'M' | b'u' | b'U');
        assert_eq!(
            Base64MultibaseEncoding::from_prefix(prefix).is_some(),
            supported
        );
    }
}

#[test]
fn official_vectors_and_leading_zeroes_round_trip() {
    assert_official_vectors(include_str!("fixtures/basic.csv"), b"yes mani !");
    assert_official_vectors(include_str!("fixtures/leading_zero.csv"), b"\0yes mani !");
    assert_official_vectors(
        include_str!("fixtures/two_leading_zeros.csv"),
        b"\0\0yes mani !",
    );
}

#[test]
fn every_small_length_round_trips_all_four_encodings() {
    let mut input = [0u8; 257];
    fill_pattern(&mut input);
    for len in 0..=input.len() {
        for encoding in Base64MultibaseEncoding::ALL {
            let required = base64_multibase_encoded_len(encoding, len).unwrap();
            let mut encoded = [0x55; 400];
            let written =
                encode_base64_multibase_into(encoding, &input[..len], &mut encoded, GENEROUS)
                    .unwrap();
            assert_eq!(written, required);
            assert_eq!(encoded[0], encoding.prefix());
            assert_eq!(
                validate_base64_multibase(&encoded[..written], GENEROUS),
                Ok(encoding)
            );

            let mut decoded = [0xa5; 257];
            let result =
                decode_base64_multibase_into(&encoded[..written], &mut decoded, GENEROUS).unwrap();
            assert_eq!(result.encoding(), encoding);
            assert_eq!(result.written(), len);
            assert_eq!(&decoded[..len], &input[..len]);
            assert!(decoded[len..].iter().all(|byte| *byte == 0xa5));
        }
    }
}

#[test]
fn unsupported_prefixes_and_noncanonical_payloads_fail_closed() {
    let empty = validate_base64_multibase(b"", GENEROUS).unwrap_err();
    assert_eq!(empty.kind(), Base64MultibaseErrorKind::MissingPrefix);

    for prefix in u8::MIN..=u8::MAX {
        if !matches!(prefix, b'm' | b'M' | b'u' | b'U') {
            let error = validate_base64_multibase(&[prefix], GENEROUS).unwrap_err();
            assert_eq!(error.kind(), Base64MultibaseErrorKind::UnsupportedPrefix);
            assert_eq!(error.position(), Some(0));
            assert_eq!(error.prefix(), Some(prefix));
        }
    }

    for malformed in [
        &b"mZg=="[..],
        b"MZg",
        b"uZg==",
        b"UZg",
        b"mZh",
        b"uZh",
        b"m_w",
        b"u/w",
        b"M_w==",
        b"U/w==",
    ] {
        let mut output = [0x3c; 16];
        let before = output;
        let error = decode_base64_multibase_into(malformed, &mut output, GENEROUS).unwrap_err();
        assert_eq!(error.kind(), Base64MultibaseErrorKind::InvalidPayload);
        assert_eq!(output, before);
    }
}

#[test]
fn one_shot_limits_and_capacity_are_transactional() {
    let input = b"bounded multibase";
    let encoding = Base64MultibaseEncoding::Base64UrlPad;
    let required = base64_multibase_encoded_len(encoding, input.len()).unwrap();
    let cases = [
        (
            Base64MultibaseLimits::new(input.len() - 1, required, input.len()),
            Base64MultibaseErrorKind::InputLimitExceeded,
        ),
        (
            Base64MultibaseLimits::new(input.len(), required - 1, input.len()),
            Base64MultibaseErrorKind::OutputLimitExceeded,
        ),
        (
            Base64MultibaseLimits::new(input.len(), required, input.len() - 1),
            Base64MultibaseErrorKind::WorkLimitExceeded,
        ),
    ];
    for (limits, expected) in cases {
        let mut output = [0x6d; 128];
        let before = output;
        let error = encode_base64_multibase_into(encoding, input, &mut output, limits).unwrap_err();
        assert_eq!(error.kind(), expected);
        assert_eq!(output, before);
    }

    let mut short = [0x7e; 127];
    let before = short;
    let error = encode_base64_multibase_into(encoding, input, &mut short[..required - 1], GENEROUS)
        .unwrap_err();
    assert_eq!(error.kind(), Base64MultibaseErrorKind::OutputTooSmall);
    assert_eq!(error.required(), Some(required));
    assert_eq!(error.available(), Some(required - 1));
    assert_eq!(short, before);

    let encoded = drive_encoder(encoding, input, GENEROUS);
    let decode_limits = Base64MultibaseLimits::new(encoded.len(), input.len(), encoded.len());
    let mut decoded = [0x4a; 64];
    let result = decode_base64_multibase_into(&encoded, &mut decoded, decode_limits).unwrap();
    assert_eq!(&decoded[..result.written()], input);

    let decode_cases = [
        (
            Base64MultibaseLimits::new(encoded.len() - 1, input.len(), encoded.len()),
            Base64MultibaseErrorKind::InputLimitExceeded,
        ),
        (
            Base64MultibaseLimits::new(encoded.len(), input.len() - 1, encoded.len()),
            Base64MultibaseErrorKind::OutputLimitExceeded,
        ),
        (
            Base64MultibaseLimits::new(encoded.len(), input.len(), encoded.len() - 1),
            Base64MultibaseErrorKind::WorkLimitExceeded,
        ),
    ];
    for (limits, expected) in decode_cases {
        let mut output = [0x92; 64];
        let before = output;
        let error = decode_base64_multibase_into(&encoded, &mut output, limits).unwrap_err();
        assert_eq!(error.kind(), expected);
        assert_eq!(output, before);
    }

    let mut short_decode = [0x2d; 64];
    let before = short_decode;
    let error =
        decode_base64_multibase_into(&encoded, &mut short_decode[..input.len() - 1], GENEROUS)
            .unwrap_err();
    assert_eq!(error.kind(), Base64MultibaseErrorKind::OutputTooSmall);
    assert_eq!(error.required(), Some(input.len()));
    assert_eq!(error.available(), Some(input.len() - 1));
    assert_eq!(short_decode, before);
}

#[test]
fn incremental_states_resume_at_every_one_byte_boundary() {
    let mut input = [0u8; 193];
    fill_pattern(&mut input);
    for len in 0..=input.len() {
        for encoding in Base64MultibaseEncoding::ALL {
            let encoded = drive_encoder(encoding, &input[..len], GENEROUS);
            let decoded = drive_decoder(&encoded, GENEROUS);
            assert_eq!(decoded.0, encoding);
            assert_eq!(decoded.1, input[..len]);
        }
    }
}

#[test]
fn incremental_limit_failure_is_absorbing_and_reset_recovers() {
    let limits = Base64MultibaseLimits::new(8, 2, 8);
    let mut encoder = Base64MultibaseEncoder::new(Base64MultibaseEncoding::Base64, limits).unwrap();
    let mut output = [0u8; 2];
    let update = encoder.update(b"f", &mut output).unwrap();
    assert_eq!(update.input_consumed(), 1);
    let finish = encoder.finish(&mut output[1..]).unwrap();
    assert!(matches!(
        finish.status(),
        Base64MultibaseStatus::OutputFull(_)
    ));
    assert_eq!(
        encoder.finish(&mut []).unwrap_err().kind(),
        Base64MultibaseErrorKind::OutputLimitExceeded
    );
    assert_eq!(
        encoder.update(b"", &mut []).unwrap_err().kind(),
        Base64MultibaseErrorKind::TerminalState
    );
    encoder.reset();
    assert_eq!(encoder.source_position(), 0);

    let mut decoder = Base64MultibaseDecoder::new(Base64MultibaseLimits::new(3, 0, 3));
    let step = decoder.update(b"mZg", &mut []).unwrap();
    assert_eq!(step.input_consumed(), 3);
    let step = decoder.finish(&mut []).unwrap();
    assert!(matches!(
        step.status(),
        Base64MultibaseStatus::OutputFull(_)
    ));
    assert_eq!(
        decoder.finish(&mut []).unwrap_err().kind(),
        Base64MultibaseErrorKind::OutputLimitExceeded
    );
    decoder.clear();
    assert_eq!(decoder.encoding(), None);
}

#[cfg(feature = "alloc")]
#[test]
fn allocation_helpers_are_exact_and_bounded() {
    use base64_ng_multibase::{decode_base64_multibase_to_vec, encode_base64_multibase_to_string};

    for encoding in Base64MultibaseEncoding::ALL {
        let encoded = encode_base64_multibase_to_string(encoding, b"alloc", GENEROUS).unwrap();
        let decoded = decode_base64_multibase_to_vec(encoded.as_bytes(), GENEROUS).unwrap();
        assert_eq!(decoded.encoding(), encoding);
        assert_eq!(decoded.as_bytes(), b"alloc");
        assert_eq!(decoded.into_bytes(), b"alloc");
    }
}

fn assert_official_vectors(csv: &str, raw: &[u8]) {
    for encoding in Base64MultibaseEncoding::ALL {
        let expected = vector(csv, encoding.registry_name());
        let mut output = [0u8; 128];
        let written = encode_base64_multibase_into(encoding, raw, &mut output, GENEROUS).unwrap();
        assert_eq!(&output[..written], expected.as_bytes());
        let mut decoded = [0u8; 64];
        let result =
            decode_base64_multibase_into(expected.as_bytes(), &mut decoded, GENEROUS).unwrap();
        assert_eq!(result.encoding(), encoding);
        assert_eq!(&decoded[..result.written()], raw);
    }
}

fn vector<'a>(csv: &'a str, name: &str) -> &'a str {
    csv.lines()
        .find_map(|line| {
            let (encoding, value) = line.split_once(',')?;
            (encoding.trim() == name).then(|| value.trim().trim_matches('"'))
        })
        .unwrap()
}

fn fill_pattern(output: &mut [u8]) {
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(73).wrapping_add(29);
    }
}

fn drive_encoder(
    encoding: Base64MultibaseEncoding,
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> Vec<u8> {
    let mut state = Base64MultibaseEncoder::new(encoding, limits).unwrap();
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let mut output = [0u8; 1];
        let step = state.update(&input[offset..=offset], &mut output).unwrap();
        offset += step.input_consumed();
        result.extend_from_slice(&output[..step.output_produced()]);
    }
    loop {
        let mut output = [0u8; 1];
        let step = state.finish(&mut output).unwrap();
        result.extend_from_slice(&output[..step.output_produced()]);
        if step.status() == Base64MultibaseStatus::Complete {
            return result;
        }
    }
}

fn drive_decoder(
    input: &[u8],
    limits: Base64MultibaseLimits,
) -> (Base64MultibaseEncoding, Vec<u8>) {
    let mut state = Base64MultibaseDecoder::new(limits);
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let mut output = [0u8; 1];
        let step = state.update(&input[offset..=offset], &mut output).unwrap();
        offset += step.input_consumed();
        result.extend_from_slice(&output[..step.output_produced()]);
    }
    loop {
        let mut output = [0u8; 1];
        let step = state.finish(&mut output).unwrap();
        result.extend_from_slice(&output[..step.output_produced()]);
        if step.status() == Base64MultibaseStatus::Complete {
            return (state.encoding().unwrap(), result);
        }
    }
}
