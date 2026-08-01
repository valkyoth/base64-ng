#![no_main]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use base64::{Engine as _, engine::general_purpose};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use base64_ng::{
    Engine, Standard, StaticBackendToken, UrlSafe, decoded_capacity, runtime::Backend,
};
use libfuzzer_sys::fuzz_target;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const MAX_INPUT_LEN: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let input = &data[..data.len().min(MAX_INPUT_LEN)];
        if std::is_x86_feature_detected!("ssse3") && std::is_x86_feature_detected!("sse4.1") {
            compare_backend(Backend::Ssse3Sse41, input);
        }
        if std::is_x86_feature_detected!("avx2") {
            compare_backend(Backend::Avx2, input);
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    let _ = data;
});

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_backend(backend: Backend, input: &[u8]) {
    // SAFETY: each caller checks the complete runtime feature bundle before
    // constructing this thread-bound token, and uses it on the same thread.
    let token = unsafe { StaticBackendToken::assume_supported(backend) }
        .expect("runtime-supported x86 backend must pass its admission KAT");

    compare_standard::<true>(&token, input, Engine::<Standard, true>::new());
    compare_standard::<false>(&token, input, Engine::<Standard, false>::new());
    compare_url_safe::<true>(&token, input, Engine::<UrlSafe, true>::new());
    compare_url_safe::<false>(&token, input, Engine::<UrlSafe, false>::new());

    let raw = &input[..input.len().min(4096)];
    compare_valid_standard::<true>(&token, raw, &general_purpose::STANDARD);
    compare_valid_standard::<false>(&token, raw, &general_purpose::STANDARD_NO_PAD);
    compare_valid_url_safe::<true>(&token, raw, &general_purpose::URL_SAFE);
    compare_valid_url_safe::<false>(&token, raw, &general_purpose::URL_SAFE_NO_PAD);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_standard<const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    reference: Engine<Standard, PAD>,
) {
    compare_results(
        input,
        |input, output| token.decode_standard::<PAD>(input, output),
        |input, output| reference.decode_slice(input, output),
    );
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_url_safe<const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    reference: Engine<UrlSafe, PAD>,
) {
    compare_results(
        input,
        |input, output| token.decode_url_safe::<PAD>(input, output),
        |input, output| reference.decode_slice(input, output),
    );
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_results(
    input: &[u8],
    accelerated: impl Fn(&[u8], &mut [u8]) -> Result<usize, base64_ng::DecodeError>,
    reference: impl Fn(&[u8], &mut [u8]) -> Result<usize, base64_ng::DecodeError>,
) {
    let capacity = decoded_capacity(input.len());
    let mut accelerated_output = vec![0x55; capacity];
    let mut reference_output = vec![0xaa; capacity];
    let accelerated_result = accelerated(input, &mut accelerated_output);
    let reference_result = reference(input, &mut reference_output);
    assert_eq!(accelerated_result, reference_result);
    match reference_result {
        Ok(written) => assert_eq!(
            &accelerated_output[..written],
            &reference_output[..written]
        ),
        Err(_) => assert!(accelerated_output.iter().all(|byte| *byte == 0x55)),
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_valid_standard<const PAD: bool>(
    token: &StaticBackendToken,
    raw: &[u8],
    reference: &general_purpose::GeneralPurpose,
) {
    let encoded = reference.encode(raw);
    let mut output = vec![0u8; raw.len()];
    let written = token
        .decode_standard::<PAD>(encoded.as_bytes(), &mut output)
        .unwrap();
    assert_eq!(&output[..written], raw);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_valid_url_safe<const PAD: bool>(
    token: &StaticBackendToken,
    raw: &[u8],
    reference: &general_purpose::GeneralPurpose,
) {
    let encoded = reference.encode(raw);
    let mut output = vec![0u8; raw.len()];
    let written = token
        .decode_url_safe::<PAD>(encoded.as_bytes(), &mut output)
        .unwrap();
    assert_eq!(&output[..written], raw);
}
