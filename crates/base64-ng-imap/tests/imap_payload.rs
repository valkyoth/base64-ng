//! RFC 3501 payload conformance, limits, and state-machine tests.

use base64_ng_imap::{
    ImapPayloadErrorKind, ImapPayloadLimits, ImapPayloadStatus, ModifiedUtf7PayloadDecoder,
    ModifiedUtf7PayloadEncoder, decode_modified_utf7_payload_into,
    decode_modified_utf7_payload_to_vec, encode_modified_utf7_payload_into,
    encode_modified_utf7_payload_to_string, modified_utf7_payload_decoded_len,
    modified_utf7_payload_encoded_len, validate_modified_utf7_payload,
};

const LIMITS: ImapPayloadLimits = ImapPayloadLimits::new(4_096, 4_096, 4_096);

#[test]
fn rfc_3501_payload_vectors_are_exact() {
    let vectors: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (&[0x53, 0xf0, 0x53, 0x17], b"U,BTFw"),
        (&[0x65, 0xe5, 0x67, 0x2c, 0x8a, 0x9e], b"ZeVnLIqe"),
        (&[0x00, 0xe9], b"AOk"),
        (&[0xd8, 0x3d, 0xde, 0x00], b"2D3eAA"),
    ];
    for (plain, encoded) in vectors {
        let mut output = [0u8; 32];
        let written = encode_modified_utf7_payload_into(plain, &mut output, LIMITS).unwrap();
        assert_eq!(&output[..written], *encoded);
        assert_eq!(
            modified_utf7_payload_encoded_len(plain.len()),
            Ok(encoded.len())
        );

        let mut decoded = [0u8; 32];
        let written = decode_modified_utf7_payload_into(encoded, &mut decoded, LIMITS).unwrap();
        assert_eq!(&decoded[..written], *plain);
        assert_eq!(
            modified_utf7_payload_decoded_len(encoded, LIMITS),
            Ok(plain.len())
        );
    }
}

#[test]
fn every_small_even_utf16be_storage_length_round_trips() {
    let mut input = [0u8; 258];
    fill_pattern(&mut input);
    for length in (0..=input.len()).step_by(2) {
        let mut encoded = [0xa5; 400];
        let written =
            encode_modified_utf7_payload_into(&input[..length], &mut encoded, LIMITS).unwrap();
        assert!(
            encoded[..written]
                .iter()
                .all(|byte| { byte.is_ascii_alphanumeric() || matches!(*byte, b'+' | b',') })
        );
        assert!(!encoded[..written].contains(&b'='));
        let mut decoded = [0x5a; 258];
        let decoded_len =
            decode_modified_utf7_payload_into(&encoded[..written], &mut decoded, LIMITS).unwrap();
        assert_eq!(decoded_len, length);
        assert_eq!(&decoded[..length], &input[..length]);
        assert!(decoded[length..].iter().all(|byte| *byte == 0x5a));
    }
}

#[test]
fn malformed_padding_alphabet_tails_and_odd_storage_are_rejected() {
    for malformed in [b"AAA=".as_slice(), b"AAA/", b"AAA-", b"AA A", b"A", b"AB"] {
        let error = validate_modified_utf7_payload(malformed, LIMITS).unwrap_err();
        assert_eq!(error.kind(), ImapPayloadErrorKind::InvalidPayload);
    }

    assert_eq!(
        validate_modified_utf7_payload(b"AA", LIMITS)
            .unwrap_err()
            .kind(),
        ImapPayloadErrorKind::InvalidUtf16BeLength
    );
    assert_eq!(
        modified_utf7_payload_encoded_len(3).unwrap_err().kind(),
        ImapPayloadErrorKind::InvalidUtf16BeLength
    );
}

