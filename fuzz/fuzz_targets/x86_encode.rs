#![no_main]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use base64::{Engine as _, engine::general_purpose};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use base64_ng::{StaticBackendToken, checked_encoded_len, runtime::Backend};
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
        if std::is_x86_feature_detected!("avx512f")
            && std::is_x86_feature_detected!("avx512bw")
            && std::is_x86_feature_detected!("avx512vl")
            && std::is_x86_feature_detected!("avx512vbmi")
        {
            compare_backend(Backend::Avx512Vbmi, input);
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

    compare_standard::<true>(&token, input, &general_purpose::STANDARD);
    compare_standard::<false>(&token, input, &general_purpose::STANDARD_NO_PAD);
    compare_url_safe::<true>(&token, input, &general_purpose::URL_SAFE);
    compare_url_safe::<false>(&token, input, &general_purpose::URL_SAFE_NO_PAD);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_standard<const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    reference: &general_purpose::GeneralPurpose,
) {
    let mut output = vec![0u8; checked_encoded_len(input.len(), PAD).unwrap()];
    let written = token.encode_standard::<PAD>(input, &mut output).unwrap();
    output.truncate(written);
    assert_eq!(output, reference.encode(input).as_bytes());
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn compare_url_safe<const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    reference: &general_purpose::GeneralPurpose,
) {
    let mut output = vec![0u8; checked_encoded_len(input.len(), PAD).unwrap()];
    let written = token.encode_url_safe::<PAD>(input, &mut output).unwrap();
    output.truncate(written);
    assert_eq!(output, reference.encode(input).as_bytes());
}
