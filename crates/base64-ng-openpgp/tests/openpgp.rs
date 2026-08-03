#![allow(missing_docs)]

use base64_ng_openpgp::{
    ArmorDocumentParser, ArmorEncoder, ArmorHeader, ArmorType, ChecksumGeneration, ChecksumPolicy,
    ChecksumStatus, GenerationOptions, LineEnding, OpenPgpErrorKind, OpenPgpLimits,
    armor_encoded_len, encode_armor_into, encode_armor_to_string, parse_armor_document,
};

const RFC9580_INLINE_SIGNED: &[u8] = include_bytes!("fixtures/rfc9580-inline-signed.asc");

fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| (index * 73 + 19).to_le_bytes()[0])
        .collect()
}

#[test]
fn every_ordinary_type_round_trips_with_both_checksum_modes() {
    let kinds = [
        ArmorType::Message,
        ArmorType::PublicKey,
        ArmorType::PrivateKey,
        ArmorType::Signature,
    ];
    let headers = [
        ArmorHeader::new("Comment", "bounded test: value").unwrap(),
        ArmorHeader::new("X-Unknown", "preserved").unwrap(),
    ];
    for kind in kinds {
        for len in 0..=193 {
            let payload = pattern(len);
            for checksum in [ChecksumGeneration::Omit, ChecksumGeneration::LegacyCrc24] {
                let options = GenerationOptions::new(checksum).with_line_ending(LineEnding::Lf);
                let text = encode_armor_to_string(
                    kind,
                    &headers,
                    &payload,
                    OpenPgpLimits::default(),
                    options,
                )
                .unwrap();
                assert_eq!(
                    text.len(),
                    armor_encoded_len(kind, &headers, len, options).unwrap()
                );
                let document = parse_armor_document(
                    text.as_bytes(),
                    OpenPgpLimits::default(),
                    ChecksumPolicy::Rfc9580,
                )
                .unwrap();
                let block = &document.blocks()[0];
                assert_eq!(block.kind(), kind);
                assert_eq!(block.headers(), headers);
                assert_eq!(block.contents(), payload);
                assert_eq!(
                    block.checksum_status(),
                    if checksum == ChecksumGeneration::Omit {
                        ChecksumStatus::Absent
                    } else {
                        ChecksumStatus::Valid
                    }
                );
            }
        }
    }
}

