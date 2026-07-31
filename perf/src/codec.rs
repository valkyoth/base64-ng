use base64::Engine as _;
use base64::engine::general_purpose;
use base64_ng::perf_evidence::{self, EvidenceBackend};
use base64ct::{
    Base64, Base64Unpadded, Base64Url, Base64UrlUnpadded, Encoding as Base64CtEncoding,
};

#[derive(Clone, Copy)]
pub enum Profile {
    StandardPadded,
    StandardUnpadded,
    UrlSafePadded,
    UrlSafeUnpadded,
}

impl Profile {
    pub const ALL: [Self; 4] = [
        Self::StandardPadded,
        Self::StandardUnpadded,
        Self::UrlSafePadded,
        Self::UrlSafeUnpadded,
    ];

    pub const fn alphabet(self) -> &'static str {
        match self {
            Self::StandardPadded | Self::StandardUnpadded => "standard",
            Self::UrlSafePadded | Self::UrlSafeUnpadded => "url-safe",
        }
    }

    pub const fn padding(self) -> &'static str {
        match self {
            Self::StandardPadded | Self::UrlSafePadded => "padded",
            Self::StandardUnpadded | Self::UrlSafeUnpadded => "unpadded",
        }
    }

    pub fn encoded_len(self, input_len: usize) -> usize {
        base64_ng::checked_encoded_len(
            input_len,
            matches!(self, Self::StandardPadded | Self::UrlSafePadded),
        )
        .expect("performance input length fits")
    }
}

pub fn make_input(len: usize) -> Vec<u8> {
    let mut output = vec![0u8; len];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = index.wrapping_mul(37).wrapping_add(len) as u8;
    }
    output
}

pub fn canonical_encoded(profile: Profile, input: &[u8]) -> Vec<u8> {
    let mut output = vec![0u8; profile.encoded_len(input.len())];
    let written = encode(
        "base64-ng",
        Some(EvidenceBackend::Scalar),
        profile,
        input,
        &mut output,
    );
    output.truncate(written);
    output
}

pub fn encode(
    engine: &str,
    backend: Option<EvidenceBackend>,
    profile: Profile,
    input: &[u8],
    output: &mut [u8],
) -> usize {
    match engine {
        "base64-ng" => encode_ng(backend.expect("base64-ng backend"), profile, input, output),
        "base64-0.23.0" => base64_engine(profile)
            .encode_slice(input, output)
            .expect("base64 encode succeeds"),
        "base64ct-1.8.3" => encode_base64ct(profile, input, output),
        _ => panic!("unknown benchmark engine"),
    }
}

pub fn decode(
    engine: &str,
    backend: Option<EvidenceBackend>,
    profile: Profile,
    input: &[u8],
    output: &mut [u8],
) -> usize {
    match engine {
        "base64-ng" => decode_ng(backend.expect("base64-ng backend"), profile, input, output),
        "base64-0.23.0" => base64_engine(profile)
            .decode_slice(input, output)
            .expect("base64 decode succeeds"),
        "base64ct-1.8.3" => decode_base64ct(profile, input, output),
        _ => panic!("unknown benchmark engine"),
    }
}

fn encode_ng(backend: EvidenceBackend, profile: Profile, input: &[u8], output: &mut [u8]) -> usize {
    let result = match profile {
        Profile::StandardPadded => perf_evidence::encode_standard::<true>(backend, input, output),
        Profile::StandardUnpadded => {
            perf_evidence::encode_standard::<false>(backend, input, output)
        }
        Profile::UrlSafePadded => perf_evidence::encode_url_safe::<true>(backend, input, output),
        Profile::UrlSafeUnpadded => perf_evidence::encode_url_safe::<false>(backend, input, output),
    };
    result
        .expect("requested base64-ng backend is available")
        .expect("base64-ng encode succeeds")
}

fn decode_ng(backend: EvidenceBackend, profile: Profile, input: &[u8], output: &mut [u8]) -> usize {
    let result = match profile {
        Profile::StandardPadded => perf_evidence::decode_standard::<true>(backend, input, output),
        Profile::StandardUnpadded => {
            perf_evidence::decode_standard::<false>(backend, input, output)
        }
        Profile::UrlSafePadded => perf_evidence::decode_url_safe::<true>(backend, input, output),
        Profile::UrlSafeUnpadded => perf_evidence::decode_url_safe::<false>(backend, input, output),
    };
    result
        .expect("requested base64-ng backend is available")
        .expect("base64-ng decode succeeds")
}

fn base64_engine(profile: Profile) -> general_purpose::GeneralPurpose {
    match profile {
        Profile::StandardPadded => general_purpose::STANDARD,
        Profile::StandardUnpadded => general_purpose::STANDARD_NO_PAD,
        Profile::UrlSafePadded => general_purpose::URL_SAFE,
        Profile::UrlSafeUnpadded => general_purpose::URL_SAFE_NO_PAD,
    }
}

fn encode_base64ct(profile: Profile, input: &[u8], output: &mut [u8]) -> usize {
    match profile {
        Profile::StandardPadded => Base64::encode(input, output),
        Profile::StandardUnpadded => Base64Unpadded::encode(input, output),
        Profile::UrlSafePadded => Base64Url::encode(input, output),
        Profile::UrlSafeUnpadded => Base64UrlUnpadded::encode(input, output),
    }
    .expect("base64ct encode succeeds")
    .len()
}

fn decode_base64ct(profile: Profile, input: &[u8], output: &mut [u8]) -> usize {
    match profile {
        Profile::StandardPadded => Base64::decode(input, output),
        Profile::StandardUnpadded => Base64Unpadded::decode(input, output),
        Profile::UrlSafePadded => Base64Url::decode(input, output),
        Profile::UrlSafeUnpadded => Base64UrlUnpadded::decode(input, output),
    }
    .expect("base64ct decode succeeds")
    .len()
}
