use base64_ng::{DecodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

fn fill_pattern(output: &mut [u8]) {
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::try_from((index * 29 + 17) % 251).unwrap();
    }
}

#[test]
fn strict_decode_surfaces_match_expected_for_simd_sized_inputs() {
    let mut input = [0u8; 96];
    fill_pattern(&mut input);

    let mut encoded = [0u8; 128];
    let encoded_len = STANDARD.encode_slice(&input, &mut encoded).unwrap();
    let mut decoded = [0u8; 96];
    let decoded_len = STANDARD
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    assert_eq!(decoded_len, input.len());
    assert_eq!(decoded, input);

    let encoded_len = URL_SAFE_NO_PAD.encode_slice(&input, &mut encoded).unwrap();
    let mut decoded = [0u8; 96];
    let decoded_len = URL_SAFE_NO_PAD
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .unwrap();
    assert_eq!(decoded_len, input.len());
    assert_eq!(decoded, input);
}

#[test]
fn strict_decode_keeps_public_error_shape_for_simd_sized_inputs() {
    let mut encoded = [0u8; 128];
    let mut input = [0u8; 96];
    fill_pattern(&mut input);
    let encoded_len = STANDARD.encode_slice(&input, &mut encoded).unwrap();

    encoded[37] = b'!';
    let mut decoded = [0u8; 96];
    assert_eq!(
        STANDARD.decode_slice(&encoded[..encoded_len], &mut decoded),
        Err(DecodeError::InvalidByte {
            index: 37,
            byte: b'!',
        })
    );

    let encoded_len = URL_SAFE.encode_slice(&input, &mut encoded).unwrap();
    encoded[63] = b'/';
    assert_eq!(
        URL_SAFE.decode_slice(&encoded[..encoded_len], &mut decoded),
        Err(DecodeError::InvalidByte {
            index: 63,
            byte: b'/',
        })
    );

    let encoded_len = STANDARD_NO_PAD
        .encode_slice(&input[..95], &mut encoded)
        .unwrap();
    encoded[encoded_len - 1] = b'!';
    assert_eq!(
        STANDARD_NO_PAD.decode_slice(&encoded[..encoded_len], &mut decoded),
        Err(DecodeError::InvalidByte {
            index: encoded_len - 1,
            byte: b'!',
        })
    );
}

#[cfg(all(
    feature = "simd",
    feature = "std",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[test]
fn runtime_report_exposes_strict_decode_backend() {
    let expected = if std::is_x86_feature_detected!("avx512f")
        && std::is_x86_feature_detected!("avx512bw")
        && std::is_x86_feature_detected!("avx512vl")
        && std::is_x86_feature_detected!("avx512vbmi")
    {
        base64_ng::runtime::Backend::Avx512Vbmi
    } else if std::is_x86_feature_detected!("avx2") {
        base64_ng::runtime::Backend::Avx2
    } else if std::is_x86_feature_detected!("ssse3") && std::is_x86_feature_detected!("sse4.1") {
        base64_ng::runtime::Backend::Ssse3Sse41
    } else {
        base64_ng::runtime::Backend::Scalar
    };

    assert_eq!(
        base64_ng::runtime::backend_report().active_decode_backend(),
        expected
    );
}

#[cfg(all(
    feature = "simd",
    feature = "std",
    target_arch = "aarch64",
    target_endian = "little"
))]
#[test]
fn runtime_report_exposes_aarch64_strict_decode_backend() {
    assert_eq!(
        base64_ng::runtime::backend_report().active_decode_backend(),
        base64_ng::runtime::Backend::Neon
    );
}
