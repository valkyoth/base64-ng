//! Const-capable transforms over validated codec settings.

use super::{
    contracts::InputError,
    specifications::{
        Base64, CodecSettings, DecodePadding, EncodePadding, RuntimeSpec, StrictStandardPadded,
        StrictStandardUnpadded, StrictUrlSafePadded, StrictUrlSafeUnpadded, TrailingBits,
    },
};

/// Error returned by an exact const transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstTransformError {
    /// The exact output length cannot be represented by `usize`.
    LengthOverflow,
    /// The encoded input is malformed under the selected codec settings.
    Input(InputError),
    /// The output array length is not the exact result length.
    OutputLengthMismatch {
        /// Exact required output bytes.
        required: usize,
        /// Compile-time output array length supplied by the caller.
        actual: usize,
    },
}

impl core::fmt::Display for ConstTransformError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => formatter.write_str("base64 output length overflows usize"),
            Self::Input(error) => error.fmt(formatter),
            Self::OutputLengthMismatch { required, actual } => write!(
                formatter,
                "base64 exact output length mismatch: required {required}, actual {actual}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConstTransformError {}

impl CodecSettings {
    /// Returns the exact encoded length in const and runtime contexts.
    pub const fn encoded_len(self, input_len: usize) -> Result<usize, ConstTransformError> {
        let Some(complete) = (input_len / 3).checked_mul(4) else {
            return Err(ConstTransformError::LengthOverflow);
        };
        let remainder = input_len % 3;
        let tail = match (remainder, self.encode_padding()) {
            (0, _) => 0,
            (_, EncodePadding::Padded) => 4,
            (value, EncodePadding::Unpadded) => value + 1,
        };
        match complete.checked_add(tail) {
            Some(value) => Ok(value),
            None => Err(ConstTransformError::LengthOverflow),
        }
    }

    /// Validates encoded input and returns its exact decoded length.
    pub const fn decoded_len(self, input: &[u8]) -> Result<usize, ConstTransformError> {
        validate_and_measure(self, input)
    }

    /// Encodes into an array whose length must equal the exact result length.
    pub const fn encode_array<const INPUT: usize, const OUTPUT: usize>(
        self,
        input: &[u8; INPUT],
    ) -> Result<[u8; OUTPUT], ConstTransformError> {
        let required = match self.encoded_len(INPUT) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if required != OUTPUT {
            return Err(ConstTransformError::OutputLengthMismatch {
                required,
                actual: OUTPUT,
            });
        }

        let mut output = [0u8; OUTPUT];
        encode_exact(self, input, &mut output);
        Ok(output)
    }

    /// Decodes into an array whose length must equal the exact result length.
    pub const fn decode_array<const INPUT: usize, const OUTPUT: usize>(
        self,
        input: &[u8; INPUT],
    ) -> Result<[u8; OUTPUT], ConstTransformError> {
        let required = match self.decoded_len(input) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if required != OUTPUT {
            return Err(ConstTransformError::OutputLengthMismatch {
                required,
                actual: OUTPUT,
            });
        }

        let mut output = [0u8; OUTPUT];
        decode_exact(self, input, &mut output);
        Ok(output)
    }
}

macro_rules! const_codec_methods {
    ($specification:ty, $settings:expr) => {
        impl Base64<$specification> {
            /// Returns const-capable immutable settings for this codec.
            #[must_use]
            pub const fn const_settings(&self) -> CodecSettings {
                $settings
            }

            /// Encodes into an array whose length must equal the exact result.
            pub const fn encode_array<const INPUT: usize, const OUTPUT: usize>(
                &self,
                input: &[u8; INPUT],
            ) -> Result<[u8; OUTPUT], ConstTransformError> {
                self.const_settings().encode_array(input)
            }

            /// Decodes into an array whose length must equal the exact result.
            pub const fn decode_array<const INPUT: usize, const OUTPUT: usize>(
                &self,
                input: &[u8; INPUT],
            ) -> Result<[u8; OUTPUT], ConstTransformError> {
                self.const_settings().decode_array(input)
            }
        }
    };
}

const_codec_methods!(StrictStandardPadded, StrictStandardPadded::const_settings());
const_codec_methods!(
    StrictStandardUnpadded,
    StrictStandardUnpadded::const_settings()
);
const_codec_methods!(StrictUrlSafePadded, StrictUrlSafePadded::const_settings());
const_codec_methods!(
    StrictUrlSafeUnpadded,
    StrictUrlSafeUnpadded::const_settings()
);

impl Base64<RuntimeSpec> {
    /// Returns const-capable immutable settings for this runtime codec.
    #[must_use]
    pub const fn const_settings(&self) -> CodecSettings {
        self.specification().const_settings()
    }

    /// Encodes into an array whose length must equal the exact result.
    pub const fn encode_array<const INPUT: usize, const OUTPUT: usize>(
        &self,
        input: &[u8; INPUT],
    ) -> Result<[u8; OUTPUT], ConstTransformError> {
        self.const_settings().encode_array(input)
    }

    /// Decodes into an array whose length must equal the exact result.
    pub const fn decode_array<const INPUT: usize, const OUTPUT: usize>(
        &self,
        input: &[u8; INPUT],
    ) -> Result<[u8; OUTPUT], ConstTransformError> {
        self.const_settings().decode_array(input)
    }
}

#[allow(clippy::manual_is_multiple_of)]
const fn validate_and_measure(
    settings: CodecSettings,
    input: &[u8],
) -> Result<usize, ConstTransformError> {
    if matches!(settings.decode_padding(), DecodePadding::RequireCanonical) && input.len() % 4 != 0
    {
        return Err(ConstTransformError::Input(InputError::TruncatedInput {
            index: input.len(),
        }));
    }

    let mut read = 0;
    let mut written = 0usize;
    while input.len() - read >= 4 {
        let (produced, terminal) = match validate_quantum(settings, input, read) {
            Ok(value) => value,
            Err(error) => return Err(ConstTransformError::Input(error)),
        };
        written = match written.checked_add(produced) {
            Some(value) => value,
            None => return Err(ConstTransformError::LengthOverflow),
        };
        read += 4;
        if terminal && read != input.len() {
            return Err(ConstTransformError::Input(InputError::TrailingData {
                index: read,
            }));
        }
    }

    if read == input.len() {
        return Ok(written);
    }
    if matches!(settings.decode_padding(), DecodePadding::RequireCanonical) {
        return Err(ConstTransformError::Input(InputError::TruncatedInput {
            index: input.len(),
        }));
    }

    match input.len() - read {
        2 => {
            if let Err(error) = decode_symbol(settings, input[read], read) {
                return Err(ConstTransformError::Input(error));
            }
            let second = match decode_symbol(settings, input[read + 1], read + 1) {
                Ok(value) => value,
                Err(error) => return Err(ConstTransformError::Input(error)),
            };
            if second & 0x0f != 0
                && matches!(settings.trailing_bits(), TrailingBits::RequireCanonical)
            {
                return Err(ConstTransformError::Input(
                    InputError::NonCanonicalTrailingBits { index: read + 1 },
                ));
            }
            add_decoded_len(written, 1)
        }
        3 => validate_three_symbol_tail(settings, input, read, written),
        _ => Err(ConstTransformError::Input(InputError::InvalidLength)),
    }
}

const fn validate_three_symbol_tail(
    settings: CodecSettings,
    input: &[u8],
    read: usize,
    written: usize,
) -> Result<usize, ConstTransformError> {
    let first = match decode_symbol(settings, input[read], read) {
        Ok(value) => value,
        Err(error) => return Err(ConstTransformError::Input(error)),
    };
    let second = match decode_symbol(settings, input[read + 1], read + 1) {
        Ok(value) => value,
        Err(error) => return Err(ConstTransformError::Input(error)),
    };
    let _ = first;
    if input[read + 2] == b'=' {
        if matches!(settings.decode_padding(), DecodePadding::Forbid) {
            return Err(ConstTransformError::Input(InputError::InvalidPadding {
                index: read + 2,
            }));
        }
        if second & 0x0f != 0 && matches!(settings.trailing_bits(), TrailingBits::RequireCanonical)
        {
            return Err(ConstTransformError::Input(
                InputError::NonCanonicalTrailingBits { index: read + 1 },
            ));
        }
        return add_decoded_len(written, 1);
    }

    let third = match decode_symbol(settings, input[read + 2], read + 2) {
        Ok(value) => value,
        Err(error) => return Err(ConstTransformError::Input(error)),
    };
    if third & 0x03 != 0 && matches!(settings.trailing_bits(), TrailingBits::RequireCanonical) {
        return Err(ConstTransformError::Input(
            InputError::NonCanonicalTrailingBits { index: read + 2 },
        ));
    }
    add_decoded_len(written, 2)
}

const fn validate_quantum(
    settings: CodecSettings,
    input: &[u8],
    read: usize,
) -> Result<(usize, bool), InputError> {
    let first = match decode_symbol(settings, input[read], read) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    let second = match decode_symbol(settings, input[read + 1], read + 1) {
        Ok(value) => value,
        Err(error) => return Err(error),
    };
    let _ = first;
    match (input[read + 2], input[read + 3]) {
        (b'=', b'=') => {
            if matches!(settings.decode_padding(), DecodePadding::Forbid) {
                return Err(InputError::InvalidPadding { index: read + 2 });
            }
            if second & 0x0f != 0
                && matches!(settings.trailing_bits(), TrailingBits::RequireCanonical)
            {
                return Err(InputError::NonCanonicalTrailingBits { index: read + 1 });
            }
            Ok((1, true))
        }
        (b'=', _) => Err(InputError::InvalidPadding { index: read + 2 }),
        (third, b'=') => {
            if matches!(settings.decode_padding(), DecodePadding::Forbid) {
                return Err(InputError::InvalidPadding { index: read + 3 });
            }
            let third = match decode_symbol(settings, third, read + 2) {
                Ok(value) => value,
                Err(error) => return Err(error),
            };
            if third & 0x03 != 0
                && matches!(settings.trailing_bits(), TrailingBits::RequireCanonical)
            {
                return Err(InputError::NonCanonicalTrailingBits { index: read + 2 });
            }
            Ok((2, true))
        }
        (third, fourth) => {
            if let Err(error) = decode_symbol(settings, third, read + 2) {
                return Err(error);
            }
            if let Err(error) = decode_symbol(settings, fourth, read + 3) {
                return Err(error);
            }
            Ok((3, false))
        }
    }
}

const fn decode_symbol(settings: CodecSettings, byte: u8, index: usize) -> Result<u8, InputError> {
    match settings.alphabet().decode_byte(byte) {
        Some(value) => Ok(value),
        None => Err(InputError::InvalidByte { index, byte }),
    }
}

const fn add_decoded_len(written: usize, additional: usize) -> Result<usize, ConstTransformError> {
    match written.checked_add(additional) {
        Some(value) => Ok(value),
        None => Err(ConstTransformError::LengthOverflow),
    }
}

#[allow(clippy::cast_lossless)]
const fn encode_exact(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    let alphabet = settings.alphabet().as_array();
    let mut read = 0;
    let mut write = 0;
    while input.len() - read >= 3 {
        output[write] = alphabet[(input[read] >> 2) as usize];
        output[write + 1] = alphabet[(((input[read] & 3) << 4) | (input[read + 1] >> 4)) as usize];
        output[write + 2] =
            alphabet[(((input[read + 1] & 15) << 2) | (input[read + 2] >> 6)) as usize];
        output[write + 3] = alphabet[(input[read + 2] & 63) as usize];
        read += 3;
        write += 4;
    }

    let remaining = input.len() - read;
    if remaining != 0 {
        output[write] = alphabet[(input[read] >> 2) as usize];
        output[write + 1] = alphabet[((input[read] & 3) << 4) as usize];
        if remaining == 2 {
            output[write + 1] =
                alphabet[(((input[read] & 3) << 4) | (input[read + 1] >> 4)) as usize];
            output[write + 2] = alphabet[((input[read + 1] & 15) << 2) as usize];
            if matches!(settings.encode_padding(), EncodePadding::Padded) {
                output[write + 3] = b'=';
            }
        } else if matches!(settings.encode_padding(), EncodePadding::Padded) {
            output[write + 2] = b'=';
            output[write + 3] = b'=';
        }
    }
}

const fn decode_exact(settings: CodecSettings, input: &[u8], output: &mut [u8]) {
    let mut read = 0;
    let mut write = 0;
    while input.len() - read >= 4 {
        let first = decode_value(settings, input[read]);
        let second = decode_value(settings, input[read + 1]);
        output[write] = (first << 2) | (second >> 4);
        write += 1;
        if input[read + 2] != b'=' {
            let third = decode_value(settings, input[read + 2]);
            output[write] = (second << 4) | (third >> 2);
            write += 1;
            if input[read + 3] != b'=' {
                output[write] = (third << 6) | decode_value(settings, input[read + 3]);
                write += 1;
            }
        }
        read += 4;
    }

    if read + 2 <= input.len() {
        let first = decode_value(settings, input[read]);
        let second = decode_value(settings, input[read + 1]);
        output[write] = (first << 2) | (second >> 4);
        if read + 3 == input.len() && input[read + 2] != b'=' {
            output[write + 1] = (second << 4) | (decode_value(settings, input[read + 2]) >> 2);
        }
    }
}

const fn decode_value(settings: CodecSettings, byte: u8) -> u8 {
    match settings.alphabet().decode_byte(byte) {
        Some(value) => value,
        None => 0,
    }
}
