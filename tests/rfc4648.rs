use base64_ng::{
    DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    checked_encoded_len, ct, decoded_capacity, decoded_len, encoded_len, runtime,
};

#[cfg(feature = "stream")]
use base64_ng::stream::{Decoder, DecoderReader, Encoder, EncoderReader};

#[cfg(feature = "stream")]
use std::io::{Cursor, Read, Write};

#[cfg(feature = "stream")]
struct ChunkedReader<'a> {
    input: &'a [u8],
    max_chunk: usize,
}

#[cfg(feature = "stream")]
impl Read for ChunkedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.input.len().min(self.max_chunk).min(output.len());
        if len == 0 {
            return Ok(0);
        }

        output[..len].copy_from_slice(&self.input[..len]);
        self.input = &self.input[len..];
        Ok(len)
    }
}

#[test]
fn rfc4648_standard_round_trips() {
    let cases: &[&[u8]] = &[
        b"",
        b"f",
        b"fo",
        b"foo",
        b"foob",
        b"fooba",
        b"foobar",
        b"The quick brown fox jumps over the lazy dog",
    ];

    for case in cases {
        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD.encode_slice(case, &mut encoded).unwrap();
        let mut decoded = [0u8; 128];
        let decoded_len = STANDARD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], *case);
    }
}

#[test]
fn runtime_backend_report_keeps_scalar_active() {
    let report = runtime::backend_report();
    let display = report.to_string();
    let snapshot = report.snapshot();

    assert_eq!(report.active, runtime::Backend::Scalar);
    assert_eq!(snapshot.active, "scalar");
    assert_eq!(snapshot.candidate, report.candidate.as_str());
    assert_eq!(
        snapshot.candidate_required_cpu_features,
        report.candidate_required_cpu_features()
    );
    assert_eq!(snapshot.simd_feature_enabled, cfg!(feature = "simd"));
    assert!(!snapshot.accelerated_backend_active);
    assert!(snapshot.unsafe_boundary_enforced);
    assert_eq!(snapshot.security_posture, report.security_posture.as_str());
    assert_eq!(report.active.as_str(), "scalar");
    assert_eq!(report.active.to_string(), "scalar");
    assert!(runtime::Backend::Scalar.required_cpu_features().is_empty());
    assert_eq!(
        report.candidate_required_cpu_features(),
        report.candidate.required_cpu_features()
    );
    assert_eq!(runtime::Backend::Avx512Vbmi.as_str(), "avx512-vbmi");
    assert_eq!(
        runtime::Backend::Avx512Vbmi.required_cpu_features(),
        ["avx512f", "avx512bw", "avx512vl", "avx512vbmi"]
    );
    assert_eq!(runtime::Backend::Avx2.required_cpu_features(), ["avx2"]);
    assert_eq!(runtime::Backend::Neon.required_cpu_features(), ["neon"]);
    assert!(display.contains("active=scalar"));
    assert!(display.contains("candidate_required_cpu_features="));
    assert!(display.contains("accelerated_backend_active=false"));
    assert!(!report.accelerated_backend_active);
    assert!(report.unsafe_boundary_enforced);
    assert_eq!(report.simd_feature_enabled, cfg!(feature = "simd"));

    if report.candidate == runtime::Backend::Scalar {
        assert_eq!(
            report.security_posture,
            runtime::SecurityPosture::ScalarOnly
        );
        assert_eq!(report.security_posture.as_str(), "scalar-only");
    } else {
        assert_eq!(
            report.security_posture,
            runtime::SecurityPosture::SimdCandidateScalarActive
        );
        assert_eq!(
            report.security_posture.as_str(),
            "simd-candidate-scalar-active"
        );
    }
}

#[test]
fn runtime_backend_policy_assertions_are_explicit() {
    let report = runtime::backend_report();

    assert_eq!(
        runtime::require_backend_policy(runtime::BackendPolicy::ScalarExecutionOnly),
        Ok(())
    );
    assert!(report.satisfies(runtime::BackendPolicy::ScalarExecutionOnly));
    assert_eq!(
        runtime::BackendPolicy::ScalarExecutionOnly.as_str(),
        "scalar-execution-only"
    );
    assert_eq!(
        runtime::BackendPolicy::HighAssuranceScalarOnly.to_string(),
        "high-assurance-scalar-only"
    );
    let artificial_report = runtime::BackendReport {
        active: runtime::Backend::Scalar,
        candidate: runtime::Backend::Avx2,
        simd_feature_enabled: true,
        accelerated_backend_active: false,
        unsafe_boundary_enforced: true,
        security_posture: runtime::SecurityPosture::SimdCandidateScalarActive,
    };
    let artificial_error = runtime::BackendPolicyError {
        policy: runtime::BackendPolicy::HighAssuranceScalarOnly,
        report: artificial_report,
    };
    assert_eq!(
        artificial_error.to_string(),
        "runtime backend policy `high-assurance-scalar-only` was not satisfied (active=scalar candidate=avx2 candidate_required_cpu_features=[avx2] simd_feature_enabled=true accelerated_backend_active=false unsafe_boundary_enforced=true security_posture=simd-candidate-scalar-active)"
    );

    let simd_feature_policy =
        runtime::require_backend_policy(runtime::BackendPolicy::SimdFeatureDisabled);
    if cfg!(feature = "simd") {
        assert!(!report.satisfies(runtime::BackendPolicy::SimdFeatureDisabled));
        assert_eq!(
            simd_feature_policy.unwrap_err().policy,
            runtime::BackendPolicy::SimdFeatureDisabled
        );
    } else {
        assert!(report.satisfies(runtime::BackendPolicy::SimdFeatureDisabled));
        assert_eq!(simd_feature_policy, Ok(()));
    }

    let no_candidate_policy =
        runtime::require_backend_policy(runtime::BackendPolicy::NoDetectedSimdCandidate);
    if report.candidate == runtime::Backend::Scalar {
        assert!(report.satisfies(runtime::BackendPolicy::NoDetectedSimdCandidate));
        assert_eq!(no_candidate_policy, Ok(()));
    } else {
        assert!(!report.satisfies(runtime::BackendPolicy::NoDetectedSimdCandidate));
        assert_eq!(
            no_candidate_policy.unwrap_err().policy,
            runtime::BackendPolicy::NoDetectedSimdCandidate
        );
    }

    let high_assurance_policy =
        runtime::require_backend_policy(runtime::BackendPolicy::HighAssuranceScalarOnly);
    if report.satisfies(runtime::BackendPolicy::HighAssuranceScalarOnly) {
        assert_eq!(high_assurance_policy, Ok(()));
    } else {
        let err = high_assurance_policy.unwrap_err();
        assert_eq!(err.policy, runtime::BackendPolicy::HighAssuranceScalarOnly);
        assert!(err.to_string().contains("high-assurance-scalar-only"));
        assert!(err.to_string().contains("active=scalar"));
    }
}

