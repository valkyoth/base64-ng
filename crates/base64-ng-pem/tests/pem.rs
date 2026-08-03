//! RFC 7468 grammar, boundary, resource, and secret-release tests.

use base64_ng_pem::{
    PemBlockEncoder, PemDocumentParser, PemErrorKind, PemGenerationOptions, PemLabel, PemLimits,
    PemLineEnding, PemParsePolicy, PemParseReport, encode_pem_block_into,
    encode_pem_block_to_string, parse_pem_document,
};

const LIMITS: PemLimits = PemLimits::new(65_536, 65_536, 32_768, 4096, 128, 16, 4096, 65_536);

#[test]
fn labels_follow_rfc_syntax_and_generation_requires_uppercase() {
    for valid in ["", "CERTIFICATE", "PRIVATE KEY", "X.509 CERTIFICATE", "a"] {
        assert!(PemLabel::new(valid).is_ok(), "{valid:?}");
    }
    for invalid in [" KEY", "KEY ", "A  B", "A--B", "A -B", "A- B", "A\nB"] {
        assert!(PemLabel::new(invalid).is_err(), "{invalid:?}");
    }
    let lowercase = PemLabel::new("certificate").unwrap();
    assert_eq!(
        encode_pem_block_to_string(&lowercase, b"x", LIMITS, PemGenerationOptions::default())
            .unwrap_err()
            .kind(),
        PemErrorKind::NonCanonicalLabel
    );
}

#[test]
fn canonical_generation_and_strict_parse_round_trip_boundaries() {
    let label = PemLabel::new("PUBLIC KEY").unwrap();
    for len in 0..=193 {
        let payload: Vec<u8> = (0..len)
            .scan(19u8, |state, _| {
                let value = *state;
                *state = state.wrapping_add(73);
                Some(value)
            })
            .collect();
        let encoded =
            encode_pem_block_to_string(&label, &payload, LIMITS, PemGenerationOptions::default())
                .unwrap();
        assert!(encoded.starts_with("-----BEGIN PUBLIC KEY-----\r\n"));
        assert!(encoded.ends_with("-----END PUBLIC KEY-----\r\n"));
        let document =
            parse_pem_document(encoded.as_bytes(), LIMITS, PemParsePolicy::Strict).unwrap();
        assert_eq!(document.blocks().len(), 1);
        assert_eq!(document.blocks()[0].contents(), payload);
        assert_eq!(document.report(), PemParseReport::default());
    }
}

#[test]
fn lf_generation_is_explicitly_compatible_not_strict() {
    let label = PemLabel::new("CERTIFICATE").unwrap();
    let encoded = encode_pem_block_to_string(
        &label,
        b"abc",
        LIMITS,
        PemGenerationOptions::new(PemLineEnding::Lf, true),
    )
    .unwrap();
    assert_eq!(
        parse_pem_document(encoded.as_bytes(), LIMITS, PemParsePolicy::Strict)
            .unwrap_err()
            .kind(),
        PemErrorKind::NonCanonicalLayout
    );
    let document = parse_pem_document(
        encoded.as_bytes(),
        LIMITS,
        PemParsePolicy::Rfc7468Compatible,
    )
    .unwrap();
    assert!(document.report().non_crlf_line_endings >= 2);
}

#[test]
fn omitted_terminal_ending_uses_standard_not_strict_grammar() {
    let encoded = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        b"abc",
        LIMITS,
        PemGenerationOptions::new(PemLineEnding::CrLf, false),
    )
    .unwrap();
    assert_eq!(
        parse_pem_document(encoded.as_bytes(), LIMITS, PemParsePolicy::Strict)
            .unwrap_err()
            .kind(),
        PemErrorKind::NonCanonicalLayout
    );
    assert!(
        parse_pem_document(
            encoded.as_bytes(),
            LIMITS,
            PemParsePolicy::Rfc7468Compatible,
        )
        .is_ok()
    );
}

#[test]
fn compatible_policy_reports_surrounding_text_layout_skips_and_mismatch() {
    let input = b"subject text\n\t-----BEGIN certificate----- \n Zm 9v !!\nYmFy\n-----END OTHER-----\ntrailing";
    let document = parse_pem_document(input, LIMITS, PemParsePolicy::Rfc7468Compatible).unwrap();
    assert_eq!(document.blocks()[0].contents(), b"foobar");
    let report = document.report();
    assert!(report.adjacent_text_bytes > 0);
    assert!(report.skipped_body_bytes > 0);
    assert!(report.noncanonical_body_lines > 0);
    assert!(report.non_crlf_line_endings > 0);
    assert_eq!(report.mismatched_end_labels, 1);
    assert!(report.noncanonical_labels > 0);
}

