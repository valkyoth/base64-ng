use alloc::{string::String, vec::Vec};
use core::fmt;

#[cfg(feature = "secrets")]
use base64_ng::clear_bytes;
#[cfg(feature = "secrets")]
use base64_ng::secret::{SecretArray, SecretArrayFrame, SecretInput};
use base64_ng::{
    Base64, BodyCodec, Codec, DecodedArray, Failure, OneShotError, OperationError, Status,
};
use serde::{Deserializer, Serializer, de::Visitor};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdapterError {
    InvalidInput(&'static str),
    InvalidBodyLayout,
    OutputLimit,
    AllocationFailed,
    InternalFailure,
    #[cfg(feature = "secrets")]
    InvalidSecretInput,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(kind) => write!(formatter, "invalid base64 input ({kind})"),
            Self::InvalidBodyLayout => formatter.write_str("invalid base64 body layout"),
            Self::OutputLimit => formatter.write_str("base64 output exceeds configured limit"),
            Self::AllocationFailed => formatter.write_str("base64 output allocation failed"),
            Self::InternalFailure => formatter.write_str("base64 transform failed"),
            #[cfg(feature = "secrets")]
            Self::InvalidSecretInput => formatter.write_str("invalid secret base64 input"),
        }
    }
}

fn map_one_shot(error: OneShotError) -> AdapterError {
    match error {
        OneShotError::Input(error) => AdapterError::InvalidInput(error.kind().as_str()),
        OneShotError::OutputTooSmall { .. } | OneShotError::AllocationLimitExceeded { .. } => {
            AdapterError::OutputLimit
        }
        OneShotError::AllocationFailed { .. } => AdapterError::AllocationFailed,
        _ => AdapterError::InternalFailure,
    }
}

fn serialize_text<S>(encoded: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        serializer.serialize_str(encoded)
    } else {
        serializer.serialize_bytes(encoded.as_bytes())
    }
}

pub(crate) fn serialize_codec<C, S>(
    codec: &Base64<C>,
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    C: Codec,
    S: Serializer,
{
    let encoded = codec
        .encode_to_string(bytes)
        .map_err(serde::ser::Error::custom)?;
    serialize_text(&encoded, serializer)
}

pub(crate) fn serialize_body<C, S>(
    body: &BodyCodec<C>,
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    C: Codec,
    S: Serializer,
{
    let encoded = encode_body(body, bytes).map_err(serde::ser::Error::custom)?;
    serialize_text(&encoded, serializer)
}

#[cfg(feature = "secrets")]
pub(crate) fn serialize_secret_codec<C, S>(
    codec: &Base64<C>,
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    C: Codec,
    S: Serializer,
{
    let mut encoded = codec
        .encode_to_string(bytes)
        .map_err(serde::ser::Error::custom)?
        .into_bytes();
    let result = match core::str::from_utf8(&encoded) {
        Ok(text) => serialize_text(text, serializer),
        Err(_) => Err(serde::ser::Error::custom("base64 transform failed")),
    };
    clear_bytes(&mut encoded);
    result
}

pub(crate) fn deserialize_vec<'de, C, D>(
    codec: &Base64<C>,
    deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
    C: Codec,
    D: Deserializer<'de>,
{
    deserialize_input(deserializer, InputVisitor(VecDecoder { codec }))
}

pub(crate) fn deserialize_body_vec<'de, C, D>(
    body: &BodyCodec<C>,
    deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
    C: Codec,
    D: Deserializer<'de>,
{
    deserialize_input(deserializer, InputVisitor(BodyVecDecoder { body }))
}

pub(crate) fn deserialize_bounded<'de, C, D, const CAP: usize>(
    codec: &Base64<C>,
    deserializer: D,
) -> Result<DecodedArray<CAP>, D::Error>
where
    C: Codec,
    D: Deserializer<'de>,
{
    deserialize_input(
        deserializer,
        InputVisitor(BoundedDecoder::<C, CAP> { codec }),
    )
}

pub(crate) fn deserialize_body_bounded<'de, C, D, const CAP: usize>(
    body: &BodyCodec<C>,
    deserializer: D,
) -> Result<DecodedArray<CAP>, D::Error>
where
    C: Codec,
    D: Deserializer<'de>,
{
    deserialize_input(
        deserializer,
        InputVisitor(BodyBoundedDecoder::<C, CAP> { body }),
    )
}

#[cfg(feature = "secrets")]
pub(crate) fn deserialize_secret<'de, C, D, const CAP: usize>(
    codec: &Base64<C>,
    deserializer: D,
) -> Result<SecretArray<CAP>, D::Error>
where
    C: Codec,
    D: Deserializer<'de>,
{
    deserialize_input(
        deserializer,
        InputVisitor(SecretDecoder::<C, CAP> { codec }),
    )
}