#[test]
fn rfc9580_checksum_policy_reports_defects_while_strict_policy_rejects() {
    let options =
        GenerationOptions::new(ChecksumGeneration::LegacyCrc24).with_line_ending(LineEnding::Lf);
    let valid = encode_armor_to_string(
        ArmorType::Message,
        &[],
        b"checksum payload",
        OpenPgpLimits::default(),
        options,
    )
    .unwrap();

    let footer = valid.lines().find(|line| line.starts_with('=')).unwrap();
    let mismatch = valid.replacen(footer, "=AAAA", 1);
    let parsed = parse_armor_document(
        mismatch.as_bytes(),
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    assert_eq!(
        parsed.blocks()[0].checksum_status(),
        ChecksumStatus::Mismatch
    );
    assert_eq!(
        parse_armor_document(
            mismatch.as_bytes(),
            OpenPgpLimits::default(),
            ChecksumPolicy::RequireValidCrc24,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::ChecksumMismatch
    );

    let malformed = valid.replacen(footer, "=bad", 1);
    let parsed = parse_armor_document(
        malformed.as_bytes(),
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    assert_eq!(
        parsed.blocks()[0].checksum_status(),
        ChecksumStatus::Malformed
    );
    assert_eq!(
        parse_armor_document(
            malformed.as_bytes(),
            OpenPgpLimits::default(),
            ChecksumPolicy::RequireValidCrc24,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::ChecksumMalformed
    );

    let absent = encode_armor_to_string(
        ArmorType::Message,
        &[],
        b"checksum payload",
        OpenPgpLimits::default(),
        GenerationOptions::default(),
    )
    .unwrap();
    assert_eq!(
        parse_armor_document(
            absent.as_bytes(),
            OpenPgpLimits::default(),
            ChecksumPolicy::RequireValidCrc24,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::ChecksumMissing
    );
}

#[test]
fn official_rfc9580_inline_signed_example_parses() {
    let document = parse_armor_document(
        RFC9580_INLINE_SIGNED,
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    assert_eq!(document.blocks().len(), 1);
    assert_eq!(document.blocks()[0].kind(), ArmorType::Message);
    assert_eq!(document.blocks()[0].contents().len(), 302);
    assert_eq!(
        document.blocks()[0].checksum_status(),
        ChecksumStatus::Absent
    );
}

#[test]
fn incremental_parser_and_encoder_handle_every_partition() {
    let payload = pattern(257);
    let expected = encode_armor_to_string(
        ArmorType::PublicKey,
        &[],
        &payload,
        OpenPgpLimits::default(),
        GenerationOptions::default().with_line_ending(LineEnding::Lf),
    )
    .unwrap();
    for split in 0..=expected.len() {
        let mut parser =
            ArmorDocumentParser::new(OpenPgpLimits::default(), ChecksumPolicy::Rfc9580);
        parser.update(&expected.as_bytes()[..split]).unwrap();
        parser.update(&expected.as_bytes()[split..]).unwrap();
        assert_eq!(parser.finish().unwrap().blocks()[0].contents(), payload);
    }
    let mut encoder = ArmorEncoder::new(
        ArmorType::PublicKey,
        &[],
        OpenPgpLimits::default(),
        GenerationOptions::default().with_line_ending(LineEnding::Lf),
    )
    .unwrap();
    for byte in &payload {
        encoder.update(core::slice::from_ref(byte)).unwrap();
    }
    assert_eq!(encoder.finish_to_string().unwrap(), expected);
}

#[test]
fn multiple_blocks_allow_only_bounded_adjacent_whitespace() {
    let first = encode_armor_to_string(
        ArmorType::Message,
        &[],
        b"first",
        OpenPgpLimits::default(),
        GenerationOptions::default().with_line_ending(LineEnding::Lf),
    )
    .unwrap();
    let second = encode_armor_to_string(
        ArmorType::Signature,
        &[],
        b"second",
        OpenPgpLimits::default(),
        GenerationOptions::default().with_line_ending(LineEnding::Lf),
    )
    .unwrap();
    let document = format!(" \t\n{first}\n\t\n{second} \n");
    let parsed = parse_armor_document(
        document.as_bytes(),
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    assert_eq!(parsed.blocks().len(), 2);
    assert_eq!(parsed.blocks()[0].contents(), b"first");
    assert_eq!(parsed.blocks()[1].contents(), b"second");
    assert!(parsed.adjacent_whitespace_bytes() > 0);
}

#[test]
fn complete_framing_and_headers_fail_closed() {
    let cases = [
        (
            b"-----BEGIN PGP MESSAGE-----\nComment: x\nYWJj\n-----END PGP MESSAGE-----\n"
                .as_slice(),
            OpenPgpErrorKind::InvalidHeader,
        ),
        (
            b"-----BEGIN PGP MESSAGE-----\nBad:x\n\nYWJj\n-----END PGP MESSAGE-----\n".as_slice(),
            OpenPgpErrorKind::InvalidHeader,
        ),
        (
            b"-----BEGIN PGP MESSAGE-----\n\nYWJj\n-----END PGP SIGNATURE-----\n".as_slice(),
            OpenPgpErrorKind::MismatchedEndBoundary,
        ),
        (
            b"prefix\n-----BEGIN PGP MESSAGE-----\n\nYWJj\n-----END PGP MESSAGE-----\n".as_slice(),
            OpenPgpErrorKind::TrailingAmbiguity,
        ),
        (
            b"-----BEGIN PGP SIGNED MESSAGE-----\n\nYWJj\n-----END PGP SIGNED MESSAGE-----\n"
                .as_slice(),
            OpenPgpErrorKind::InvalidBoundary,
        ),
    ];
    for (input, kind) in cases {
        assert_eq!(
            parse_armor_document(input, OpenPgpLimits::default(), ChecksumPolicy::Rfc9580)
                .unwrap_err()
                .kind(),
            kind
        );
    }
}

#[test]
fn generation_is_transactional_and_debug_redacts_payloads() {
    let payload = b"CANARY-SECRET-PAYLOAD";
    let mut output = [0xa5; 8];
    let error = encode_armor_into(
        ArmorType::PrivateKey,
        &[],
        payload,
        &mut output,
        OpenPgpLimits::default(),
        GenerationOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), OpenPgpErrorKind::OutputTooSmall);
    assert_eq!(output, [0xa5; 8]);

    let text = encode_armor_to_string(
        ArmorType::PrivateKey,
        &[],
        payload,
        OpenPgpLimits::default(),
        GenerationOptions::default(),
    )
    .unwrap();
    let document = parse_armor_document(
        text.as_bytes(),
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    let debug = format!("{document:?} {:?}", document.blocks()[0]);
    assert!(!debug.contains("CANARY"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_resource_dimension_is_enforced() {
    let base = b"-----BEGIN PGP MESSAGE-----\n\nYWJj\n-----END PGP MESSAGE-----\n";
    let limited =
        |input, encoded, decoded, line, headers, header_bytes, label, blocks, adjacent, work| {
            OpenPgpLimits::new(
                input,
                encoded,
                decoded,
                line,
                headers,
                header_bytes,
                label,
                blocks,
                adjacent,
                work,
            )
        };
    let max = usize::MAX;
    let cases = [
        (
            limited(1, max, max, max, max, max, max, max, max, max),
            OpenPgpErrorKind::InputLimitExceeded,
        ),
        (
            limited(max, max, max, 10, max, max, max, max, max, max),
            OpenPgpErrorKind::PhysicalLineTooLong,
        ),
        (
            limited(max, max, max, max, max, max, 3, max, max, max),
            OpenPgpErrorKind::LabelLimitExceeded,
        ),
        (
            limited(max, max, max, max, max, max, max, 0, max, max),
            OpenPgpErrorKind::BlockLimitExceeded,
        ),
        (
            limited(max, max, max, max, max, max, max, max, max, 1),
            OpenPgpErrorKind::WorkLimitExceeded,
        ),
        (
            limited(max, max, 2, max, max, max, max, max, max, max),
            OpenPgpErrorKind::DecodedOutputLimitExceeded,
        ),
    ];
    for (limits, expected) in cases {
        assert_eq!(
            parse_armor_document(base, limits, ChecksumPolicy::Rfc9580)
                .unwrap_err()
                .kind(),
            expected
        );
    }

    let with_header =
        b"-----BEGIN PGP MESSAGE-----\nComment: x\n\nYWJj\n-----END PGP MESSAGE-----\n";
    for (limits, expected) in [
        (
            limited(max, max, max, max, 0, max, max, max, max, max),
            OpenPgpErrorKind::HeaderCountLimitExceeded,
        ),
        (
            limited(max, max, max, max, max, 1, max, max, max, max),
            OpenPgpErrorKind::HeaderBytesLimitExceeded,
        ),
    ] {
        assert_eq!(
            parse_armor_document(with_header, limits, ChecksumPolicy::Rfc9580)
                .unwrap_err()
                .kind(),
            expected
        );
    }

    let adjacent = b" \n-----BEGIN PGP MESSAGE-----\n\nYWJj\n-----END PGP MESSAGE-----\n";
    assert_eq!(
        parse_armor_document(
            adjacent,
            limited(max, max, max, max, max, max, max, max, 1, max),
            ChecksumPolicy::Rfc9580,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::AdjacentDocumentLimitExceeded
    );

    let long_body = format!(
        "-----BEGIN PGP MESSAGE-----\n\n{}\n-----END PGP MESSAGE-----\n",
        "A".repeat(77)
    );
    assert_eq!(
        parse_armor_document(
            long_body.as_bytes(),
            OpenPgpLimits::default(),
            ChecksumPolicy::Rfc9580,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::BodyLineTooLong
    );

    assert_eq!(
        encode_armor_to_string(
            ArmorType::Message,
            &[],
            b"abc",
            limited(max, 1, max, max, max, max, max, max, max, max),
            GenerationOptions::default(),
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::EncodedOutputLimitExceeded
    );
}

#[cfg(feature = "std")]
#[test]
fn bounded_reader_and_writer_round_trip_short_io() {
    use std::io::{Cursor, Write};

    struct ShortWriter(Vec<u8>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let take = bytes.len().min(3);
            self.0.extend_from_slice(&bytes[..take]);
            Ok(take)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let payload = pattern(1025);
    let mut writer = ShortWriter(Vec::new());
    base64_ng_openpgp::write_armor_block(
        &mut writer,
        ArmorType::Message,
        &[],
        &payload,
        OpenPgpLimits::default(),
        GenerationOptions::new(ChecksumGeneration::LegacyCrc24),
    )
    .unwrap();
    let document = base64_ng_openpgp::read_armor_document(
        Cursor::new(writer.0),
        OpenPgpLimits::default(),
        ChecksumPolicy::RequireValidCrc24,
    )
    .unwrap();
    assert_eq!(document.blocks()[0].contents(), payload);
}

#[cfg(feature = "secrets")]
#[test]
fn secret_parser_selects_exact_type_and_redacts() {
    let payload = b"CANARY-OPENPGP-PRIVATE-MATERIAL";
    let text = encode_armor_to_string(
        ArmorType::PrivateKey,
        &[],
        payload,
        OpenPgpLimits::default(),
        GenerationOptions::new(ChecksumGeneration::LegacyCrc24),
    )
    .unwrap();
    let block = base64_ng_openpgp::parse_secret_armor_block(
        text.as_bytes(),
        ArmorType::PrivateKey,
        OpenPgpLimits::default(),
        ChecksumPolicy::RequireValidCrc24,
    )
    .unwrap();
    assert_eq!(block.contents().expose_secret().as_ref(), payload);
    assert!(!format!("{block:?}").contains("CANARY"));
    assert_eq!(
        base64_ng_openpgp::parse_secret_armor_block(
            text.as_bytes(),
            ArmorType::PublicKey,
            OpenPgpLimits::default(),
            ChecksumPolicy::RequireValidCrc24,
        )
        .unwrap_err()
        .kind(),
        OpenPgpErrorKind::SecretBlockSelection
    );
}
