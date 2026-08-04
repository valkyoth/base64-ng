use base64_ng::{Base64, BodyCodec, Codec};

use crate::adapter::{AdapterError, map_one_shot};

/// Default decoded-byte ceiling for allocating Serde compatibility adapters.
pub const DEFAULT_SERDE_DECODE_MAX_LEN: usize = 1024 * 1024;

/// Maximum decoded capacity supported by stack-backed Serde adapters.
///
/// Larger ordinary values must use an allocating adapter with an explicit
/// limit. This ceiling does not constrain storage allocated by the upstream
/// Serde data format before input reaches this crate. The release gate builds
/// an invalid `4097`-byte instantiation and requires compilation to fail.
pub const MAX_SERDE_STACK_DECODED_BYTES: usize = 4096;

pub(crate) fn enforce_codec_input_limit<C: Codec>(
    codec: &Base64<C>,
    input_len: usize,
    maximum_decoded: usize,
) -> Result<(), AdapterError> {
    let maximum_encoded = codec.encoded_len(maximum_decoded).map_err(map_one_shot)?;
    if input_len > maximum_encoded {
        return Err(AdapterError::OutputLimit);
    }
    Ok(())
}

pub(crate) fn enforce_body_input_limit<C: Codec>(
    body: &BodyCodec<C>,
    input_len: usize,
    maximum_decoded: usize,
) -> Result<(), AdapterError> {
    let maximum_payload = body
        .codec()
        .encoded_len(maximum_decoded)
        .map_err(map_one_shot)?;
    let maximum_body = body
        .wrapping()
        .checked_output_len(maximum_payload)
        .ok_or(AdapterError::InternalFailure)?;
    if input_len > maximum_body {
        return Err(AdapterError::OutputLimit);
    }
    Ok(())
}

#[allow(clippy::manual_assert)]
pub(crate) const fn enforce_stack_capacity<const CAP: usize>() {
    if CAP > MAX_SERDE_STACK_DECODED_BYTES {
        panic!("Serde decoded capacity exceeds the supported 4096-byte stack limit");
    }
}