fn deserialize_input<'de, D, V>(deserializer: D, visitor: V) -> Result<V::Value, D::Error>
where
    D: Deserializer<'de>,
    V: Visitor<'de>,
{
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(visitor)
    } else {
        deserializer.deserialize_bytes(visitor)
    }
}

trait DecodeInput {
    type Output;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError>;
}

struct InputVisitor<T>(T);

impl<'de, T: DecodeInput> Visitor<'de> for InputVisitor<T> {
    type Value = T::Output;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Base64 text as a string or byte string")
    }

    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.0.decode(value.as_bytes()).map_err(E::custom)
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.0.decode(value.as_bytes()).map_err(E::custom)
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.0.decode(value.as_bytes()).map_err(E::custom)
    }

    fn visit_borrowed_bytes<E: serde::de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        self.0.decode(value).map_err(E::custom)
    }

    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.0.decode(value).map_err(E::custom)
    }

    fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        self.0.decode(&value).map_err(E::custom)
    }
}

struct VecDecoder<'a, C> {
    codec: &'a Base64<C>,
}

impl<C: Codec> DecodeInput for VecDecoder<'_, C> {
    type Output = Vec<u8>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        self.codec.decode_to_vec(input).map_err(map_one_shot)
    }
}

struct BoundedDecoder<'a, C, const CAP: usize> {
    codec: &'a Base64<C>,
}

impl<C: Codec, const CAP: usize> DecodeInput for BoundedDecoder<'_, C, CAP> {
    type Output = DecodedArray<CAP>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        self.codec.decode_bounded(input).map_err(map_one_shot)
    }
}

struct BodyVecDecoder<'a, C> {
    body: &'a BodyCodec<C>,
}

impl<C: Codec> DecodeInput for BodyVecDecoder<'_, C> {
    type Output = Vec<u8>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        let required = body_decoded_len(self.body, input)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(required)
            .map_err(|_| AdapterError::AllocationFailed)?;
        output.resize(required, 0);
        decode_body_into(self.body, input, &mut output)?;
        Ok(output)
    }
}

struct BodyBoundedDecoder<'a, C, const CAP: usize> {
    body: &'a BodyCodec<C>,
}

impl<C: Codec, const CAP: usize> DecodeInput for BodyBoundedDecoder<'_, C, CAP> {
    type Output = DecodedArray<CAP>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        let required = body_decoded_len(self.body, input)?;
        if required > CAP {
            return Err(AdapterError::OutputLimit);
        }
        let mut output = [0u8; CAP];
        let written = decode_body_into(self.body, input, &mut output)?;
        DecodedArray::from_array(output, written).map_err(|_| AdapterError::InternalFailure)
    }
}

#[cfg(feature = "secrets")]
struct SecretDecoder<'a, C, const CAP: usize> {
    codec: &'a Base64<C>,
}

#[cfg(feature = "secrets")]
impl<C: Codec, const CAP: usize> DecodeInput for SecretDecoder<'_, C, CAP> {
    type Output = SecretArray<CAP>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        let mut frame = SecretArrayFrame::<CAP>::new(self.codec)
            .map_err(|_| AdapterError::InvalidSecretInput)?;
        frame
            .update(&SecretInput::new(input))
            .map_err(|_| AdapterError::InvalidSecretInput)?;
        frame.finish().map_err(|_| AdapterError::InvalidSecretInput)
    }
}

fn encode_body<C: Codec>(body: &BodyCodec<C>, input: &[u8]) -> Result<String, AdapterError> {
    let payload_len = body
        .codec()
        .encoded_len(input.len())
        .map_err(map_one_shot)?;
    let required = body
        .wrapping()
        .checked_output_len(payload_len)
        .ok_or(AdapterError::InternalFailure)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| AdapterError::AllocationFailed)?;
    output.resize(required, 0);

    let width = body.wrapping().line_width().get();
    let input_per_line = (width / 4) * 3;
    let mut read = 0usize;
    let mut write = 0usize;
    while input.len().saturating_sub(read) > input_per_line {
        let end = read + input_per_line;
        write += body
            .codec()
            .encode_into(&input[read..end], &mut output[write..write + width])
            .map_err(map_one_shot)?;
        let separator = body.wrapping().line_ending().as_bytes();
        output[write..write + separator.len()].copy_from_slice(separator);
        write += separator.len();
        read = end;
    }
    write += body
        .codec()
        .encode_into(&input[read..], &mut output[write..])
        .map_err(map_one_shot)?;
    debug_assert_eq!(write, required);
    String::from_utf8(output).map_err(|_| AdapterError::InternalFailure)
}