#[test]
fn unpadded_round_trips() {
    for input_len in 0..64 {
        let mut input = [0u8; 64];
        for (index, byte) in input.iter_mut().enumerate() {
            *byte = (index * 37 + input_len) as u8;
        }

        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD_NO_PAD
            .encode_slice(&input[..input_len], &mut encoded)
            .unwrap();
        assert!(!encoded[..encoded_len].contains(&b'='));

        let mut decoded = [0u8; 64];
        let decoded_len = STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], &input[..input_len]);
    }
}

#[test]
fn url_safe_alphabet_is_distinct() {
    let mut padded = [0u8; 8];
    let padded_len = URL_SAFE.encode_slice(b"\xfb\xff", &mut padded).unwrap();
    assert_eq!(&padded[..padded_len], b"-_8=");

    let mut unpadded = [0u8; 8];
    let unpadded_len = URL_SAFE_NO_PAD
        .encode_slice(b"\xfb\xff", &mut unpadded)
        .unwrap();
    assert_eq!(&unpadded[..unpadded_len], b"-_8");
}

#[test]
fn ct_decoder_matches_strict_for_canonical_inputs() {
    for input_len in 0..64 {
        let mut input = [0u8; 64];
        for (index, byte) in input.iter_mut().enumerate() {
            *byte = (index * 29 + input_len * 3) as u8;
        }
        let input = &input[..input_len];

        let mut encoded = [0u8; 128];
        let mut strict = [0u8; 64];
        let mut ct_output = [0u8; 64];

        let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
        let strict_len = STANDARD
            .decode_slice(&encoded[..encoded_len], &mut strict)
            .unwrap();
        let ct_len = ct::STANDARD
            .decode_slice(&encoded[..encoded_len], &mut ct_output)
            .unwrap();
        assert_eq!(&ct_output[..ct_len], &strict[..strict_len]);

        let encoded_len = STANDARD_NO_PAD.encode_slice(input, &mut encoded).unwrap();
        let strict_len = STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut strict)
            .unwrap();
        let ct_len = ct::STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut ct_output)
            .unwrap();
        assert_eq!(&ct_output[..ct_len], &strict[..strict_len]);

        let encoded_len = URL_SAFE.encode_slice(input, &mut encoded).unwrap();
        let strict_len = URL_SAFE
            .decode_slice(&encoded[..encoded_len], &mut strict)
            .unwrap();
        let ct_len = ct::URL_SAFE
            .decode_slice(&encoded[..encoded_len], &mut ct_output)
            .unwrap();
        assert_eq!(&ct_output[..ct_len], &strict[..strict_len]);

        let encoded_len = URL_SAFE_NO_PAD.encode_slice(input, &mut encoded).unwrap();
        let strict_len = URL_SAFE_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut strict)
            .unwrap();
        let ct_len = ct::URL_SAFE_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut ct_output)
            .unwrap();
        assert_eq!(&ct_output[..ct_len], &strict[..strict_len]);
    }
}

