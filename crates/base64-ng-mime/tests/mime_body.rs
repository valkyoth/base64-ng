//! RFC 2045 body conformance and state-machine integration tests.

use base64_ng_mime::{
    MimeBodyDecodePolicy, MimeBodyDecoder, MimeBodyEncoder, MimeBodyErrorKind, MimeBodyLimits,
    MimeBodyStatus, MimeBodyTerminalLineEnding, decode_mime_content_transfer_body_into,
    decode_mime_content_transfer_body_to_vec, encode_mime_content_transfer_body_into,
    encode_mime_content_transfer_body_to_string, mime_content_transfer_body_encoded_len,
};

const LIMITS: MimeBodyLimits = MimeBodyLimits::new(4096, 8192, 4096, 998, 1024, 128);

#[test]
fn normative_vectors_encode_and_decode() {
    let vectors: &[(&[u8], &str)] = &[
        (b"", ""),
        (b"f", "Zg==\r\n"),
        (b"fo", "Zm8=\r\n"),
        (b"foo", "Zm9v\r\n"),
        (b"foobar", "Zm9vYmFy\r\n"),
    ];
    for (plain, encoded) in vectors {
        let actual = encode_mime_content_transfer_body_to_string(
            plain,
            LIMITS,
            MimeBodyTerminalLineEnding::IncludeCrLf,
        )
        .unwrap();
        assert_eq!(&actual, encoded);
        let (decoded, report) = decode_mime_content_transfer_body_to_vec(
            actual.as_bytes(),
            MimeBodyDecodePolicy::Canonical,
            LIMITS,
        )
        .unwrap();
        assert_eq!(&decoded, plain);
        assert!(!report.has_transport_warning());
    }
}

#[test]
fn canonical_lines_are_76_columns_and_terminal_is_explicit() {
    let input = [0x5au8; 58];
    let encoded = encode_mime_content_transfer_body_to_string(
        &input,
        LIMITS,
        MimeBodyTerminalLineEnding::IncludeCrLf,
    )
    .unwrap();
    let lines: Vec<_> = encoded.split("\r\n").collect();
    assert_eq!(lines[0].len(), 76);
    assert_eq!(lines[1].len(), 4);
    assert_eq!(lines[2], "");

    let omitted = encode_mime_content_transfer_body_to_string(
        &input,
        LIMITS,
        MimeBodyTerminalLineEnding::Omit,
    )
    .unwrap();
    assert!(!omitted.ends_with("\r\n"));
    assert_eq!(
        mime_content_transfer_body_encoded_len(
            input.len(),
            MimeBodyTerminalLineEnding::IncludeCrLf
        )
        .unwrap(),
        encoded.len()
    );
}

#[test]
fn every_small_length_respects_line_boundaries_and_round_trips() {
    let mut input = [0u8; 256];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap()
            .wrapping_mul(73)
            .wrapping_add(19);
    }
    for length in 0..=input.len() {
        let encoded = encode_mime_content_transfer_body_to_string(
            &input[..length],
            LIMITS,
            MimeBodyTerminalLineEnding::IncludeCrLf,
        )
        .unwrap();
        if !encoded.is_empty() {
            assert!(encoded.ends_with("\r\n"));
            assert!(
                encoded[..encoded.len() - 2]
                    .split("\r\n")
                    .all(|line| !line.is_empty() && line.len() <= 76)
            );
        }
        let (decoded, _) = decode_mime_content_transfer_body_to_vec(
            encoded.as_bytes(),
            MimeBodyDecodePolicy::Canonical,
            LIMITS,
        )
        .unwrap();
        assert_eq!(decoded, &input[..length]);
    }
}

#[test]
fn compatible_decode_ignores_bounded_transport_bytes_and_reports_them() {
    let input = b"Z m\r\n9!v\tYmFy";
    let (decoded, report) = decode_mime_content_transfer_body_to_vec(
        input,
        MimeBodyDecodePolicy::Rfc2045Compatible,
        LIMITS,
    )
    .unwrap();
    assert_eq!(decoded, b"foobar");
    assert_eq!(report.skipped_nonalphabet_bytes(), 5);
    assert_eq!(report.skipped_non_whitespace_bytes(), 1);
    assert_eq!(report.bare_line_endings(), 0);
    assert!(report.has_transport_warning());
}

#[test]
fn compatible_decode_accepts_bare_line_endings_but_canonical_rejects_them() {
    let input = b"Zm9v\nYmFy";
    let (decoded, report) = decode_mime_content_transfer_body_to_vec(
        input,
        MimeBodyDecodePolicy::Rfc2045Compatible,
        LIMITS,
    )
    .unwrap();
    assert_eq!(decoded, b"foobar");
    assert_eq!(report.bare_line_endings(), 1);

    let error =
        decode_mime_content_transfer_body_to_vec(input, MimeBodyDecodePolicy::Canonical, LIMITS)
            .unwrap_err();
    assert_eq!(error.kind(), MimeBodyErrorKind::InvalidCanonicalLayout);
}

#[test]
fn canonical_rejects_short_interior_lines_and_malformed_folding() {
    for input in [
        b"Zm9v\r\nYmFy".as_slice(),
        b"Zm9v\rX\nYmFy".as_slice(),
        b"Zm9v\r\n\r\nYmFy".as_slice(),
    ] {
        let error = decode_mime_content_transfer_body_to_vec(
            input,
            MimeBodyDecodePolicy::Canonical,
            LIMITS,
        )
        .unwrap_err();
        assert_eq!(error.kind(), MimeBodyErrorKind::InvalidCanonicalLayout);
    }
}

