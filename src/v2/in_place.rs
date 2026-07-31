//! Finite-buffer in-place transforms with explicit mutation contracts.

use super::{
    contracts::{BackendFault, InputError},
    ordinary::OneShotError,
    specifications::{Base64, Codec, CodecSettings, EncodePadding},
};

/// Error returned by a finite-buffer in-place operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InPlaceError {
    /// The caller-declared input prefix exceeds the complete buffer.
    InputLengthExceedsBuffer {
        /// Declared encoded or plaintext input bytes.
        input_len: usize,
        /// Complete caller buffer bytes.
        buffer_len: usize,
    },
    /// The encoded or staging capacity calculation overflowed `usize`.
    LengthOverflow,
    /// Reverse encoding needs more bytes than the complete caller buffer.
    OutputTooSmall {
        /// Exact encoded output bytes.
        required: usize,
        /// Complete caller buffer bytes.
        available: usize,
    },
    /// Secret decode staging cannot hold the fixed-work candidate output.
    StagingTooSmall {
        /// Required private staging bytes.
        required: usize,
        /// Available private staging bytes.
        available: usize,
    },
    /// Caller-visible and private staging byte ranges overlap.
    OverlappingBuffers,
    /// A byte range end address cannot be represented by `usize`.
    AddressRangeOverflow,
    /// The selected compatibility policy is not eligible for secret decode.
    SecretPolicyUnsupported,
    /// Strict ordinary validation rejected the encoded input.
    Input(InputError),
    /// Absolute ordinary source positions cannot be represented by `usize`.
    PositionOverflow,
    /// Fixed-work secret validation rejected the encoded input opaquely.
    InvalidSecretInput,
    /// An internal backend integrity invariant failed.
    Backend(BackendFault),
}

impl core::fmt::Display for InPlaceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputLengthExceedsBuffer {
                input_len,
                buffer_len,
            } => write!(
                formatter,
                "in-place input length {input_len} exceeds buffer length {buffer_len}"
            ),
            Self::LengthOverflow => formatter.write_str("in-place output length overflows usize"),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                formatter,
                "in-place output requires {required} bytes; buffer has {available}"
            ),
            Self::StagingTooSmall {
                required,
                available,
            } => write!(
                formatter,
                "secret decode staging requires {required} bytes; staging has {available}"
            ),
            Self::OverlappingBuffers => {
                formatter.write_str("caller buffer and private staging overlap")
            }
            Self::AddressRangeOverflow => {
                formatter.write_str("caller byte range address overflows usize")
            }
            Self::SecretPolicyUnsupported => {
                formatter.write_str("codec policy is not eligible for secret decoding")
            }
            Self::Input(error) => error.fmt(formatter),
            Self::PositionOverflow => formatter.write_str("base64 source position overflows usize"),
            Self::InvalidSecretInput => formatter.write_str("invalid secret base64 input"),
            Self::Backend(fault) => write!(formatter, "base64 backend fault: {}", fault.as_str()),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InPlaceError {}

impl<S: Codec> Base64<S> {
    /// Encodes the first `input_len` bytes into the same buffer in reverse.
    ///
    /// Validation and exact capacity checks finish before mutation. Every
    /// returned error leaves the complete buffer unchanged. On success, the
    /// returned prefix contains the complete encoded value.
    pub fn encode_in_place(
        &self,
        buffer: &mut [u8],
        input_len: usize,
    ) -> Result<usize, InPlaceError> {
        require_input_prefix(input_len, buffer.len())?;
        let required = self.encoded_len(input_len).map_err(map_one_shot_error)?;
        if required > buffer.len() {
            return Err(InPlaceError::OutputTooSmall {
                required,
                available: buffer.len(),
            });
        }
        encode_reverse(self.settings(), buffer, input_len, required);
        Ok(required)
    }

    /// Decodes the first `input_len` bytes by compacting forward in place.
    ///
    /// Strict validation and exact sizing finish before mutation. Every
    /// returned error leaves the complete buffer unchanged. On success, the
    /// returned prefix contains the decoded value; bytes after it are residual
    /// ordinary storage and have no cleanup guarantee.
    pub fn decode_in_place(
        &self,
        buffer: &mut [u8],
        input_len: usize,
    ) -> Result<usize, InPlaceError> {
        require_input_prefix(input_len, buffer.len())?;
        let required = self
            .decoded_len(&buffer[..input_len])
            .map_err(map_one_shot_error)?;
        decode_forward(self.settings(), buffer, input_len);
        Ok(required)
    }
}