#[test]
fn ct_decoder_rejects_malformed_inputs() {
    let mut output = [0u8; 8];

    assert_eq!(
        ct::STANDARD.decode_slice(b"AA-A", &mut output),
        Err(DecodeError::InvalidInput)
    );
    assert_eq!(
        ct::URL_SAFE.decode_slice(b"AA+A", &mut output),
        Err(DecodeError::InvalidInput)
    );
    assert_eq!(
        ct::STANDARD.decode_slice(b"AA=A", &mut output),
        Err(DecodeError::InvalidInput)
    );
    assert_eq!(
        ct::STANDARD.decode_slice(b"Zh==", &mut output),
        Err(DecodeError::InvalidInput)
    );
    assert_eq!(
        ct::STANDARD_NO_PAD.decode_slice(b"Zg==", &mut output),
        Err(DecodeError::InvalidInput)
    );

    let mut too_small = [0u8; 1];
    assert_eq!(
        ct::STANDARD.decode_slice(b"aGk=", &mut too_small),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
}

#[test]
fn ct_decode_slice_clear_tail_scrubs_unused_output() {
    let mut standard = [0xff; 8];
    let written = ct::STANDARD
        .decode_slice_clear_tail(b"aGk=", &mut standard)
        .unwrap();
    assert_eq!(&standard[..written], b"hi");
    assert_eq!(&standard[written..], &[0; 6]);

    let mut standard_no_pad = [0xff; 8];
    let written = ct::STANDARD_NO_PAD
        .decode_slice_clear_tail(b"aGk", &mut standard_no_pad)
        .unwrap();
    assert_eq!(&standard_no_pad[..written], b"hi");
    assert_eq!(&standard_no_pad[written..], &[0; 6]);
}

#[test]
fn ct_decode_slice_clear_tail_scrubs_output_on_error() {
    let mut invalid_byte = [0xff; 8];
    assert_eq!(
        ct::STANDARD.decode_slice_clear_tail(b"Zm9v$g==", &mut invalid_byte),
        Err(DecodeError::InvalidInput)
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut invalid_padding = [0xff; 8];
    assert_eq!(
        ct::STANDARD.decode_slice_clear_tail(b"Zh==", &mut invalid_padding),
        Err(DecodeError::InvalidInput)
    );
    assert!(invalid_padding.iter().all(|byte| *byte == 0));

    let mut too_small = [0xff; 1];
    assert_eq!(
        ct::STANDARD.decode_slice_clear_tail(b"aGk=", &mut too_small),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
    assert!(too_small.iter().all(|byte| *byte == 0));
}

#[test]
fn ct_decode_in_place_matches_slice_for_canonical_inputs() {
    for input_len in 0..64 {
        let mut input = [0u8; 64];
        for (index, byte) in input.iter_mut().enumerate() {
            *byte = (index * 31 + input_len * 5) as u8;
        }
        let input = &input[..input_len];

        let mut encoded = [0u8; 128];
        let mut expected = [0u8; 64];

        let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
        let expected_len = ct::STANDARD
            .decode_slice(&encoded[..encoded_len], &mut expected)
            .unwrap();
        let mut in_place = [0u8; 128];
        in_place[..encoded_len].copy_from_slice(&encoded[..encoded_len]);
        let decoded = ct::STANDARD
            .decode_in_place(&mut in_place[..encoded_len])
            .unwrap();
        assert_eq!(decoded, &expected[..expected_len]);

        let encoded_len = STANDARD_NO_PAD.encode_slice(input, &mut encoded).unwrap();
        let expected_len = ct::STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut expected)
            .unwrap();
        in_place[..encoded_len].copy_from_slice(&encoded[..encoded_len]);
        let decoded = ct::STANDARD_NO_PAD
            .decode_in_place(&mut in_place[..encoded_len])
            .unwrap();
        assert_eq!(decoded, &expected[..expected_len]);
    }
}

#[test]
fn ct_decode_in_place_clear_tail_scrubs_buffer() {
    let mut standard = *b"aGk=";
    let len = {
        let decoded = ct::STANDARD
            .decode_in_place_clear_tail(&mut standard)
            .unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard[len..], &[0, 0]);

    let mut standard_no_pad = *b"aGk";
    let len = {
        let decoded = ct::STANDARD_NO_PAD
            .decode_in_place_clear_tail(&mut standard_no_pad)
            .unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard_no_pad[len..], &[0]);

    let mut invalid_byte = *b"Zm9v$g==";
    assert_eq!(
        ct::STANDARD.decode_in_place_clear_tail(&mut invalid_byte),
        Err(DecodeError::InvalidInput)
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut invalid_padding = *b"Zh==";
    assert_eq!(
        ct::STANDARD.decode_in_place_clear_tail(&mut invalid_padding),
        Err(DecodeError::InvalidInput)
    );
    assert!(invalid_padding.iter().all(|byte| *byte == 0));
}

#[test]
fn ct_decoder_uses_non_localized_malformed_errors() {
    let mut output = [0u8; 8];

    for input in [b"$AAA", b"A$AA", b"AA$A", b"AAA$"] {
        assert_eq!(
            ct::STANDARD.decode_slice(input, &mut output),
            Err(DecodeError::InvalidInput)
        );
    }

    for input in [b"AA=A", b"Zm9=", b"Zh=="] {
        assert_eq!(
            ct::STANDARD.decode_slice(input, &mut output),
            Err(DecodeError::InvalidInput)
        );
    }
}

#[test]
fn const_array_encoding_matches_runtime_encoding() {
    const STANDARD_HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    const STANDARD_HELLO_NO_PAD: [u8; 7] = STANDARD_NO_PAD.encode_array(b"hello");
    const URL_SAFE_BYTES: [u8; 4] = URL_SAFE.encode_array(b"\xfb\xff");
    const URL_SAFE_BYTES_NO_PAD: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    const EMPTY: [u8; 0] = STANDARD.encode_array(b"");

    assert_eq!(&STANDARD_HELLO, b"aGVsbG8=");
    assert_eq!(&STANDARD_HELLO_NO_PAD, b"aGVsbG8");
    assert_eq!(&URL_SAFE_BYTES, b"-_8=");
    assert_eq!(&URL_SAFE_BYTES_NO_PAD, b"-_8");
    assert_eq!(&EMPTY, b"");
}

#[test]
fn decoded_capacity_is_upper_bound() {
    for encoded_len in 0..128 {
        assert!(decoded_capacity(encoded_len) <= encoded_len / 4 * 3 + 2);
    }
}

#[test]
fn decoded_len_reports_exact_lengths() {
    assert_eq!(decoded_len(b"", true), Ok(0));
    assert_eq!(decoded_len(b"Zg==", true), Ok(1));
    assert_eq!(decoded_len(b"Zm8=", true), Ok(2));
    assert_eq!(decoded_len(b"Zm9v", true), Ok(3));
    assert_eq!(decoded_len(b"Zg", false), Ok(1));
    assert_eq!(decoded_len(b"Zm8", false), Ok(2));
    assert_eq!(decoded_len(b"Zm9v", false), Ok(3));
    assert_eq!(STANDARD.decoded_len(b"Zm9v"), Ok(3));
    assert_eq!(STANDARD_NO_PAD.decoded_len(b"Zm9v"), Ok(3));
}

#[test]
fn validate_strict_reports_canonical_inputs_without_decoding() {
    assert!(STANDARD.validate(b""));
    assert!(STANDARD.validate(b"Zg=="));
    assert!(STANDARD.validate(b"Zm8="));
    assert!(STANDARD.validate(b"Zm9v"));
    assert!(STANDARD_NO_PAD.validate(b"Zg"));
    assert!(STANDARD_NO_PAD.validate(b"Zm8"));
    assert!(STANDARD_NO_PAD.validate(b"Zm9v"));
    assert!(URL_SAFE.validate(b"-_8="));
    assert!(URL_SAFE_NO_PAD.validate(b"-_8"));

    assert_eq!(STANDARD.validate_result(b"Zg=="), Ok(()));
    assert_eq!(STANDARD_NO_PAD.validate_result(b"Zg"), Ok(()));
}

#[test]
fn validate_strict_rejects_malformed_inputs_without_decoding() {
    assert!(!STANDARD.validate(b"Zg"));
    assert!(!STANDARD.validate(b"Zg==="));
    assert!(!STANDARD.validate(b"Z==="));
    assert!(!STANDARD.validate(b"AA=A"));
    assert!(!STANDARD.validate(b"Zh=="));
    assert!(!STANDARD.validate(b"Zm9v\n"));
    assert!(!STANDARD.validate(b"Zm-v"));
    assert!(!URL_SAFE.validate(b"Zm+v"));
    assert!(!STANDARD_NO_PAD.validate(b"Zg=="));

    assert_eq!(
        STANDARD.validate_result(b"Zg"),
        Err(DecodeError::InvalidLength)
    );
    assert_eq!(
        STANDARD.validate_result(b"Zm-v"),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD.validate_result(b"Zh=="),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
}

#[test]
fn validate_legacy_ignores_transport_whitespace_without_decoding() {
    assert!(STANDARD.validate_legacy(b" Z\r\ng\t== "));
    assert!(STANDARD.validate_legacy(b" Zm\r\n9v "));
    assert!(STANDARD_NO_PAD.validate_legacy(b" Z\r\ng "));
    assert_eq!(STANDARD.validate_legacy_result(b" Z\r\ng== "), Ok(()));

    assert!(!STANDARD.validate_legacy(b" Z-g== "));
    assert!(!STANDARD.validate_legacy(b" Zg== A"));
    assert_eq!(
        STANDARD.validate_legacy_result(b" Z-g== "),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD.validate_legacy_result(b" Zg== A"),
        Err(DecodeError::InvalidPadding { index: 6 })
    );
}

#[test]
fn decoded_len_rejects_bad_lengths_and_padding() {
    assert_eq!(decoded_len(b"Z", true), Err(DecodeError::InvalidLength));
    assert_eq!(decoded_len(b"Z", false), Err(DecodeError::InvalidLength));
    assert_eq!(
        decoded_len(b"Z=m9", true),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        decoded_len(b"Zm=9", true),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        decoded_len(b"Zm8=", false),
        Err(DecodeError::InvalidPadding { index: 3 })
    );
}

#[test]
fn checked_encoded_len_reports_overflow() {
    assert_eq!(
        encoded_len(usize::MAX, true),
        Err(EncodeError::LengthOverflow)
    );
    assert_eq!(
        STANDARD.encoded_len(usize::MAX),
        Err(EncodeError::LengthOverflow)
    );
    assert_eq!(checked_encoded_len(usize::MAX, true), None);
    assert_eq!(checked_encoded_len(usize::MAX, false), None);
    assert_eq!(STANDARD.checked_encoded_len(usize::MAX), None);
    assert_eq!(STANDARD_NO_PAD.checked_encoded_len(usize::MAX), None);
}

#[test]
fn encode_slice_reports_small_outputs() {
    let mut output = [0u8; 1];
    assert_eq!(
        STANDARD.encode_slice(b"hi", &mut output),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 1,
        })
    );
}

#[test]
fn encode_slice_clear_tail_scrubs_unused_output() {
    let mut standard = [0xff; 12];
    let written = STANDARD
        .encode_slice_clear_tail(b"hello", &mut standard)
        .unwrap();
    assert_eq!(&standard[..written], b"aGVsbG8=");
    assert_eq!(&standard[written..], &[0; 4]);

    let mut standard_no_pad = [0xff; 10];
    let written = STANDARD_NO_PAD
        .encode_slice_clear_tail(b"hello", &mut standard_no_pad)
        .unwrap();
    assert_eq!(&standard_no_pad[..written], b"aGVsbG8");
    assert_eq!(&standard_no_pad[written..], &[0; 3]);
}

#[test]
fn encode_slice_clear_tail_scrubs_output_on_error() {
    let mut output = [0xff; 3];
    assert_eq!(
        STANDARD.encode_slice_clear_tail(b"hi", &mut output),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 3,
        })
    );
    assert!(output.iter().all(|byte| *byte == 0));
}

