use base64::Engine as _;
use base64_ng::{Base64, Codec, EncodePadding};
use base64ct::Encoding as _;

pub fn apply<S: Codec>(
    codec: &Base64<S>,
    operation: &str,
    encode: bool,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, ()> {
    let settings = codec.settings();
    let padded = settings.encode_padding() == EncodePadding::Padded;
    let url = settings.alphabet().as_array()[62] == b'-';
    if operation == "base64" {
        use base64::engine::general_purpose::*;
        let external = match (url, padded) {
            (false, true) => STANDARD,
            (false, false) => STANDARD_NO_PAD,
            (true, true) => URL_SAFE,
            (true, false) => URL_SAFE_NO_PAD,
        };
        return if encode {
            external.encode_slice(input, output).map_err(|_| ())
        } else {
            external.decode_slice(input, output).map_err(|_| ())
        };
    }
    macro_rules! run {
        ($ty:ty) => {
            if encode {
                <$ty>::encode(input, output)
                    .map(|s| s.len())
                    .map_err(|_| ())
            } else {
                <$ty>::decode(input, output)
                    .map(|s| s.len())
                    .map_err(|_| ())
            }
        };
    }
    match (url, padded) {
        (false, true) => run!(base64ct::Base64),
        (false, false) => run!(base64ct::Base64Unpadded),
        (true, true) => run!(base64ct::Base64Url),
        (true, false) => run!(base64ct::Base64UrlUnpadded),
    }
}

#[cfg(feature = "std")]
pub fn exact<S: Codec>(
    codec: &Base64<S>,
    name: &str,
    encode: bool,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, ()> {
    use base64_ng::perf_evidence::{self as perf, EvidenceBackend};
    let backend = EvidenceBackend::ALL
        .into_iter()
        .find(|b| b.as_str() == name)
        .ok_or(())?;
    let settings = codec.settings();
    let padded = settings.encode_padding() == EncodePadding::Padded;
    let url = settings.alphabet().as_array()[62] == b'-';
    macro_rules! run {
        ($pad:literal) => {
            (if url {
                perf::encode_url_safe::<$pad>(backend, input, output)
            } else {
                perf::encode_standard::<$pad>(backend, input, output)
            })
            .ok_or(())?
            .map_err(|_| ())
        };
    }
    // Encode and decode have distinct error types.
    if encode {
        return if padded { run!(true) } else { run!(false) };
    }
    let result = match (url, padded) {
        (false, true) => perf::decode_standard::<true>(backend, input, output),
        (false, false) => perf::decode_standard::<false>(backend, input, output),
        (true, true) => perf::decode_url_safe::<true>(backend, input, output),
        (true, false) => perf::decode_url_safe::<false>(backend, input, output),
    };
    result.ok_or(())?.map_err(|_| ())
}
