#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

//! `base64-ng` is a `no_std`-first Base64 encoder and decoder.
//!
//! This initial release provides strict scalar RFC 4648-style behavior and
//! caller-owned output buffers. Future SIMD fast paths will be required to
//! match this scalar module byte-for-byte.
//!
//! # Examples
//!
//! Encode and decode with caller-owned buffers:
//!
//! ```
//! use base64_ng::{STANDARD, encoded_len};
//!
//! let input = b"hello";
//! let mut encoded = [0u8; encoded_len(5, true)];
//! let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"aGVsbG8=");
//!
//! let mut decoded = [0u8; 5];
//! let decoded_len = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
//! assert_eq!(&decoded[..decoded_len], input);
//! ```
//!
//! Use the URL-safe no-padding engine:
//!
//! ```
//! use base64_ng::URL_SAFE_NO_PAD;
//!
//! let mut encoded = [0u8; 3];
//! let encoded_len = URL_SAFE_NO_PAD.encode_slice(b"\xfb\xff", &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"-_8");
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

/// Standard Base64 engine with padding.
pub const STANDARD: Engine<Standard, true> = Engine::new();

/// Standard Base64 engine without padding.
pub const STANDARD_NO_PAD: Engine<Standard, false> = Engine::new();

/// URL-safe Base64 engine with padding.
pub const URL_SAFE: Engine<UrlSafe, true> = Engine::new();

/// URL-safe Base64 engine without padding.
pub const URL_SAFE_NO_PAD: Engine<UrlSafe, false> = Engine::new();

/// Returns the encoded length for an input length and padding policy.
///
/// # Panics
///
/// Panics if the encoded length would overflow `usize`. Use
/// [`checked_encoded_len`] when handling untrusted length metadata without an
/// actual input slice.
///
/// # Examples
///
/// ```
/// use base64_ng::encoded_len;
///
/// assert_eq!(encoded_len(5, true), 8);
/// assert_eq!(encoded_len(5, false), 7);
/// ```
#[must_use]
pub const fn encoded_len(input_len: usize, padded: bool) -> usize {
    match checked_encoded_len(input_len, padded) {
        Some(len) => len,
        None => panic!("encoded base64 length overflows usize"),
    }
}

/// Returns the encoded length, or `None` if it would overflow `usize`.
///
/// # Examples
///
/// ```
/// use base64_ng::checked_encoded_len;
///
/// assert_eq!(checked_encoded_len(5, true), Some(8));
/// assert_eq!(checked_encoded_len(usize::MAX, true), None);
/// ```
#[must_use]
pub const fn checked_encoded_len(input_len: usize, padded: bool) -> Option<usize> {
    let groups = input_len / 3;
    if groups > usize::MAX / 4 {
        return None;
    }
    let full = groups * 4;
    let rem = input_len % 3;
    if rem == 0 {
        Some(full)
    } else if padded {
        full.checked_add(4)
    } else {
        full.checked_add(rem + 1)
    }
}

/// Returns the maximum decoded length for an encoded input length.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_capacity;
///
/// assert_eq!(decoded_capacity(8), 6);
/// assert_eq!(decoded_capacity(7), 5);
/// ```
#[must_use]
pub const fn decoded_capacity(encoded_len: usize) -> usize {
    let rem = encoded_len % 4;
    encoded_len / 4 * 3
        + if rem == 2 {
            1
        } else if rem == 3 {
            2
        } else {
            0
        }
}

/// Returns the exact decoded length implied by input length and padding.
///
/// This validates padding placement and impossible lengths, but it does not
/// validate alphabet membership or non-canonical trailing bits.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_len;
///
/// assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
/// assert_eq!(decoded_len(b"aGVsbG8", false).unwrap(), 5);
/// ```
pub fn decoded_len(input: &[u8], padded: bool) -> Result<usize, DecodeError> {
    if padded {
        decoded_len_padded(input)
    } else {
        decoded_len_unpadded(input)
    }
}

/// A Base64 alphabet.
pub trait Alphabet {
    /// Encoding table indexed by 6-bit values.
    const ENCODE: [u8; 64];