#[test]
fn one_shot_errors_are_transactional_and_limits_are_distinct() {
    let plain = [0x53, 0xf0, 0x53, 0x17];
    let required = modified_utf7_payload_encoded_len(plain.len()).unwrap();
    let cases = [
        (
            ImapPayloadLimits::new(plain.len() - 1, required, plain.len()),
            ImapPayloadErrorKind::InputLimitExceeded,
        ),
        (
            ImapPayloadLimits::new(plain.len(), required - 1, plain.len()),
            ImapPayloadErrorKind::OutputLimitExceeded,
        ),
        (
            ImapPayloadLimits::new(plain.len(), required, plain.len() - 1),
            ImapPayloadErrorKind::WorkLimitExceeded,
        ),
    ];
    for (limits, kind) in cases {
        let mut output = [0xa5; 32];
        let before = output;
        let error = encode_modified_utf7_payload_into(&plain, &mut output, limits).unwrap_err();
        assert_eq!(error.kind(), kind);
        assert_eq!(output, before);
    }

    let mut short = [0x6d; 5];
    let before = short;
    let error = encode_modified_utf7_payload_into(&plain, &mut short, LIMITS).unwrap_err();
    assert_eq!(error.kind(), ImapPayloadErrorKind::OutputTooSmall);
    assert_eq!(error.required(), Some(required));
    assert_eq!(error.available(), Some(short.len()));
    assert_eq!(short, before);

    let mut decoded = [0x77; 16];
    let before = decoded;
    let error = decode_modified_utf7_payload_into(b"AA", &mut decoded, LIMITS).unwrap_err();
    assert_eq!(error.kind(), ImapPayloadErrorKind::InvalidUtf16BeLength);
    assert_eq!(decoded, before);
}

#[test]
fn incremental_states_resume_with_one_byte_fragments() {
    let mut input = [0u8; 130];
    fill_pattern(&mut input);
    for length in (0..=input.len()).step_by(2) {
        let encoded = drive_encoder(&input[..length], LIMITS);
        let decoded = drive_decoder(&encoded, LIMITS).unwrap();
        assert_eq!(decoded, input[..length]);
    }
}

#[test]
fn incremental_odd_length_and_malformed_tail_latch_until_reset() {
    let mut encoder = ModifiedUtf7PayloadEncoder::new(LIMITS);
    let mut output = [0u8; 8];
    let step = encoder.update(&[0x53], &mut output).unwrap();
    assert_eq!(step.input_consumed(), 1);
    assert_eq!(
        encoder.finish(&mut output).unwrap_err().kind(),
        ImapPayloadErrorKind::InvalidUtf16BeLength
    );
    assert_eq!(
        encoder.update(&[], &mut []).unwrap_err().kind(),
        ImapPayloadErrorKind::TerminalState
    );
    encoder.clear();
    assert_eq!(encoder.source_position(), 0);

    let mut decoder = ModifiedUtf7PayloadDecoder::new(LIMITS);
    let step = decoder.update(b"AA", &mut output).unwrap();
    assert_eq!(step.input_consumed(), 2);
    assert_eq!(
        decoder.finish(&mut output).unwrap_err().kind(),
        ImapPayloadErrorKind::InvalidUtf16BeLength
    );
    assert_eq!(
        decoder.finish(&mut output).unwrap_err().kind(),
        ImapPayloadErrorKind::TerminalState
    );
    decoder.reset();
    assert_eq!(decoder.source_position(), 0);
}

#[test]
fn allocation_helpers_validate_then_allocate_exact_outputs() {
    let bytes = [0x53, 0xf0, 0x53, 0x17];
    let encoded = encode_modified_utf7_payload_to_string(&bytes, LIMITS).unwrap();
    assert_eq!(encoded, "U,BTFw");
    let decoded = decode_modified_utf7_payload_to_vec(encoded.as_bytes(), LIMITS).unwrap();
    assert_eq!(decoded, bytes);
}

fn drive_encoder(input: &[u8], limits: ImapPayloadLimits) -> Vec<u8> {
    let mut state = ModifiedUtf7PayloadEncoder::new(limits);
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
        if step.status() == ImapPayloadStatus::Complete {
            return result;
        }
    }
}

fn drive_decoder(input: &[u8], limits: ImapPayloadLimits) -> Result<Vec<u8>, ImapPayloadErrorKind> {
    let mut state = ModifiedUtf7PayloadDecoder::new(limits);
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let mut output = [0u8; 1];
        let step = state
            .update(&input[offset..=offset], &mut output)
            .map_err(base64_ng_imap::ImapPayloadError::kind)?;
        offset += step.input_consumed();
        result.extend_from_slice(&output[..step.output_produced()]);
    }
    loop {
        let mut output = [0u8; 1];
        let step = state
            .finish(&mut output)
            .map_err(base64_ng_imap::ImapPayloadError::kind)?;
        result.extend_from_slice(&output[..step.output_produced()]);
        if step.status() == ImapPayloadStatus::Complete {
            return Ok(result);
        }
    }
}

fn fill_pattern(output: &mut [u8]) {
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(73).wrapping_add(29);
    }
}