#[test]
fn decode_slice_reports_small_outputs() {
    let mut output = [0u8; 1];
    assert_eq!(
        STANDARD.decode_slice(b"aGk=", &mut output),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"aGk", &mut output),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
}

#[test]
fn decode_slice_clear_tail_scrubs_unused_output() {
    let mut standard = [0xff; 8];
    let written = STANDARD
        .decode_slice_clear_tail(b"aGk=", &mut standard)
        .unwrap();
    assert_eq!(&standard[..written], b"hi");
    assert_eq!(&standard[written..], &[0; 6]);

    let mut standard_no_pad = [0xff; 8];
    let written = STANDARD_NO_PAD
        .decode_slice_clear_tail(b"aGk", &mut standard_no_pad)
        .unwrap();
    assert_eq!(&standard_no_pad[..written], b"hi");
    assert_eq!(&standard_no_pad[written..], &[0; 6]);
}

#[test]
fn decode_slice_clear_tail_scrubs_output_on_error() {
    let mut invalid_byte = [0xff; 8];
    assert_eq!(
        STANDARD.decode_slice_clear_tail(b"Zm9v$g==", &mut invalid_byte),
        Err(DecodeError::InvalidByte {
            index: 4,
            byte: b'$',
        })
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut too_small = [0xff; 1];
    assert_eq!(
        STANDARD.decode_slice_clear_tail(b"aGk=", &mut too_small),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
    assert!(too_small.iter().all(|byte| *byte == 0));
}

#[test]
fn encodes_in_place() {
    let mut standard = [0u8; 8];
    standard[..5].copy_from_slice(b"hello");
    let encoded = STANDARD.encode_in_place(&mut standard, 5).unwrap();
    assert_eq!(encoded, b"aGVsbG8=");

    let mut standard_no_pad = [0u8; 7];
    standard_no_pad[..5].copy_from_slice(b"hello");
    let encoded = STANDARD_NO_PAD
        .encode_in_place(&mut standard_no_pad, 5)
        .unwrap();
    assert_eq!(encoded, b"aGVsbG8");

    let mut url_safe = [0u8; 4];
    url_safe[..2].copy_from_slice(b"\xfb\xff");
    let encoded = URL_SAFE.encode_in_place(&mut url_safe, 2).unwrap();
    assert_eq!(encoded, b"-_8=");

    let mut url_safe_no_pad = [0u8; 3];
    url_safe_no_pad[..2].copy_from_slice(b"\xfb\xff");
    let encoded = URL_SAFE_NO_PAD
        .encode_in_place(&mut url_safe_no_pad, 2)
        .unwrap();
    assert_eq!(encoded, b"-_8");
}

#[test]
fn encode_in_place_reports_bad_lengths() {
    let mut too_small = [0u8; 3];
    too_small[..2].copy_from_slice(b"hi");
    assert_eq!(
        STANDARD.encode_in_place(&mut too_small, 2),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 3,
        })
    );

    let mut buffer = [0u8; 2];
    assert_eq!(
        STANDARD.encode_in_place(&mut buffer, 3),
        Err(EncodeError::InputTooLarge {
            input_len: 3,
            buffer_len: 2,
        })
    );
}

#[test]
fn encode_in_place_clear_tail_scrubs_unused_bytes() {
    let mut standard = [0xff; 12];
    standard[..5].copy_from_slice(b"hello");
    let len = {
        let encoded = STANDARD
            .encode_in_place_clear_tail(&mut standard, 5)
            .unwrap();
        assert_eq!(encoded, b"aGVsbG8=");
        encoded.len()
    };
    assert_eq!(&standard[len..], &[0; 4]);

    let mut standard_no_pad = [0xff; 10];
    standard_no_pad[..5].copy_from_slice(b"hello");
    let len = {
        let encoded = STANDARD_NO_PAD
            .encode_in_place_clear_tail(&mut standard_no_pad, 5)
            .unwrap();
        assert_eq!(encoded, b"aGVsbG8");
        encoded.len()
    };
    assert_eq!(&standard_no_pad[len..], &[0; 3]);

    let mut url_safe = [0xff; 6];
    url_safe[..2].copy_from_slice(b"\xfb\xff");
    let len = {
        let encoded = URL_SAFE
            .encode_in_place_clear_tail(&mut url_safe, 2)
            .unwrap();
        assert_eq!(encoded, b"-_8=");
        encoded.len()
    };
    assert_eq!(&url_safe[len..], &[0; 2]);
}