    /// Decode one byte into a 6-bit value.
    fn decode(byte: u8) -> Option<u8>;
}

/// The RFC 4648 standard Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Standard;

impl Alphabet for Standard {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
}

/// The RFC 4648 URL-safe Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UrlSafe;

impl Alphabet for UrlSafe {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
}

/// A zero-sized Base64 engine parameterized by alphabet and padding policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Engine<A, const PAD: bool> {
    alphabet: core::marker::PhantomData<A>,
}

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Creates a new engine value.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }

    /// Returns the encoded length for this engine's padding policy.
    #[must_use]
    pub const fn encoded_len(&self, input_len: usize) -> usize {
        encoded_len(input_len, PAD)
    }

    /// Returns the encoded length for this engine, or `None` on overflow.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        checked_encoded_len(input_len, PAD)
    }

    /// Returns the exact decoded length implied by input length and padding.
    ///
    /// This validates padding placement and impossible lengths, but it does not
    /// validate alphabet membership or non-canonical trailing bits.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        decoded_len(input, PAD)
    }

    /// Encodes `input` into `output`, returning the number of bytes written.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let mut read = 0;
        let mut write = 0;
        while read + 3 <= input.len() {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = A::ENCODE[(b0 >> 2) as usize];
            output[write + 1] = A::ENCODE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize];
            output[write + 2] = A::ENCODE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize];
            output[write + 3] = A::ENCODE[(b2 & 0b0011_1111) as usize];

            read += 3;
            write += 4;
        }

        match input.len() - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = A::ENCODE[(b0 >> 2) as usize];
                output[write + 1] = A::ENCODE[((b0 & 0b0000_0011) << 4) as usize];
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                    write += 2;
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = A::ENCODE[(b0 >> 2) as usize];
                output[write + 1] = A::ENCODE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize];
                output[write + 2] = A::ENCODE[((b1 & 0b0000_1111) << 2) as usize];
                write += 3;
                if PAD {
                    output[write] = b'=';
                    write += 1;
                }
            }
            _ => unreachable!(),
        }

        Ok(write)
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0u8; 8];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        if input_len > buffer.len() {
            return Err(EncodeError::InputTooLarge {
                input_len,
                buffer_len: buffer.len(),
            });
        }

        let required = checked_encoded_len(input_len, PAD).ok_or(EncodeError::LengthOverflow)?;
        if buffer.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: buffer.len(),
            });
        }

        let mut read = input_len;
        let mut write = required;

        match input_len % 3 {
            0 => {}
            1 => {
                read -= 1;
                let b0 = buffer[read];
                if PAD {
                    write -= 4;
                    buffer[write] = A::ENCODE[(b0 >> 2) as usize];
                    buffer[write + 1] = A::ENCODE[((b0 & 0b0000_0011) << 4) as usize];
                    buffer[write + 2] = b'=';
                    buffer[write + 3] = b'=';
                } else {
                    write -= 2;
                    buffer[write] = A::ENCODE[(b0 >> 2) as usize];
                    buffer[write + 1] = A::ENCODE[((b0 & 0b0000_0011) << 4) as usize];
                }
            }
            2 => {
                read -= 2;
                let b0 = buffer[read];
                let b1 = buffer[read + 1];
                if PAD {
                    write -= 4;
                    buffer[write] = A::ENCODE[(b0 >> 2) as usize];
                    buffer[write + 1] = A::ENCODE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize];
                    buffer[write + 2] = A::ENCODE[((b1 & 0b0000_1111) << 2) as usize];
                    buffer[write + 3] = b'=';
                } else {
                    write -= 3;
                    buffer[write] = A::ENCODE[(b0 >> 2) as usize];
                    buffer[write + 1] = A::ENCODE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize];
                    buffer[write + 2] = A::ENCODE[((b1 & 0b0000_1111) << 2) as usize];
                }
            }
            _ => unreachable!(),
        }

        while read > 0 {
            read -= 3;
            write -= 4;
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let b2 = buffer[read + 2];

            buffer[write] = A::ENCODE[(b0 >> 2) as usize];
            buffer[write + 1] = A::ENCODE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize];
            buffer[write + 2] = A::ENCODE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize];
            buffer[write + 3] = A::ENCODE[(b2 & 0b0011_1111) as usize];
        }

        debug_assert_eq!(write, 0);
        Ok(&mut buffer[..required])
    }

    /// Decodes `input` into `output`, returning the number of bytes written.
    ///
    /// This is strict decoding. Whitespace, mixed alphabets, malformed padding,
    /// and trailing non-padding data are rejected.
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        if input.is_empty() {
            return Ok(0);
        }

        if PAD {
            decode_padded::<A>(input, output)
        } else {
            decode_unpadded::<A>(input, output)
        }
    }

    /// Decodes `input` into a newly allocated byte vector.
    ///
    /// This is strict decoding with the same semantics as [`Self::decode_slice`].
    #[cfg(feature = "alloc")]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let mut output = alloc::vec![0; self.decoded_len(input)?];
        let written = self.decode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Decodes the buffer in place and returns the decoded prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD_NO_PAD;
    ///
    /// let mut buffer = *b"Zm9vYmFy";
    /// let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"foobar");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        let len = Self::decode_slice_to_start(buffer)?;
        Ok(&mut buffer[..len])
    }

    fn decode_slice_to_start(buffer: &mut [u8]) -> Result<usize, DecodeError> {
        let input_len = buffer.len();
        let mut read = 0;
        let mut write = 0;
        while read + 4 <= input_len {
            let chunk = [
                buffer[read],
                buffer[read + 1],
                buffer[read + 2],
                buffer[read + 3],
            ];
            let written = decode_chunk::<A, PAD>(&chunk, &mut buffer[write..])
                .map_err(|err| err.with_index_offset(read))?;
            read += 4;
            write += written;
            if written < 3 {
                if read != input_len {
                    return Err(DecodeError::InvalidPadding { index: read - 4 });
                }
                return Ok(write);
            }
        }

        let rem = input_len - read;
        if rem == 0 {
            return Ok(write);
        }
        if PAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut tail = [0u8; 3];
        tail[..rem].copy_from_slice(&buffer[read..input_len]);
        decode_tail_unpadded::<A>(&tail[..rem], &mut buffer[write..])
            .map_err(|err| err.with_index_offset(read))
            .map(|n| write + n)
    }
}

/// Encoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The encoded output length would overflow `usize`.
    LengthOverflow,
    /// The caller-provided input length exceeds the provided buffer.
    InputTooLarge {
        /// Requested input bytes.
        input_len: usize,
        /// Available buffer bytes.
        buffer_len: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => f.write_str("base64 output length overflows usize"),
            Self::InputTooLarge {
                input_len,
                buffer_len,
            } => write!(
                f,
                "base64 input length {input_len} exceeds buffer length {buffer_len}"
            ),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Decoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The encoded input length is impossible for the selected padding policy.
    InvalidLength,
    /// A byte is not valid for the selected alphabet.
    InvalidByte {
        /// Byte index in the input.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// Padding is missing, misplaced, or non-canonical.
    InvalidPadding {
        /// Byte index where padding became invalid.
        index: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("invalid base64 input length"),
            Self::InvalidByte { index, byte } => {
                write!(f, "invalid base64 byte 0x{byte:02x} at index {index}")
            }
            Self::InvalidPadding { index } => write!(f, "invalid base64 padding at index {index}"),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 decode output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

impl DecodeError {
    fn with_index_offset(self, offset: usize) -> Self {
        match self {
            Self::InvalidByte { index, byte } => Self::InvalidByte {
                index: index + offset,
                byte,
            },
            Self::InvalidPadding { index } => Self::InvalidPadding {
                index: index + offset,
            },
            Self::InvalidLength | Self::OutputTooSmall { .. } => self,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

fn decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let written = decode_chunk::<A, true>(&input[read..read + 4], &mut output[write..])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }
    Ok(write)
}

fn decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 4 <= input.len() {
        let written = decode_chunk::<A, false>(&input[read..read + 4], &mut output[write..])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
    }
    decode_tail_unpadded::<A>(&input[read..], &mut output[write..])
        .map_err(|err| err.with_index_offset(read))
        .map(|n| write + n)
}

fn decoded_len_padded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let mut padding = 0;
    if input[input.len() - 1] == b'=' {
        padding += 1;
    }
    if input[input.len() - 2] == b'=' {
        padding += 1;
    }
    if padding == 0
        && let Some(index) = input.iter().position(|byte| *byte == b'=')
    {
        return Err(DecodeError::InvalidPadding { index });
    }
    if padding > 0 {
        let first_pad = input.len() - padding;
        if input[..first_pad].contains(&b'=') {
            return Err(DecodeError::InvalidPadding {
                index: input.iter().position(|byte| *byte == b'=').unwrap_or(0),
            });
        }
    }
    Ok(input.len() / 4 * 3 - padding)
}

fn decoded_len_unpadded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }
    if input.contains(&b'=') {
        return Err(DecodeError::InvalidPadding {
            index: input.iter().position(|byte| *byte == b'=').unwrap_or(0),
        });
    }
    Ok(decoded_capacity(input.len()))
}