pub(super) fn require_input_prefix(
    input_len: usize,
    buffer_len: usize,
) -> Result<(), InPlaceError> {
    if input_len > buffer_len {
        Err(InPlaceError::InputLengthExceedsBuffer {
            input_len,
            buffer_len,
        })
    } else {
        Ok(())
    }
}

pub(super) fn require_disjoint_slices(left: &[u8], right: &[u8]) -> Result<(), InPlaceError> {
    require_disjoint_ranges(
        left.as_ptr() as usize,
        left.len(),
        right.as_ptr() as usize,
        right.len(),
    )
}

fn require_disjoint_ranges(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), InPlaceError> {
    let left_end = left_start
        .checked_add(left_len)
        .ok_or(InPlaceError::AddressRangeOverflow)?;
    let right_end = right_start
        .checked_add(right_len)
        .ok_or(InPlaceError::AddressRangeOverflow)?;
    if left_len != 0 && right_len != 0 && left_start < right_end && right_start < left_end {
        Err(InPlaceError::OverlappingBuffers)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn require_disjoint_ranges_for_test(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), InPlaceError> {
    require_disjoint_ranges(left_start, left_len, right_start, right_len)
}

fn map_one_shot_error(error: OneShotError) -> InPlaceError {
    match error {
        OneShotError::LengthOverflow => InPlaceError::LengthOverflow,
        OneShotError::Input(error) => InPlaceError::Input(error),
        OneShotError::PositionOverflow => InPlaceError::PositionOverflow,
        OneShotError::OutputTooSmall {
            required,
            available,
        } => InPlaceError::OutputTooSmall {
            required,
            available,
        },
        OneShotError::Backend(fault) => InPlaceError::Backend(fault),
        OneShotError::AllocationLimitExceeded { .. } | OneShotError::AllocationFailed { .. } => {
            InPlaceError::Backend(BackendFault::ImpossibleState)
        }
    }
}

fn encode_reverse(settings: CodecSettings, buffer: &mut [u8], input_len: usize, output_len: usize) {
    let alphabet = settings.alphabet().as_array();
    let mut read = input_len;
    let mut write = output_len;
    let tail = input_len % 3;

    if tail != 0 {
        read -= tail;
        let first = buffer[read];
        let second = if tail == 2 { buffer[read + 1] } else { 0 };
        let tail_len = if settings.encode_padding() == EncodePadding::Padded {
            4
        } else {
            tail + 1
        };
        write -= tail_len;
        buffer[write] = alphabet[usize::from(first >> 2)];
        buffer[write + 1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
        if tail == 2 {
            buffer[write + 2] = alphabet[usize::from((second & 15) << 2)];
            if tail_len == 4 {
                buffer[write + 3] = b'=';
            }
        } else if tail_len == 4 {
            buffer[write + 2] = b'=';
            buffer[write + 3] = b'=';
        }
    }

    while read != 0 {
        read -= 3;
        write -= 4;
        let first = buffer[read];
        let second = buffer[read + 1];
        let third = buffer[read + 2];
        buffer[write] = alphabet[usize::from(first >> 2)];
        buffer[write + 1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
        buffer[write + 2] = alphabet[usize::from(((second & 15) << 2) | (third >> 6))];
        buffer[write + 3] = alphabet[usize::from(third & 63)];
    }
}

fn decode_forward(settings: CodecSettings, buffer: &mut [u8], input_len: usize) {
    let mut read = 0;
    let mut write = 0;
    while input_len - read >= 4 {
        let first = decode_value(settings, buffer[read]);
        let second = decode_value(settings, buffer[read + 1]);
        let third_byte = buffer[read + 2];
        let fourth_byte = buffer[read + 3];
        buffer[write] = (first << 2) | (second >> 4);
        write += 1;
        if third_byte != b'=' {
            let third = decode_value(settings, third_byte);
            buffer[write] = (second << 4) | (third >> 2);
            write += 1;
            if fourth_byte != b'=' {
                buffer[write] = (third << 6) | decode_value(settings, fourth_byte);
                write += 1;
            }
        }
        read += 4;
    }

    if input_len - read >= 2 {
        let first = decode_value(settings, buffer[read]);
        let second = decode_value(settings, buffer[read + 1]);
        buffer[write] = (first << 2) | (second >> 4);
        if input_len - read == 3 {
            buffer[write + 1] = (second << 4) | (decode_value(settings, buffer[read + 2]) >> 2);
        }
    }
}

fn decode_value(settings: CodecSettings, byte: u8) -> u8 {
    settings.alphabet().decode_byte(byte).unwrap_or(0)
}