fn body_decoded_len<C: Codec>(body: &BodyCodec<C>, input: &[u8]) -> Result<usize, AdapterError> {
    require_body_layout(body, input)?;
    let mut decoder = body.codec().decoder();
    let mut scratch = [0u8; 3];
    let mut written = 0usize;
    for_payload_bytes(body, input, |byte| {
        let step = decoder
            .update(core::slice::from_ref(&byte), &mut scratch)
            .map_err(map_operation)?;
        let progress = step.progress();
        if progress.input_consumed() != 1 {
            return Err(AdapterError::InternalFailure);
        }
        written = written
            .checked_add(progress.output_produced())
            .ok_or(AdapterError::InternalFailure)?;
        Ok(())
    })?;
    loop {
        let step = decoder.finish(&mut scratch).map_err(map_operation)?;
        let progress = step.progress();
        written = written
            .checked_add(progress.output_produced())
            .ok_or(AdapterError::InternalFailure)?;
        match step.status() {
            Status::Complete => break,
            Status::OutputFull(_) if progress.output_produced() != 0 => {}
            Status::OutputFull(_) | Status::NeedInput => {
                return Err(AdapterError::InternalFailure);
            }
            _ => return Err(AdapterError::InternalFailure),
        }
    }
    Ok(written)
}

fn decode_body_into<C: Codec>(
    body: &BodyCodec<C>,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, AdapterError> {
    require_body_layout(body, input)?;
    let mut decoder = body.codec().decoder();
    let mut written = 0usize;
    for_payload_bytes(body, input, |byte| {
        let step = decoder
            .update(core::slice::from_ref(&byte), &mut output[written..])
            .map_err(map_operation)?;
        let progress = step.progress();
        if progress.input_consumed() != 1 {
            return Err(AdapterError::InternalFailure);
        }
        written = written
            .checked_add(progress.output_produced())
            .ok_or(AdapterError::InternalFailure)?;
        Ok(())
    })?;
    finish_decoder(&mut decoder, output, &mut written)?;
    Ok(written)
}

fn require_body_layout<C>(body: &BodyCodec<C>, input: &[u8]) -> Result<(), AdapterError> {
    body.wrapping()
        .payload_len(input)
        .map(|_| ())
        .ok_or(AdapterError::InvalidBodyLayout)
}

fn for_payload_bytes<C, F>(
    body: &BodyCodec<C>,
    input: &[u8],
    mut consume: F,
) -> Result<(), AdapterError>
where
    F: FnMut(u8) -> Result<(), AdapterError>,
{
    let separator = body.wrapping().line_ending().as_bytes();
    let mut index = 0usize;
    while index < input.len() {
        let separator_end = index.saturating_add(separator.len());
        if separator_end <= input.len() && &input[index..separator_end] == separator {
            index = separator_end;
        } else {
            consume(input[index])?;
            index += 1;
        }
    }
    Ok(())
}

fn finish_decoder(
    decoder: &mut base64_ng::DecoderState,
    output: &mut [u8],
    written: &mut usize,
) -> Result<(), AdapterError> {
    loop {
        let step = decoder
            .finish(&mut output[*written..])
            .map_err(map_operation)?;
        let progress = step.progress();
        *written = written
            .checked_add(progress.output_produced())
            .ok_or(AdapterError::InternalFailure)?;
        match step.status() {
            Status::Complete => return Ok(()),
            Status::OutputFull(_) if progress.output_produced() != 0 => {}
            Status::OutputFull(_) | Status::NeedInput => {
                return Err(AdapterError::InternalFailure);
            }
            _ => return Err(AdapterError::InternalFailure),
        }
    }
}

fn map_operation(error: OperationError) -> AdapterError {
    match error {
        OperationError::Failed(Failure::Input(error)) => {
            AdapterError::InvalidInput(error.kind().as_str())
        }
        _ => AdapterError::InternalFailure,
    }
}