#[test]
fn compatible_policy_reports_boundary_blanks() {
    let input = b" \t-----BEGIN CERTIFICATE----- \r\nZg==\r\n\t-----END CERTIFICATE----- \t";
    let document = parse_pem_document(input, LIMITS, PemParsePolicy::Rfc7468Compatible).unwrap();
    assert_eq!(document.blocks()[0].contents(), b"f");
    assert_eq!(
        document.report(),
        PemParseReport {
            noncanonical_boundary_lines: 2,
            ..PemParseReport::default()
        }
    );
}

#[test]
fn newline_dense_input_does_not_require_a_document_wide_line_index() {
    let input = vec![b'\n'; 65_536];
    let limits = PemLimits::new(65_536, 65_536, 65_536, 1, 128, 1, 65_536, 65_536);
    assert_eq!(
        parse_pem_document(&input, limits, PemParsePolicy::Rfc7468Compatible)
            .unwrap_err()
            .kind(),
        PemErrorKind::BeginBoundaryMissing
    );
}

#[test]
fn strict_policy_rejects_mismatch_padding_layout_and_legacy_headers() {
    let cases = [
        (
            b"-----BEGIN A-----\r\nZg==\r\n-----END B-----\r\n".as_slice(),
            PemErrorKind::MismatchedEndLabel,
        ),
        (
            b"-----BEGIN A-----\r\nZh==\r\n-----END A-----\r\n".as_slice(),
            PemErrorKind::InvalidBody,
        ),
        (
            b"-----BEGIN A-----\r\nProc-Type: 4,ENCRYPTED\r\nZg==\r\n-----END A-----\r\n"
                .as_slice(),
            PemErrorKind::LegacyHeadersNotSupported,
        ),
        (
            b"-----BEGIN A-----\r\nZg=\r\n-----END A-----\r\n".as_slice(),
            PemErrorKind::InvalidBody,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_pem_document(input, LIMITS, PemParsePolicy::Strict)
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn multiple_blocks_preserve_order() {
    let first = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        b"one",
        LIMITS,
        PemGenerationOptions::default(),
    )
    .unwrap();
    let second = encode_pem_block_to_string(
        &PemLabel::new("PUBLIC KEY").unwrap(),
        b"two",
        LIMITS,
        PemGenerationOptions::default(),
    )
    .unwrap();
    let input = format!("prefix\r\n{first}between\r\n{second}suffix");
    let parsed = parse_pem_document(input.as_bytes(), LIMITS, PemParsePolicy::Strict).unwrap();
    assert_eq!(parsed.blocks()[0].label().as_str(), "CERTIFICATE");
    assert_eq!(parsed.blocks()[0].contents(), b"one");
    assert_eq!(parsed.blocks()[1].label().as_str(), "PUBLIC KEY");
    assert_eq!(parsed.blocks()[1].contents(), b"two");
    assert!(parsed.report().adjacent_text_bytes > 0);
}

#[test]
fn generation_is_transactional_on_every_preflight_error() {
    let label = PemLabel::new("CERTIFICATE").unwrap();
    let mut output = [0xa5; 128];
    let before = output;
    let error = encode_pem_block_into(
        &label,
        b"payload",
        &mut output,
        PemLimits::new(10, 10, 10, 80, 64, 1, 0, 10),
        PemGenerationOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), PemErrorKind::EncodedOutputLimitExceeded);
    assert_eq!(output, before);
}

#[test]
fn generation_checks_the_longest_line_actually_emitted() {
    let label = PemLabel::new("A").unwrap();
    let limits = PemLimits::new(128, 128, 8, 17, 1, 1, 0, 128);
    let encoded =
        encode_pem_block_to_string(&label, b"payload", limits, PemGenerationOptions::default())
            .unwrap();
    assert_eq!(
        parse_pem_document(encoded.as_bytes(), limits, PemParsePolicy::Strict)
            .unwrap()
            .blocks()[0]
            .contents(),
        b"payload"
    );

    let too_narrow = PemLimits::new(256, 256, 49, 63, 1, 1, 0, 256);
    assert_eq!(
        encode_pem_block_to_string(
            &label,
            &[0u8; 49],
            too_narrow,
            PemGenerationOptions::default(),
        )
        .unwrap_err()
        .kind(),
        PemErrorKind::PhysicalLineTooLong
    );
}

#[test]
fn every_limit_has_a_deterministic_failure() {
    let document = b"-----BEGIN CERTIFICATE-----\r\nZg==\r\n-----END CERTIFICATE-----\r\n";
    let cases = [
        (
            PemLimits::new(1, 1000, 1000, 1000, 100, 10, 1000, 1000),
            PemErrorKind::InputLimitExceeded,
        ),
        (
            PemLimits::new(1000, 1000, 0, 1000, 100, 10, 1000, 1000),
            PemErrorKind::DecodedOutputLimitExceeded,
        ),
        (
            PemLimits::new(1000, 1000, 1000, 4, 100, 10, 1000, 1000),
            PemErrorKind::PhysicalLineTooLong,
        ),
        (
            PemLimits::new(1000, 1000, 1000, 1000, 1, 10, 1000, 1000),
            PemErrorKind::LabelLimitExceeded,
        ),
        (
            PemLimits::new(1000, 1000, 1000, 1000, 100, 0, 1000, 1000),
            PemErrorKind::BlockLimitExceeded,
        ),
        (
            PemLimits::new(1000, 1000, 1000, 1000, 100, 10, 1000, 1),
            PemErrorKind::WorkLimitExceeded,
        ),
    ];
    for (limits, expected) in cases {
        assert_eq!(
            parse_pem_document(document, limits, PemParsePolicy::Strict)
                .unwrap_err()
                .kind(),
            expected
        );
    }
}

#[test]
fn incremental_collection_is_chunk_boundary_independent_and_terminal_on_limit() {
    let encoded = encode_pem_block_to_string(
        &PemLabel::new("PRIVATE KEY").unwrap(),
        b"secret-ish",
        LIMITS,
        PemGenerationOptions::default(),
    )
    .unwrap();
    for chunk in 1..=encoded.len() {
        let mut parser = PemDocumentParser::new(LIMITS, PemParsePolicy::Strict);
        for bytes in encoded.as_bytes().chunks(chunk) {
            parser.update(bytes).unwrap();
        }
        assert_eq!(
            parser.finish().unwrap().blocks()[0].contents(),
            b"secret-ish"
        );
    }

    for chunk in 1..=11 {
        let mut collector = PemBlockEncoder::new(
            PemLabel::new("PRIVATE KEY").unwrap(),
            LIMITS,
            PemGenerationOptions::default(),
        )
        .unwrap();
        for bytes in b"secret-ish".chunks(chunk) {
            collector.update(bytes).unwrap();
        }
        assert_eq!(collector.finish_to_string().unwrap(), encoded);
    }

    let mut parser = PemDocumentParser::new(
        PemLimits::new(1, 10, 10, 10, 10, 1, 1, 1),
        PemParsePolicy::Strict,
    );
    assert_eq!(
        parser.update(b"too long").unwrap_err().kind(),
        PemErrorKind::InputLimitExceeded
    );
    assert_eq!(
        parser.update(b"x").unwrap_err().kind(),
        PemErrorKind::TerminalState
    );
}

#[cfg(feature = "secrets")]
#[test]
fn secret_parser_requires_strict_single_expected_label_and_redacts() {
    use base64_ng_pem::parse_pem_secret_block;

    let label = PemLabel::new("PRIVATE KEY").unwrap();
    let encoded = encode_pem_block_to_string(
        &label,
        b"classified",
        LIMITS,
        PemGenerationOptions::default(),
    )
    .unwrap();
    let secret =
        parse_pem_secret_block(encoded.as_bytes(), &label, LIMITS, PemParsePolicy::Strict).unwrap();
    assert_eq!(secret.contents().expose_secret().as_ref(), b"classified");
    let debug = format!("{secret:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("classified"));
    assert_eq!(
        parse_pem_secret_block(
            encoded.as_bytes(),
            &PemLabel::new("PUBLIC KEY").unwrap(),
            LIMITS,
            PemParsePolicy::Strict,
        )
        .unwrap_err()
        .kind(),
        PemErrorKind::SecretBlockSelection
    );
    let malformed = encoded.replace("Y2xhc3NpZmllZA==", "Y2xhc3NpZmllZ!==");
    assert_eq!(
        parse_pem_secret_block(malformed.as_bytes(), &label, LIMITS, PemParsePolicy::Strict,)
            .unwrap_err()
            .kind(),
        PemErrorKind::InvalidBody
    );
}
