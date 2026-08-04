use base64_ng::checked_encoded_len;

/// Default decoded-byte ceiling for dynamic secret convenience methods.
pub const DEFAULT_SECRET_VEC_DECODE_MAX_LEN: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EncodedInputLimit {
    pub(crate) maximum: usize,
    pub(crate) actual: usize,
}

#[inline]
pub(crate) fn enforce_encoded_input_limit<const PAD: bool>(
    decoded_limit: usize,
    input_len: usize,
) -> Result<(), EncodedInputLimit> {
    let maximum = checked_encoded_len(decoded_limit, PAD).unwrap_or(usize::MAX);
    if input_len > maximum {
        return Err(EncodedInputLimit {
            maximum,
            actual: input_len,
        });
    }
    Ok(())
}