#[test]
fn canonical_padding_and_trailing_bits_remain_strict() {
    for input in [
        b"Zg=".as_slice(),
        b"Zh==".as_slice(),
        b"Zg==QQ==".as_slice(),
    ] {
        let error = decode_mime_content_transfer_body_to_vec(
            input,
            MimeBodyDecodePolicy::Rfc2045Compatible,
            LIMITS,
        )
        .unwrap_err();
        assert_eq!(error.kind(), MimeBodyErrorKind::InvalidBase64);
    }
}

#[test]
fn every_limit_fails_closed() {
    let tiny_input = MimeBodyLimits::new(3, 32, 32, 76, 8, 16);
    assert_eq!(
        encode_mime_content_transfer_body_to_string(
            b"four",
            tiny_input,
            MimeBodyTerminalLineEnding::Omit
        )
        .unwrap_err()
        .kind(),
        MimeBodyErrorKind::InputLimitExceeded
    );

    let skip_limit = MimeBodyLimits::new(32, 32, 32, 76, 1, 16);
    assert_eq!(
        decode_mime_content_transfer_body_to_vec(
            b"Z  g==",
            MimeBodyDecodePolicy::Rfc2045Compatible,
            skip_limit
        )
        .unwrap_err()
        .kind(),
        MimeBodyErrorKind::SkippedNonalphabetLimitExceeded
    );

    let work_limit = MimeBodyLimits::new(32, 32, 32, 76, 16, 3);
    assert_eq!(
        decode_mime_content_transfer_body_to_vec(
            b"   Zg==",
            MimeBodyDecodePolicy::Rfc2045Compatible,
            work_limit
        )
        .unwrap_err()
        .kind(),
        MimeBodyErrorKind::WorkBeforeOutputLimitExceeded
    );
}

#[test]
fn one_shot_destinations_are_transactional() {
    let mut encoded = [0xa5; 3];
    let error = encode_mime_content_transfer_body_into(
        b"hello",
        &mut encoded,
        LIMITS,
        MimeBodyTerminalLineEnding::Omit,
    )
    .unwrap_err();
    assert_eq!(error.kind(), MimeBodyErrorKind::OutputTooSmall);
    assert_eq!(encoded, [0xa5; 3]);

    let mut decoded = [0xa5; 8];
    let error = decode_mime_content_transfer_body_into(
        b"Zm9v!===",
        &mut decoded,
        MimeBodyDecodePolicy::Canonical,
        LIMITS,
    )
    .unwrap_err();
    assert_eq!(error.kind(), MimeBodyErrorKind::InvalidCanonicalLayout);
    assert_eq!(decoded, [0xa5; 8]);
}

#[test]
fn incremental_states_resume_across_every_byte_boundary() {
    let plain = b"fragmented MIME body transport";
    let expected = encode_mime_content_transfer_body_to_string(
        plain,
        LIMITS,
        MimeBodyTerminalLineEnding::IncludeCrLf,
    )
    .unwrap();

    let mut encode_state = MimeBodyEncoder::new(LIMITS, MimeBodyTerminalLineEnding::IncludeCrLf);
    let mut encoded_body = Vec::new();
    for byte in plain {
        drive_encoder_update(
            &mut encode_state,
            core::slice::from_ref(byte),
            &mut encoded_body,
        );
    }
    drive_encoder_finish(&mut encode_state, &mut encoded_body);
    assert_eq!(encoded_body, expected.as_bytes());

    let mut decode_state = MimeBodyDecoder::new(MimeBodyDecodePolicy::Canonical, LIMITS);
    let mut plain_output = Vec::new();
    for byte in &encoded_body {
        drive_decoder_update(
            &mut decode_state,
            core::slice::from_ref(byte),
            &mut plain_output,
        );
    }
    drive_decoder_finish(&mut decode_state, &mut plain_output);
    assert_eq!(plain_output, plain);
}

fn drive_encoder_update(encoder: &mut MimeBodyEncoder, mut input: &[u8], output: &mut Vec<u8>) {
    while !input.is_empty() {
        let mut byte = [0u8; 1];
        let step = encoder.update(input, &mut byte).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_produced()]);
        input = &input[step.progress().input_consumed()..];
    }
}

fn drive_encoder_finish(encoder: &mut MimeBodyEncoder, output: &mut Vec<u8>) {
    loop {
        let mut byte = [0u8; 1];
        let step = encoder.finish(&mut byte).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_produced()]);
        if step.status() == MimeBodyStatus::Complete {
            break;
        }
    }
}

fn drive_decoder_update(decoder: &mut MimeBodyDecoder, mut input: &[u8], output: &mut Vec<u8>) {
    while !input.is_empty() {
        let mut byte = [0u8; 1];
        let step = decoder.update(input, &mut byte).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_produced()]);
        input = &input[step.progress().input_consumed()..];
    }
}

fn drive_decoder_finish(decoder: &mut MimeBodyDecoder, output: &mut Vec<u8>) {
    loop {
        let mut byte = [0u8; 1];
        let step = decoder.finish(&mut byte).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_produced()]);
        if step.status() == MimeBodyStatus::Complete {
            break;
        }
    }
}