fn decode_chunk<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    debug_assert_eq!(input.len(), 4);
    let v0 = decode_byte::<A>(input[0], 0)?;
    let v1 = decode_byte::<A>(input[1], 1)?;

    match (input[2], input[3]) {
        (b'=', b'=') if PAD => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: input.iter().position(|byte| *byte == b'=').unwrap_or(0),
        }),
        _ => {
            if output.len() < 3 {
                return Err(DecodeError::OutputTooSmall {
                    required: 3,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(input[2], 2)?;
            let v3 = decode_byte::<A>(input[3], 3)?;
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            output[2] = (v2 << 6) | v3;
            Ok(3)
        }
    }
}

fn decode_tail_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    match input.len() {
        0 => Ok(0),
        2 => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        3 => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

fn decode_byte<A: Alphabet>(byte: u8, index: usize) -> Result<u8, DecodeError> {
    A::decode(byte).ok_or(DecodeError::InvalidByte { index, byte })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"f"[..], &b"Zg=="[..]),
            (&b"fo"[..], &b"Zm8="[..]),
            (&b"foo"[..], &b"Zm9v"[..]),
            (&b"foob"[..], &b"Zm9vYg=="[..]),
            (&b"fooba"[..], &b"Zm9vYmE="[..]),
            (&b"foobar"[..], &b"Zm9vYmFy"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.encode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn decodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"Zg=="[..], &b"f"[..]),
            (&b"Zm8="[..], &b"fo"[..]),
            (&b"Zm9v"[..], &b"foo"[..]),
            (&b"Zm9vYg=="[..], &b"foob"[..]),
            (&b"Zm9vYmE="[..], &b"fooba"[..]),
            (&b"Zm9vYmFy"[..], &b"foobar"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.decode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn supports_unpadded_url_safe() {
        let mut encoded = [0u8; 16];
        let written = URL_SAFE_NO_PAD
            .encode_slice(b"\xfb\xff", &mut encoded)
            .unwrap();
        assert_eq!(&encoded[..written], b"-_8");

        let mut decoded = [0u8; 2];
        let written = URL_SAFE_NO_PAD
            .decode_slice(&encoded[..written], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..written], b"\xfb\xff");
    }

    #[test]
    fn decodes_in_place() {
        let mut buffer = *b"Zm9vYmFy";
        let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
        assert_eq!(decoded, b"foobar");
    }

    #[test]
    fn rejects_non_canonical_padding_bits() {
        let mut output = [0u8; 4];
        assert_eq!(
            STANDARD.decode_slice(b"Zh==", &mut output),
            Err(DecodeError::InvalidPadding { index: 1 })
        );
        assert_eq!(
            STANDARD.decode_slice(b"Zm9=", &mut output),
            Err(DecodeError::InvalidPadding { index: 2 })
        );
    }
}