#[test]
fn encode_in_place_clear_tail_scrubs_buffer_on_error() {
    let mut too_small = [0xff; 3];
    too_small[..2].copy_from_slice(b"hi");
    assert_eq!(
        STANDARD.encode_in_place_clear_tail(&mut too_small, 2),
        Err(EncodeError::OutputTooSmall {
            required: 4,
            available: 3,
        })
    );
    assert!(too_small.iter().all(|byte| *byte == 0));

    let mut input_too_large = [0xff; 2];
    assert_eq!(
        STANDARD.encode_in_place_clear_tail(&mut input_too_large, 3),
        Err(EncodeError::InputTooLarge {
            input_len: 3,
            buffer_len: 2,
        })
    );
    assert!(input_too_large.iter().all(|byte| *byte == 0));
}

#[test]
fn decode_in_place_clear_tail_scrubs_unused_bytes() {
    let mut standard = *b"aGk=";
    let len = {
        let decoded = STANDARD.decode_in_place_clear_tail(&mut standard).unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard[len..], &[0, 0]);

    let mut standard_no_pad = *b"aGk";
    let len = {
        let decoded = STANDARD_NO_PAD
            .decode_in_place_clear_tail(&mut standard_no_pad)
            .unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard_no_pad[len..], &[0]);

    let mut url_safe = *b"-_8=";
    let len = {
        let decoded = URL_SAFE.decode_in_place_clear_tail(&mut url_safe).unwrap();
        assert_eq!(decoded, b"\xfb\xff");
        decoded.len()
    };
    assert_eq!(&url_safe[len..], &[0, 0]);
}

