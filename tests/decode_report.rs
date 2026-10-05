use base64_ng::{
    DecodeFallback, DecodeValidation, DecodeValidator, STRICT_STANDARD_PADDED,
    STRICT_URL_SAFE_UNPADDED, runtime::Backend,
};

#[test]
fn reports_small_empty_and_reference_work_without_claiming_simd() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let mut output = [0xa5; 8];
        let (len, report) = STRICT_STANDARD_PADDED
            .decode_into_with_report(b"Zm9v", &mut output, policy)
            .unwrap();
        assert_eq!(len, 3);
        assert_eq!(&output[..3], b"foo");
        assert_eq!(output[3..], [0xa5; 5]);
        assert_eq!(report.requested_validation(), policy);
        assert_eq!(
            report.validator(),
            if policy == DecodeValidation::Auto {
                DecodeValidator::ScalarTable
            } else {
                DecodeValidator::ScalarReference
            }
        );
        assert_eq!(report.selected_backend(), None);
        assert_eq!(report.output_backend(), Backend::Scalar);
        assert!(!report.checked_validation());
        assert!(!report.checked_output());
        assert_eq!(report.fallback(), DecodeFallback::None);
        let (len, empty) = STRICT_STANDARD_PADDED
            .decode_into_with_report(b"", &mut output, policy)
            .unwrap();
        assert_eq!(len, 0);
        assert_eq!(
            empty.validator(),
            if policy == DecodeValidation::Auto {
                DecodeValidator::Empty
            } else {
                DecodeValidator::ScalarReference
            }
        );
    }
}

#[test]
fn reports_preserve_errors_capacity_precedence_and_complete_destinations() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for input in [
            b"Zm9v".as_slice(),
            b"Zm9!",
            b"Zh==",
            b"Zg=",
            b"AAAA ",
            b"____",
        ] {
            for size in 0..=8 {
                let mut expected = [0xa5; 8];
                let mut actual = expected;
                let reference = STRICT_STANDARD_PADDED.decode_into_with_validation(
                    input,
                    &mut expected[..size],
                    policy,
                );
                let result = STRICT_STANDARD_PADDED.decode_into_with_report(
                    input,
                    &mut actual[..size],
                    policy,
                );
                assert_eq!(result.map(|(len, _)| len), reference);
                assert_eq!(actual, expected);
                if result.is_err() {
                    assert_eq!(actual, [0xa5; 8]);
                }
            }
        }
    }
}

#[test]
fn reports_custom_and_relaxed_reference_fallback_and_url_tails() {
    use base64_ng::{CodecBuilder, DecodePadding, EncodePadding, ValidatedAlphabet};
    let custom = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    )
    .build()
    .unwrap();
    let relaxed = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
        )
        .unwrap(),
    )
    .encode_padding(EncodePadding::Padded)
    .decode_padding(DecodePadding::Indifferent)
    .build()
    .unwrap();
    for codec in [custom, relaxed] {
        let mut output = [0; 3];
        let (_, report) = codec
            .decode_into_with_report(b"AAAA", &mut output, DecodeValidation::Auto)
            .unwrap();
        assert_eq!(report.validator(), DecodeValidator::ScalarReference);
        assert_eq!(report.output_backend(), Backend::Scalar);
    }
    let mut output = [0; 2];
    let (len, _) = STRICT_URL_SAFE_UNPADDED
        .decode_into_with_report(b"__8", &mut output, DecodeValidation::Auto)
        .unwrap();
    assert_eq!(len, 2);
    assert_eq!(output, [255; 2]);
}

#[test]
fn reports_are_per_call_values_not_global_capability_snapshots() {
    let input = [b'A'; 4096];
    let mut output = [0; 3072];
    let (_, first) = STRICT_STANDARD_PADDED
        .decode_into_with_report(&input, &mut output, DecodeValidation::Auto)
        .unwrap();
    let saved = first;
    let (_, second) = STRICT_STANDARD_PADDED
        .decode_into_with_report(b"", &mut output, DecodeValidation::ScalarReference)
        .unwrap();
    assert_eq!(first, saved);
    assert_eq!(second.validator(), DecodeValidator::ScalarReference);
    assert_eq!(second.output_backend(), Backend::Scalar);
}
