use super::wrapping::{LineEnding, LineWrap, LineWrapError};
use crate::{LineEnding as LegacyLineEnding, LineWrap as LegacyLineWrap, STANDARD};

#[test]
fn zero_width_is_unrepresentable_and_boundaries_are_checked() {
    assert_eq!(
        LineWrap::try_new(0, LineEnding::Lf),
        Err(LineWrapError::ZeroWidth)
    );

    let widest = LineWrap::try_new(usize::MAX, LineEnding::CrLf).unwrap();
    assert_eq!(widest.line_width().get(), usize::MAX);
    assert_eq!(widest.checked_output_len(usize::MAX), Some(usize::MAX));

    let narrow_lf = LineWrap::try_new(1, LineEnding::Lf).unwrap();
    let narrow_crlf = LineWrap::try_new(1, LineEnding::CrLf).unwrap();
    assert_eq!(narrow_lf.checked_output_len(0), Some(0));
    assert_eq!(narrow_lf.checked_output_len(1), Some(1));
    assert_eq!(narrow_lf.checked_output_len(2), Some(3));
    assert_eq!(narrow_lf.checked_output_len(usize::MAX), None);
    assert_eq!(narrow_crlf.checked_output_len(usize::MAX), None);
}

#[test]
fn trusted_body_constants_have_exact_layout_only() {
    assert_eq!(LineWrap::MIME_BODY_WRAP.line_width().get(), 76);
    assert_eq!(LineWrap::MIME_BODY_WRAP.line_ending(), LineEnding::CrLf);
    assert_eq!(LineWrap::PEM_BODY_LF_WRAP.line_width().get(), 64);
    assert_eq!(LineWrap::PEM_BODY_LF_WRAP.line_ending(), LineEnding::Lf);
    assert_eq!(LineWrap::PEM_BODY_CRLF_WRAP.line_width().get(), 64);
    assert_eq!(LineWrap::PEM_BODY_CRLF_WRAP.line_ending(), LineEnding::CrLf);
}

#[test]
fn length_arithmetic_matches_an_independent_small_oracle() {
    for width in 1..=16 {
        for ending in [LineEnding::Lf, LineEnding::CrLf] {
            let wrap = LineWrap::try_new(width, ending).unwrap();
            for payload_len in 0..=256 {
                let breaks = if payload_len == 0 {
                    0
                } else {
                    (payload_len - 1) / width
                };
                let expected = payload_len + breaks * ending.byte_len();
                assert_eq!(wrap.checked_output_len(payload_len), Some(expected));
            }
        }
    }
}

#[test]
fn insertion_matches_legacy_lf_and_crlf_encoding() {
    let mut input = [0u8; 96];
    for (index, byte) in input.iter_mut().enumerate() {
        let index = u8::try_from(index).unwrap();
        *byte = index.wrapping_mul(73).wrapping_add(19);
    }

    for width in [1, 2, 3, 4, 7, 16, 64, 76, usize::MAX] {
        for ending in [LineEnding::Lf, LineEnding::CrLf] {
            let wrap = LineWrap::try_new(width, ending).unwrap();
            let legacy = legacy_wrap(width, ending);
            for input_len in 0..=input.len() {
                let mut plain = [0u8; 128];
                let plain_len = STANDARD
                    .encode_slice(&input[..input_len], &mut plain)
                    .unwrap();
                let mut expected = [0u8; 384];
                let expected_len = STANDARD
                    .encode_slice_wrapped(&input[..input_len], &mut expected, legacy)
                    .unwrap();
                let mut actual = [0u8; 384];
                let actual_len = wrap.insert_into(&plain[..plain_len], &mut actual).unwrap();
                assert_eq!(&actual[..actual_len], &expected[..expected_len]);

                let mut compacted = [0u8; 128];
                let compacted_len = wrap
                    .copy_payload_into(&actual[..actual_len], &mut compacted)
                    .unwrap();
                assert_eq!(&compacted[..compacted_len], &plain[..plain_len]);
            }
        }
    }
}

#[test]
fn layout_validation_matches_legacy_wrapped_validation() {
    let payloads: &[&[u8]] = &[b"", b"Zg==", b"Zm9v", b"Zm9vYmFy", b"QUJDREVGR0g="];
    for width in 1..=8 {
        for ending in [LineEnding::Lf, LineEnding::CrLf] {
            let wrap = LineWrap::try_new(width, ending).unwrap();
            let legacy = legacy_wrap(width, ending);
            for payload in payloads {
                let mut canonical = [0u8; 64];
                let canonical_len = wrap.insert_into(payload, &mut canonical).unwrap();
                assert_layout_matches_legacy(wrap, legacy, &canonical[..canonical_len]);

                let separator = ending.as_bytes();
                let mut terminated = [0u8; 66];
                terminated[..canonical_len].copy_from_slice(&canonical[..canonical_len]);
                terminated[canonical_len..canonical_len + separator.len()]
                    .copy_from_slice(separator);
                assert_layout_matches_legacy(
                    wrap,
                    legacy,
                    &terminated[..canonical_len + separator.len()],
                );
            }
        }
    }
}

#[test]
fn malformed_layout_is_rejected_without_touching_output() {
    for (wrap, malformed) in [
        (
            LineWrap::try_new(4, LineEnding::Lf).unwrap(),
            &b"Zm9v\n\nYg=="[..],
        ),
        (
            LineWrap::try_new(4, LineEnding::Lf).unwrap(),
            &b"Zm\n9v"[..],
        ),
        (
            LineWrap::try_new(4, LineEnding::CrLf).unwrap(),
            &b"Zm9v\nYg=="[..],
        ),
        (
            LineWrap::try_new(4, LineEnding::CrLf).unwrap(),
            &b"Zm9v\rYg=="[..],
        ),
        (
            LineWrap::try_new(4, LineEnding::CrLf).unwrap(),
            &b"Zm9vYg=="[..],
        ),
    ] {
        let mut output = [0xa5; 32];
        assert_eq!(wrap.payload_len(malformed), None);
        assert_eq!(wrap.copy_payload_into(malformed, &mut output), None);
        assert_eq!(output, [0xa5; 32]);
    }

    let wrap = LineWrap::try_new(4, LineEnding::Lf).unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(wrap.insert_into(b"Zm9vYg==", &mut output[..7]), None);
    assert_eq!(output, [0xa5; 8]);
}

fn assert_layout_matches_legacy(wrap: LineWrap, legacy: LegacyLineWrap, input: &[u8]) {
    assert_eq!(
        wrap.payload_len(input).is_some(),
        STANDARD.validate_wrapped_result(input, legacy).is_ok(),
        "layout mismatch for {wrap:?}: {input:?}"
    );
}

const fn legacy_wrap(width: usize, ending: LineEnding) -> LegacyLineWrap {
    let ending = match ending {
        LineEnding::Lf => LegacyLineEnding::Lf,
        LineEnding::CrLf => LegacyLineEnding::CrLf,
    };
    LegacyLineWrap::new(width, ending)
}
