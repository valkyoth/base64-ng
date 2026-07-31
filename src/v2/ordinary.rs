//! Transactional ordinary one-shot operations.

use super::{
    contracts::{BackendFault, Failure, InputError, OperationError, Status},
    specifications::{Base64, Codec, CodecSettings, EncodePadding},
};

/// Error returned by a canonical ordinary one-shot operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum OneShotError {
    /// The encoded output length cannot be represented by `usize`.
    LengthOverflow,
    /// Strict validation rejected the ordinary input.
    Input(InputError),
    /// Absolute source positions cannot be represented by `usize`.
    PositionOverflow,
    /// The caller's destination cannot hold the complete result.
    OutputTooSmall {
        /// Exact required output bytes.
        required: usize,
        /// Available destination bytes.
        available: usize,
    },
    /// The exact output exceeds the caller-selected allocation limit.
    AllocationLimitExceeded {
        /// Exact required output bytes.
        required: usize,
        /// Maximum permitted output bytes.
        limit: usize,
    },
    /// `try_reserve_exact` could not reserve the complete output allocation.
    AllocationFailed {
        /// Exact requested output bytes.
        requested: usize,
    },
    /// An internal backend or state invariant failed.
    Backend(BackendFault),
}

impl core::fmt::Display for OneShotError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => formatter.write_str("base64 output length overflows usize"),
            Self::Input(error) => error.fmt(formatter),
            Self::PositionOverflow => formatter.write_str("base64 source position overflows usize"),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                formatter,
                "base64 output buffer too small: required {required}, available {available}"
            ),
            Self::AllocationLimitExceeded { required, limit } => write!(
                formatter,
                "base64 output length {required} exceeds allocation limit {limit}"
            ),
            Self::AllocationFailed { requested } => {
                write!(
                    formatter,
                    "failed to reserve {requested} base64 output bytes"
                )
            }
            Self::Backend(fault) => write!(formatter, "base64 backend fault: {}", fault.as_str()),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OneShotError {}

impl<S: Codec> Base64<S> {
    /// Returns the exact encoded length or reports arithmetic overflow.
    pub fn encoded_len(&self, input_len: usize) -> Result<usize, OneShotError> {
        encoded_len_for_settings(self.specification().settings(), input_len)
    }

    /// Validates ordinary input without producing decoded bytes.
    pub fn validate(&self, input: &[u8]) -> Result<(), OneShotError> {
        self.decoded_len(input).map(|_| ())
    }

    /// Validates `input` and returns its exact decoded length.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, OneShotError> {
        validate_and_measure(self, input)
    }

    /// Encodes into a caller-owned slice transactionally.
    ///
    /// Every returned error leaves the complete destination unchanged. On
    /// success, bytes after the returned initialized prefix are unchanged.
    pub fn encode_into(&self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        let required = self.encoded_len(input.len())?;
        require_output(required, output.len())?;
        encode_validated(self.settings(), input, &mut output[..required]);
        Ok(required)
    }

    /// Decodes into a caller-owned slice transactionally.
    ///
    /// Validation and exact sizing complete before the first destination
    /// write. Every returned error therefore leaves the destination unchanged.
    pub fn decode_into(&self, input: &[u8], output: &mut [u8]) -> Result<usize, OneShotError> {
        let required = self.decoded_len(input)?;
        require_output(required, output.len())?;
        decode_validated(self.settings(), input, &mut output[..required]);
        Ok(required)
    }
}

const fn encoded_len_for_settings(
    settings: CodecSettings,
    input_len: usize,
) -> Result<usize, OneShotError> {
    let Some(complete) = (input_len / 3).checked_mul(4) else {
        return Err(OneShotError::LengthOverflow);
    };
    let remainder = input_len % 3;
    let tail = if remainder == 0 {
        0
    } else if matches!(settings.encode_padding(), EncodePadding::Padded) {
        4
    } else {
        remainder + 1
    };
    match complete.checked_add(tail) {
        Some(value) => Ok(value),
        None => Err(OneShotError::LengthOverflow),
    }
}

fn require_output(required: usize, available: usize) -> Result<(), OneShotError> {
    if available < required {
        Err(OneShotError::OutputTooSmall {
            required,
            available,
        })
    } else {
        Ok(())
    }
}

