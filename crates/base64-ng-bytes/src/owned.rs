use alloc::vec::Vec;

use base64_ng::{Base64, Codec, OneShotError, Status};
use bytes::{Buf, Bytes};

use crate::{BytesDecoder, BytesEncoder, BytesError, BytesErrorKind, BytesLimits, BytesProgress};

pub(crate) fn encode<S, B>(
    codec: &Base64<S>,
    mut input: B,
    limits: BytesLimits,
) -> Result<Bytes, BytesError>
where
    S: Codec,
    B: Buf,
{
    let input_len = input.remaining();
    require_input_limit(input_len, limits)?;
    let required = codec.encoded_len(input_len).map_err(map_length_error)?;
    if required > limits.max_output_len() {
        return Err(BytesError::new(
            BytesProgress::ZERO,
            BytesErrorKind::OutputLimitExceeded {
                limit: limits.max_output_len(),
            },
        ));
    }

    let mut output = reserve_output(required)?;
    let mut encoder = BytesEncoder::new(codec.encoder(), limits);
    let update = encoder.update(&mut input, &mut output)?;
    if !matches!(update.status(), Status::NeedInput) || input.has_remaining() {
        return Err(BytesError::new(
            update.progress(),
            BytesErrorKind::ImpossibleState,
        ));
    }
    let finish = encoder.finish(&mut output)?;
    if !matches!(finish.status(), Status::Complete) || output.len() != required {
        return Err(BytesError::new(
            finish.progress(),
            BytesErrorKind::ImpossibleState,
        ));
    }
    Ok(Bytes::from(output))
}

pub(crate) fn decode<S, B>(
    codec: &Base64<S>,
    mut input: B,
    limits: BytesLimits,
) -> Result<Bytes, BytesError>
where
    S: Codec,
    B: Buf,
{
    let input_len = input.remaining();
    require_input_limit(input_len, limits)?;

    let reserve = input_len.min(limits.max_output_len());
    let mut output = reserve_output(reserve)?;
    let mut decoder = BytesDecoder::new(codec.decoder(), limits);
    let update = decoder.update(&mut input, &mut output)?;
    if !matches!(update.status(), Status::NeedInput) || input.has_remaining() {
        return Err(BytesError::new(
            update.progress(),
            BytesErrorKind::ImpossibleState,
        ));
    }
    let finish = decoder.finish(&mut output)?;
    if !matches!(finish.status(), Status::Complete) {
        return Err(BytesError::new(
            finish.progress(),
            BytesErrorKind::ImpossibleState,
        ));
    }
    Ok(Bytes::from(output))
}

fn require_input_limit(input_len: usize, limits: BytesLimits) -> Result<(), BytesError> {
    if input_len > limits.max_input_len() {
        Err(BytesError::new(
            BytesProgress::ZERO,
            BytesErrorKind::InputLimitExceeded {
                required: input_len,
                limit: limits.max_input_len(),
            },
        ))
    } else {
        Ok(())
    }
}

fn reserve_output(capacity: usize) -> Result<Vec<u8>, BytesError> {
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| {
        BytesError::new(
            BytesProgress::ZERO,
            BytesErrorKind::AllocationFailed {
                requested: capacity,
            },
        )
    })?;
    Ok(output)
}

fn map_length_error(error: OneShotError) -> BytesError {
    let kind = match error {
        OneShotError::LengthOverflow => BytesErrorKind::LengthOverflow,
        _ => BytesErrorKind::ImpossibleState,
    };
    BytesError::new(BytesProgress::ZERO, kind)
}