#[test]
fn decode_in_place_clear_tail_scrubs_buffer_on_error() {
    let mut invalid_byte = *b"Zm9v$g==";
    assert_eq!(
        STANDARD.decode_in_place_clear_tail(&mut invalid_byte),
        Err(DecodeError::InvalidByte {
            index: 4,
            byte: b'$',
        })
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut invalid_padding = *b"Zh==";
    assert_eq!(
        STANDARD.decode_in_place_clear_tail(&mut invalid_padding),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert!(invalid_padding.iter().all(|byte| *byte == 0));
}

#[test]
fn runtime_encode_errors_do_not_panic() {
    let checks = [
        std::panic::catch_unwind(|| encoded_len(usize::MAX, true).unwrap_err()).is_ok(),
        std::panic::catch_unwind(|| STANDARD.encoded_len(usize::MAX).unwrap_err()).is_ok(),
        std::panic::catch_unwind(|| {
            let mut output = [0u8; 1];
            let _ = STANDARD.encode_slice(b"hello", &mut output);
        })
        .is_ok(),
        std::panic::catch_unwind(|| {
            let mut buffer = [0u8; 2];
            let _ = STANDARD.encode_in_place(&mut buffer, 3);
        })
        .is_ok(),
    ];

    assert!(checks.into_iter().all(|passed| passed));
}

#[test]
fn malformed_runtime_decode_inputs_do_not_panic() {
    let malformed_inputs: &[&[u8]] = &[
        b"a",
        b"====",
        b"Z=m9",
        b"Zm=9",
        b"Zm8=",
        b"Zm9v$g==",
        b"AA-A",
        b"Zm9vZh==",
    ];

    for input in malformed_inputs {
        assert!(std::panic::catch_unwind(|| decoded_len(input, true)).is_ok());
        assert!(std::panic::catch_unwind(|| decoded_len(input, false)).is_ok());
        assert!(
            std::panic::catch_unwind(|| {
                let mut output = [0u8; 16];
                let _ = STANDARD.decode_slice(input, &mut output);
            })
            .is_ok()
        );
        assert!(
            std::panic::catch_unwind(|| {
                let mut buffer = input.to_vec();
                let _ = STANDARD.decode_in_place(&mut buffer);
            })
            .is_ok()
        );
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn deterministic_long_round_trips() {
    let mut input = Vec::new();
    for len in 0..=1024 {
        input.resize(len, 0);
        fill_deterministic(&mut input, len as u64);

        assert_equivalent_round_trip(&STANDARD, &input);
        assert_equivalent_round_trip(&STANDARD_NO_PAD, &input);
        assert_equivalent_round_trip(&URL_SAFE, &input);
        assert_equivalent_round_trip(&URL_SAFE_NO_PAD, &input);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn exhaustive_short_round_trips() {
    for b0 in u8::MIN..=u8::MAX {
        let input = [b0];
        assert_round_trip(&STANDARD, &input);
        assert_round_trip(&STANDARD_NO_PAD, &input);
        assert_round_trip(&URL_SAFE, &input);
        assert_round_trip(&URL_SAFE_NO_PAD, &input);
        assert_ct_round_trip(&STANDARD, &ct::STANDARD, &input);
        assert_ct_round_trip(&STANDARD_NO_PAD, &ct::STANDARD_NO_PAD, &input);
        assert_ct_round_trip(&URL_SAFE, &ct::URL_SAFE, &input);
        assert_ct_round_trip(&URL_SAFE_NO_PAD, &ct::URL_SAFE_NO_PAD, &input);
        assert_in_place_encode_matches_slice(&STANDARD, &input);
        assert_in_place_encode_matches_slice(&STANDARD_NO_PAD, &input);
        assert_in_place_encode_matches_slice(&URL_SAFE, &input);
        assert_in_place_encode_matches_slice(&URL_SAFE_NO_PAD, &input);
    }

    for b0 in u8::MIN..=u8::MAX {
        for b1 in u8::MIN..=u8::MAX {
            let input = [b0, b1];
            assert_round_trip(&STANDARD, &input);
            assert_round_trip(&STANDARD_NO_PAD, &input);
            assert_round_trip(&URL_SAFE, &input);
            assert_round_trip(&URL_SAFE_NO_PAD, &input);
            assert_ct_round_trip(&STANDARD, &ct::STANDARD, &input);
            assert_ct_round_trip(&STANDARD_NO_PAD, &ct::STANDARD_NO_PAD, &input);
            assert_ct_round_trip(&URL_SAFE, &ct::URL_SAFE, &input);
            assert_ct_round_trip(&URL_SAFE_NO_PAD, &ct::URL_SAFE_NO_PAD, &input);
            assert_in_place_encode_matches_slice(&STANDARD, &input);
            assert_in_place_encode_matches_slice(&STANDARD_NO_PAD, &input);
            assert_in_place_encode_matches_slice(&URL_SAFE, &input);
            assert_in_place_encode_matches_slice(&URL_SAFE_NO_PAD, &input);
        }
    }
}

#[test]
fn rejects_common_non_alphabet_bytes() {
    let mut output = [0u8; 4];
    for byte in [b' ', b'\n', b'\r', b'\t', 0, 0xff] {
        let input = [b'A', b'A', byte, b'A'];
        assert_eq!(
            STANDARD.decode_slice(&input, &mut output),
            Err(DecodeError::InvalidByte { index: 2, byte })
        );
    }
}

#[test]
fn rejects_all_non_alphabet_bytes_by_position() {
    let mut output = [0u8; 4];
    for byte in u8::MIN..=u8::MAX {
        if is_standard_alphabet_byte(byte) || byte == b'=' {
            continue;
        }

        for index in 0..4 {
            let mut input = *b"AAAA";
            input[index] = byte;
            assert_eq!(
                STANDARD.decode_slice(&input, &mut output),
                Err(DecodeError::InvalidByte { index, byte }),
                "byte {byte:#04x} at index {index}"
            );
        }
    }
}

#[test]
fn url_safe_rejects_standard_only_symbols_by_position() {
    let mut output = [0u8; 4];
    for byte in [b'+', b'/'] {
        for index in 0..4 {
            let mut input = *b"AAAA";
            input[index] = byte;
            assert_eq!(
                URL_SAFE.decode_slice(&input, &mut output),
                Err(DecodeError::InvalidByte { index, byte }),
                "byte {byte:#04x} at index {index}"
            );
        }
    }
}

#[test]
fn standard_rejects_url_safe_only_symbols_by_position() {
    let mut output = [0u8; 4];
    for byte in [b'-', b'_'] {
        for index in 0..4 {
            let mut input = *b"AAAA";
            input[index] = byte;
            assert_eq!(
                STANDARD.decode_slice(&input, &mut output),
                Err(DecodeError::InvalidByte { index, byte }),
                "byte {byte:#04x} at index {index}"
            );
        }
    }
}

#[test]
fn legacy_decode_ignores_transport_whitespace() {
    let input = b" aG\r\nVs\tbG8= ";
    assert_eq!(STANDARD.decoded_len_legacy(input), Ok(5));

    let mut output = [0u8; 5];
    let written = STANDARD.decode_slice_legacy(input, &mut output).unwrap();
    assert_eq!(&output[..written], b"hello");

    #[cfg(feature = "alloc")]
    {
        let decoded = STANDARD.decode_vec_legacy(input).unwrap();
        assert_eq!(decoded, b"hello");
    }

    let mut in_place = *b" aG\r\nVs\tbG8= ";
    let decoded = STANDARD.decode_in_place_legacy(&mut in_place).unwrap();
    assert_eq!(decoded, b"hello");
}

#[test]
fn legacy_decode_supports_unpadded_whitespace() {
    let input = b" aG\r\nVs\tbG8 ";
    assert_eq!(STANDARD_NO_PAD.decoded_len_legacy(input), Ok(5));

    let mut output = [0u8; 5];
    let written = STANDARD_NO_PAD
        .decode_slice_legacy(input, &mut output)
        .unwrap();
    assert_eq!(&output[..written], b"hello");
}

#[test]
fn legacy_decode_slice_clear_tail_scrubs_unused_output() {
    let mut standard = [0xff; 8];
    let written = STANDARD
        .decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut standard)
        .unwrap();
    assert_eq!(&standard[..written], b"hi");
    assert_eq!(&standard[written..], &[0; 6]);

    let mut standard_no_pad = [0xff; 8];
    let written = STANDARD_NO_PAD
        .decode_slice_legacy_clear_tail(b" aG\r\nk ", &mut standard_no_pad)
        .unwrap();
    assert_eq!(&standard_no_pad[..written], b"hi");
    assert_eq!(&standard_no_pad[written..], &[0; 6]);
}

#[test]
fn legacy_decode_slice_clear_tail_scrubs_output_on_error() {
    let mut invalid_byte = [0xff; 8];
    assert_eq!(
        STANDARD.decode_slice_legacy_clear_tail(b" A A - A", &mut invalid_byte),
        Err(DecodeError::InvalidByte {
            index: 5,
            byte: b'-',
        })
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut too_small = [0xff; 1];
    assert_eq!(
        STANDARD.decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut too_small),
        Err(DecodeError::OutputTooSmall {
            required: 2,
            available: 1,
        })
    );
    assert!(too_small.iter().all(|byte| *byte == 0));
}

#[test]
fn legacy_decode_keeps_strict_alphabet_and_padding_rules() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice_legacy(b"AA -A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 3,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD.decode_slice_legacy(b"Zh ==", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD.decode_slice_legacy(b"aGk= AAAA", &mut output),
        Err(DecodeError::InvalidPadding { index: 5 })
    );
}

#[test]
fn legacy_decode_reports_original_indexes_after_whitespace() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice_legacy(b" A A - A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 5,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice_legacy(b" Z h ", &mut output),
        Err(DecodeError::InvalidPadding { index: 3 })
    );
    assert_eq!(
        STANDARD.decode_slice_legacy(b"aGk= \n AAAA", &mut output),
        Err(DecodeError::InvalidPadding { index: 7 })
    );
}

#[test]
fn legacy_in_place_decode_matches_slice_for_whitespace_patterns() {
    let mut output = [0u8; 16];
    let written = STANDARD
        .decode_slice_legacy(b" aG\r\nVs\tbG8= ", &mut output)
        .unwrap();
    assert_eq!(&output[..written], b"hello");

    let mut standard = *b" aG\r\nVs\tbG8= ";
    let decoded = STANDARD.decode_in_place_legacy(&mut standard).unwrap();
    assert_eq!(decoded, b"hello");

    let mut standard_no_pad = *b" aG\r\nVs\tbG8 ";
    let decoded = STANDARD_NO_PAD
        .decode_in_place_legacy(&mut standard_no_pad)
        .unwrap();
    assert_eq!(decoded, b"hello");

    let mut url_safe_no_pad = *b" - _ 8 ";
    let decoded = URL_SAFE_NO_PAD
        .decode_in_place_legacy(&mut url_safe_no_pad)
        .unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

#[test]
fn legacy_decode_in_place_clear_tail_scrubs_unused_bytes() {
    let mut standard = *b" aG\r\nk= ";
    let len = {
        let decoded = STANDARD
            .decode_in_place_legacy_clear_tail(&mut standard)
            .unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard[len..], &[0; 6]);

    let mut standard_no_pad = *b" aG\r\nk ";
    let len = {
        let decoded = STANDARD_NO_PAD
            .decode_in_place_legacy_clear_tail(&mut standard_no_pad)
            .unwrap();
        assert_eq!(decoded, b"hi");
        decoded.len()
    };
    assert_eq!(&standard_no_pad[len..], &[0; 5]);
}

#[test]
fn legacy_decode_in_place_clear_tail_scrubs_buffer_on_error() {
    let mut invalid_byte = *b" A A - A";
    assert_eq!(
        STANDARD.decode_in_place_legacy_clear_tail(&mut invalid_byte),
        Err(DecodeError::InvalidByte {
            index: 5,
            byte: b'-',
        })
    );
    assert!(invalid_byte.iter().all(|byte| *byte == 0));

    let mut invalid_padding = *b"aGk= \n AAAA";
    assert_eq!(
        STANDARD.decode_in_place_legacy_clear_tail(&mut invalid_padding),
        Err(DecodeError::InvalidPadding { index: 7 })
    );
    assert!(invalid_padding.iter().all(|byte| *byte == 0));
}

#[test]
fn legacy_in_place_rejects_with_original_indexes() {
    let mut invalid_byte = *b" A A - A";
    assert_eq!(
        STANDARD.decode_in_place_legacy(&mut invalid_byte),
        Err(DecodeError::InvalidByte {
            index: 5,
            byte: b'-',
        })
    );

    let mut invalid_padding = *b"aGk= \n AAAA";
    assert_eq!(
        STANDARD.decode_in_place_legacy(&mut invalid_padding),
        Err(DecodeError::InvalidPadding { index: 7 })
    );
}

#[test]
fn reports_absolute_invalid_byte_indexes() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice(b"Zm9v$g==", &mut output),
        Err(DecodeError::InvalidByte {
            index: 4,
            byte: b'$',
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9vYg$", &mut output),
        Err(DecodeError::InvalidByte {
            index: 6,
            byte: b'$',
        })
    );

    let mut input = *b"Zm9vYg$";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidByte {
            index: 6,
            byte: b'$',
        })
    );
}

#[test]
fn rejects_mixed_alphabet_bytes() {
    let mut output = [0u8; 4];
    assert_eq!(
        STANDARD.decode_slice(b"AA-A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"AA_A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'_',
        })
    );
    assert_eq!(
        URL_SAFE.decode_slice(b"AA+A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'+',
        })
    );
    assert_eq!(
        URL_SAFE_NO_PAD.decode_slice(b"AA/A", &mut output),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'/',
        })
    );

    let mut standard_input = *b"AA-A";
    assert_eq!(
        STANDARD.decode_in_place(&mut standard_input),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'-',
        })
    );

    let mut url_safe_input = *b"AA+A";
    assert_eq!(
        URL_SAFE.decode_in_place(&mut url_safe_input),
        Err(DecodeError::InvalidByte {
            index: 2,
            byte: b'+',
        })
    );
}