fn validate_and_measure<S: Codec>(codec: &Base64<S>, input: &[u8]) -> Result<usize, OneShotError> {
    let mut decoder = codec.decoder();
    let mut input_offset = 0;
    let mut measured_len = 0usize;
    let mut scratch = [0u8; 3];
    while input_offset < input.len() {
        let step = decoder
            .update(&input[input_offset..], &mut scratch)
            .map_err(map_operation_error)?;
        let progress = step.progress();
        if progress.input_consumed() == 0 && progress.output_produced() == 0 {
            return Err(OneShotError::Backend(BackendFault::ImpossibleState));
        }
        input_offset += progress.input_consumed();
        measured_len = measured_len
            .checked_add(progress.output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
    }

    loop {
        let step = decoder.finish(&mut scratch).map_err(map_operation_error)?;
        measured_len = measured_len
            .checked_add(step.progress().output_produced())
            .ok_or(OneShotError::LengthOverflow)?;
        match step.status() {
            Status::Complete => return Ok(measured_len),
            Status::OutputFull(_) => {}
            Status::NeedInput => {
                return Err(OneShotError::Backend(BackendFault::ImpossibleState));
            }
        }
    }
}

pub(super) fn map_operation_error(error: OperationError) -> OneShotError {
    match error {
        OperationError::Failed(Failure::Input(error)) => OneShotError::Input(error),
        OperationError::Failed(Failure::PositionOverflow) => OneShotError::PositionOverflow,
        OperationError::Failed(Failure::Backend(fault)) => OneShotError::Backend(fault),
        OperationError::Failed(Failure::ResourceLimit) | OperationError::Terminal(_) => {
            OneShotError::Backend(BackendFault::ImpossibleState)
        }
    }
}

fn encode_validated(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    let alphabet = settings.alphabet().as_array();
    let mut read = 0;
    let mut write = 0;
    while read + 3 <= input.len() {
        let first = input[read];
        let second = input[read + 1];
        let third = input[read + 2];
        output[write] = alphabet[usize::from(first >> 2)];
        output[write + 1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
        output[write + 2] = alphabet[usize::from(((second & 15) << 2) | (third >> 6))];
        output[write + 3] = alphabet[usize::from(third & 63)];
        read += 3;
        write += 4;
    }
    encode_tail(settings, &input[read..], &mut output[write..]);
}

fn encode_tail(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    let alphabet = settings.alphabet().as_array();
    if let [first, rest @ ..] = input {
        output[0] = alphabet[usize::from(first >> 2)];
        output[1] = alphabet[usize::from((first & 3) << 4)];
        if let [second] = rest {
            output[1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
            output[2] = alphabet[usize::from((second & 15) << 2)];
            if settings.encode_padding() == EncodePadding::Padded {
                output[3] = b'=';
            }
        } else if settings.encode_padding() == EncodePadding::Padded {
            output[2..4].copy_from_slice(b"==");
        }
    }
}

fn decode_validated(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    let mut read = 0;
    let mut write = 0;
    while read + 4 <= input.len() {
        let input_quantum = &input[read..read + 4];
        let first = decode_value(settings, input_quantum[0]);
        let second = decode_value(settings, input_quantum[1]);
        output[write] = (first << 2) | (second >> 4);
        write += 1;
        if input_quantum[2] != b'=' {
            let third = decode_value(settings, input_quantum[2]);
            output[write] = (second << 4) | (third >> 2);
            write += 1;
            if input_quantum[3] != b'=' {
                output[write] = (third << 6) | decode_value(settings, input_quantum[3]);
                write += 1;
            }
        }
        read += 4;
    }
    decode_tail(settings, &input[read..], &mut output[write..]);
}

fn decode_tail(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    if let [first, second, rest @ ..] = input {
        let first = decode_value(settings, *first);
        let second = decode_value(settings, *second);
        output[0] = (first << 2) | (second >> 4);
        if let [third] = rest
            && *third != b'='
        {
            output[1] = (second << 4) | (decode_value(settings, *third) >> 2);
        }
    }
}

fn decode_value(settings: CodecSettings, byte: u8) -> u8 {
    settings.alphabet().decode_byte(byte).unwrap_or(0)
}

#[cfg(test)]
pub(super) fn encode(
    profile: super::rfc4648_oracle::Profile,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    match profile {
        super::rfc4648_oracle::Profile::StandardPadded => {
            super::specifications::STRICT_STANDARD_PADDED.encode_into(input, output)
        }
        super::rfc4648_oracle::Profile::StandardUnpadded => {
            super::specifications::STRICT_STANDARD_UNPADDED.encode_into(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafePadded => {
            super::specifications::STRICT_URL_SAFE_PADDED.encode_into(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafeUnpadded => {
            super::specifications::STRICT_URL_SAFE_UNPADDED.encode_into(input, output)
        }
    }
}

#[cfg(test)]
pub(super) fn decode(
    profile: super::rfc4648_oracle::Profile,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, OneShotError> {
    match profile {
        super::rfc4648_oracle::Profile::StandardPadded => {
            super::specifications::STRICT_STANDARD_PADDED.decode_into(input, output)
        }
        super::rfc4648_oracle::Profile::StandardUnpadded => {
            super::specifications::STRICT_STANDARD_UNPADDED.decode_into(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafePadded => {
            super::specifications::STRICT_URL_SAFE_PADDED.decode_into(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafeUnpadded => {
            super::specifications::STRICT_URL_SAFE_UNPADDED.decode_into(input, output)
        }
    }
}
