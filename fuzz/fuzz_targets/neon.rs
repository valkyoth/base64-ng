#![no_main]

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
use base64::{Engine as _, engine::general_purpose};
#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
use base64_ng::{
    Engine, Standard, StaticBackendToken, UrlSafe, checked_encoded_len, decoded_capacity,
    runtime::Backend,
};
use libfuzzer_sys::fuzz_target;

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
const MAX_INPUT_LEN: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    compare_neon(&data[..data.len().min(MAX_INPUT_LEN)]);

    #[cfg(not(all(target_arch = "aarch64", target_endian = "little")))]
    let _ = data;
});

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn compare_neon(input: &[u8]) {
    // SAFETY: NEON is part of the AArch64 architecture contract and this
    // token remains on the constructing thread.
    let token = unsafe { StaticBackendToken::assume_supported(Backend::Neon) }
        .expect("AArch64 NEON must pass its admission KAT");

    compare_encode::<true>(&token, input, &general_purpose::STANDARD, false);
    compare_encode::<false>(&token, input, &general_purpose::STANDARD_NO_PAD, false);
    compare_encode::<true>(&token, input, &general_purpose::URL_SAFE, true);
    compare_encode::<false>(&token, input, &general_purpose::URL_SAFE_NO_PAD, true);
    compare_decode::<Standard, true>(&token, input, false);
    compare_decode::<Standard, false>(&token, input, false);
    compare_decode::<UrlSafe, true>(&token, input, true);
    compare_decode::<UrlSafe, false>(&token, input, true);
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn compare_encode<const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    reference: &general_purpose::GeneralPurpose,
    url_safe: bool,
) {
    let mut output = vec![0u8; checked_encoded_len(input.len(), PAD).unwrap()];
    let written = if url_safe {
        token.encode_url_safe::<PAD>(input, &mut output).unwrap()
    } else {
        token.encode_standard::<PAD>(input, &mut output).unwrap()
    };
    output.truncate(written);
    assert_eq!(output, reference.encode(input).as_bytes());
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
fn compare_decode<A: base64_ng::Alphabet, const PAD: bool>(
    token: &StaticBackendToken,
    input: &[u8],
    url_safe: bool,
) {
    let mut accelerated = vec![0x55; decoded_capacity(input.len())];
    let mut scalar = vec![0xaa; decoded_capacity(input.len())];
    let accelerated_result = if url_safe {
        token.decode_url_safe::<PAD>(input, &mut accelerated)
    } else {
        token.decode_standard::<PAD>(input, &mut accelerated)
    };
    let scalar_result = Engine::<A, PAD>::new().decode_slice(input, &mut scalar);
    assert_eq!(accelerated_result, scalar_result);
    if let Ok(written) = scalar_result {
        assert_eq!(&accelerated[..written], &scalar[..written]);
    }
}