#[test]
fn reports_absolute_padding_indexes() {
    let mut output = [0u8; 16];
    assert_eq!(
        STANDARD.decode_slice(b"Zm9vZh==", &mut output),
        Err(DecodeError::InvalidPadding { index: 5 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9vZh", &mut output),
        Err(DecodeError::InvalidPadding { index: 5 })
    );

    let mut input = *b"Zm9vZh";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidPadding { index: 5 })
    );
}

#[test]
fn rejects_non_canonical_trailing_bits() {
    let mut output = [0u8; 4];
    assert_eq!(
        STANDARD.decode_slice(b"Zh==", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD.decode_slice(b"Zm9=", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zh", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(b"Zm9", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        URL_SAFE.decode_slice(b"-_9=", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
    assert_eq!(
        URL_SAFE_NO_PAD.decode_slice(b"-_9", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );

    let mut input = *b"Zm9";
    assert_eq!(
        STANDARD_NO_PAD.decode_in_place(&mut input),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
}

#[cfg(feature = "alloc")]
#[test]
fn alloc_helpers_round_trip() {
    let encoded = STANDARD.encode_vec(b"hello").unwrap();
    assert_eq!(encoded, b"aGVsbG8=");

    let encoded_string = STANDARD.encode_string(b"hello").unwrap();
    assert_eq!(encoded_string, "aGVsbG8=");

    let url_safe_string = URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap();
    assert_eq!(url_safe_string, "-_8");

    let decoded = STANDARD.decode_vec(&encoded).unwrap();
    assert_eq!(decoded, b"hello");

    assert_eq!(
        STANDARD_NO_PAD.decode_vec(b"Zm8=").unwrap_err(),
        DecodeError::InvalidPadding { index: 3 }
    );
    assert_eq!(
        STANDARD.decode_vec(b"Zm9v$g==").unwrap_err(),
        DecodeError::InvalidByte {
            index: 4,
            byte: b'$',
        }
    );
    assert_eq!(
        STANDARD.decode_vec(b"Zm9vZh==").unwrap_err(),
        DecodeError::InvalidPadding { index: 5 }
    );
    assert_eq!(
        STANDARD_NO_PAD.decode_vec(b"Zm9vYg$").unwrap_err(),
        DecodeError::InvalidByte {
            index: 6,
            byte: b'$',
        }
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_handles_chunk_boundaries() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    encoder.write_all(b"h").unwrap();
    encoder.write_all(b"el").unwrap();
    encoder.write_all(b"lo").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_supports_no_padding() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD_NO_PAD);
    encoder.write_all(b"he").unwrap();
    encoder.write_all(b"llo").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_exposes_inner_writer() {
    let mut encoder = Encoder::new(Vec::new(), URL_SAFE_NO_PAD);
    assert!(encoder.get_ref().is_empty());
    encoder.write_all(b"\xfb\xff").unwrap();
    assert!(encoder.get_ref().is_empty());
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"-_8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_into_inner_still_returns_writer() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    encoder.write_all(b"he").unwrap();
    let inner = encoder.into_inner();
    assert!(inner.is_empty());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_small_reads() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
    let mut output = [0u8; 8];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_no_padding() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD_NO_PAD);
    let mut encoded = Vec::new();
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_url_safe() {
    let mut reader = EncoderReader::new(&b"\xfb\xff"[..], URL_SAFE_NO_PAD);
    let mut encoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 2);
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"-_8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_into_inner_still_returns_reader() {
    let reader = EncoderReader::new(&b"hello"[..], STANDARD);
    let inner = reader.into_inner();
    assert_eq!(inner, &b"hello"[..]);
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_fragmented_sources() {
    let input = b"any carnal pleasure.";
    let source = ChunkedReader {
        input,
        max_chunk: 1,
    };
    let mut reader = EncoderReader::new(source, STANDARD);
    let mut encoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        encoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(encoded, STANDARD.encode_vec(input).unwrap());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_handles_chunk_boundaries() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    decoder.write_all(b"GVs").unwrap();
    decoder.write_all(b"bG8=").unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_supports_no_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD_NO_PAD);
    decoder.write_all(b"aGV").unwrap();
    decoder.write_all(b"sbG8").unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_bad_final_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    assert!(decoder.finish().is_err());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_trailing_input_after_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    let err = decoder.write_all(b"aGk=AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_short_trailing_input_after_pending_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"aG").unwrap();
    let err = decoder.write_all(b"k=A").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_exposes_inner_writer_after_refactor() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    assert!(decoder.get_ref().is_empty());
    decoder.write_all(b"aGk=").unwrap();
    assert_eq!(decoder.get_ref(), b"hi");
    let inner = decoder.finish().unwrap();
    assert_eq!(inner, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_into_inner_still_returns_writer() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    let inner = decoder.into_inner();
    assert!(inner.is_empty());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_small_reads() {
    let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
    let mut output = [0u8; 5];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_no_padding() {
    let mut reader = DecoderReader::new(&b"aGVsbG8"[..], STANDARD_NO_PAD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_url_safe() {
    let mut reader = DecoderReader::new(&b"-_8"[..], URL_SAFE_NO_PAD);
    let mut decoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 3);
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_rejects_bad_final_pending_input() {
    let mut reader = DecoderReader::new(&b"a"[..], STANDARD);
    let mut decoded = Vec::new();
    assert!(reader.read_to_end(&mut decoded).is_err());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_into_inner_still_returns_reader() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGVsbG8="[..]), STANDARD);
    let mut output = [0u8; 1];
    let read = reader.read(&mut output).unwrap();
    assert_eq!(read, 1);
    assert_eq!(output, [b'h']);

    let inner = reader.into_inner();
    assert_eq!(inner.position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_leaves_trailing_input_after_padding_unread() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=AA"[..]), STANDARD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hi");
    assert_eq!(reader.get_ref().position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_leaves_adjacent_payload_unread_after_padding() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=NEXT"[..]), STANDARD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hi");
    assert_eq!(reader.get_ref().position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_fragmented_sources() {
    let encoded = b"YW55IGNhcm5hbCBwbGVhc3VyZS4=";
    let source = ChunkedReader {
        input: encoded,
        max_chunk: 1,
    };
    let mut reader = DecoderReader::new(source, STANDARD);
    let mut decoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        decoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(decoded, b"any carnal pleasure.");
}

fn assert_round_trip<A, const PAD: bool>(engine: &base64_ng::Engine<A, PAD>, input: &[u8])
where
    A: base64_ng::Alphabet,
{
    let mut encoded = [0u8; 4];
    let encoded_len = engine.encode_slice(input, &mut encoded).unwrap();
    let mut decoded = [0u8; 2];
    let decoded_len = engine
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    assert_eq!(&decoded[..decoded_len], input);
}

fn assert_ct_round_trip<A, const PAD: bool>(
    engine: &base64_ng::Engine<A, PAD>,
    ct_engine: &base64_ng::ct::CtEngine<A, PAD>,
    input: &[u8],
) where
    A: base64_ng::Alphabet,
{
    let mut encoded = [0u8; 4];
    let encoded_len = engine.encode_slice(input, &mut encoded).unwrap();
    let mut decoded = [0u8; 2];
    let decoded_len = ct_engine
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    assert_eq!(&decoded[..decoded_len], input);
}

fn assert_in_place_encode_matches_slice<A, const PAD: bool>(
    engine: &base64_ng::Engine<A, PAD>,
    input: &[u8],
) where
    A: base64_ng::Alphabet,
{
    let required = engine.encoded_len(input.len()).unwrap();
    let mut expected = [0u8; 4];
    let expected_len = engine.encode_slice(input, &mut expected).unwrap();
    assert_eq!(required, expected_len);

    let mut buffer = [0u8; 4];
    buffer[..input.len()].copy_from_slice(input);
    let encoded = engine
        .encode_in_place(&mut buffer[..required], input.len())
        .unwrap();
    assert_eq!(encoded, &expected[..expected_len]);
}

fn assert_equivalent_round_trip<A, const PAD: bool>(
    engine: &base64_ng::Engine<A, PAD>,
    input: &[u8],
) where
    A: base64_ng::Alphabet,
{
    let encoded_len = engine.encoded_len(input.len()).unwrap();
    let mut encoded = vec![0u8; encoded_len];
    let written = engine.encode_slice(input, &mut encoded).unwrap();
    assert_eq!(written, encoded_len);

    let mut in_place_encode = vec![0u8; encoded_len];
    in_place_encode[..input.len()].copy_from_slice(input);
    let in_place_encoded = engine
        .encode_in_place(&mut in_place_encode, input.len())
        .unwrap();
    assert_eq!(in_place_encoded, encoded);

    let mut decoded = vec![0u8; input.len()];
    let decoded_len = engine.decode_slice(&encoded, &mut decoded).unwrap();
    assert_eq!(decoded_len, input.len());
    assert_eq!(decoded, input);

    let mut in_place_decode = encoded;
    let in_place_decoded = engine.decode_in_place(&mut in_place_decode).unwrap();
    assert_eq!(in_place_decoded, input);
}

fn fill_deterministic(output: &mut [u8], seed: u64) {
    let mut state = seed ^ 0x243f_6a88_85a3_08d3;
    for byte in output {
        state = state
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(0xbf58_476d_1ce4_e5b9);
        *byte = (state >> 56) as u8;
    }
}

fn is_standard_alphabet_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/')
}
