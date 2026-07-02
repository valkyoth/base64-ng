#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional `bytes` integration for `base64-ng`.

extern crate alloc;

use alloc::vec::Vec;
use base64_ng::{Alphabet, DecodeError, EncodeError, Engine};
use bytes::{Buf, BufMut, Bytes};

/// Encoding error for bounded `bytes` integration helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BytesEncodeError {
    /// The input buffer reports more remaining bytes than the caller permits.
    InputTooLarge {
        /// Remaining input bytes reported by [`Buf::remaining`].
        input_len: usize,
        /// Caller-provided maximum input bytes.
        max_input_len: usize,
    },
    /// Base64 encoding failed after the input-size check passed.
    Encode(EncodeError),
}

impl core::fmt::Display for BytesEncodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputTooLarge {
                input_len,
                max_input_len,
            } => write!(
                formatter,
                "bytes input length {input_len} exceeds limit {max_input_len}"
            ),
            Self::Encode(error) => core::fmt::Display::fmt(error, formatter),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InputTooLarge { .. } => None,
            Self::Encode(error) => Some(error),
        }
    }
}

impl From<EncodeError> for BytesEncodeError {
    fn from(error: EncodeError) -> Self {
        Self::Encode(error)
    }
}

/// Decoding error for bounded `bytes` integration helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BytesDecodeError {
    /// The input buffer reports more remaining bytes than the caller permits.
    InputTooLarge {
        /// Remaining input bytes reported by [`Buf::remaining`].
        input_len: usize,
        /// Caller-provided maximum input bytes.
        max_input_len: usize,
    },
    /// Base64 decoding failed after the input-size check passed.
    Decode(DecodeError),
}

impl core::fmt::Display for BytesDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputTooLarge {
                input_len,
                max_input_len,
            } => write!(
                formatter,
                "bytes input length {input_len} exceeds limit {max_input_len}"
            ),
            Self::Decode(error) => core::fmt::Display::fmt(error, formatter),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BytesDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InputTooLarge { .. } => None,
            Self::Decode(error) => Some(error),
        }
    }
}

impl From<DecodeError> for BytesDecodeError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

/// Extension helpers for [`base64_ng::Engine`] and `bytes` buffers.
pub trait EngineBytesExt<A, const PAD: bool>
where
    A: Alphabet,
{
    /// Encodes bytes into a [`Bytes`] value.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError`] if the encoded length overflows.
    fn encode_bytes(&self, input: impl AsRef<[u8]>) -> Result<Bytes, EncodeError>;

    /// Decodes Base64 bytes into a [`Bytes`] value.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if decoding fails.
    fn decode_bytes(&self, input: impl AsRef<[u8]>) -> Result<Bytes, DecodeError>;

    /// Encodes all remaining bytes from `input` into a [`Bytes`] value.
    ///
    /// The input buffer is advanced to completion only after its current
    /// chunks have been copied into a temporary contiguous buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError`] if the encoded length overflows.
    fn encode_buf<B>(&self, input: B) -> Result<Bytes, EncodeError>
    where
        B: Buf;

    /// Encodes at most `max_input_len` remaining bytes from `input` into a
    /// [`Bytes`] value.
    ///
    /// Use this variant for peer-controlled or metadata-declared frame sizes.
    /// The input buffer is not collected if [`Buf::remaining`] exceeds
    /// `max_input_len`.
    ///
    /// # Errors
    ///
    /// Returns [`BytesEncodeError::InputTooLarge`] if `input.remaining()`
    /// exceeds `max_input_len`, or [`BytesEncodeError::Encode`] if Base64
    /// encoding fails.
    fn encode_buf_limited<B>(
        &self,
        input: B,
        max_input_len: usize,
    ) -> Result<Bytes, BytesEncodeError>
    where
        B: Buf;

    /// Decodes all remaining bytes from `input` into a [`Bytes`] value.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if decoding fails.
    fn decode_buf<B>(&self, input: B) -> Result<Bytes, DecodeError>
    where
        B: Buf;

    /// Decodes at most `max_input_len` remaining Base64 bytes from `input`
    /// into a [`Bytes`] value.
    ///
    /// Use this variant for peer-controlled or metadata-declared frame sizes.
    /// The input buffer is not collected if [`Buf::remaining`] exceeds
    /// `max_input_len`.
    ///
    /// # Errors
    ///
    /// Returns [`BytesDecodeError::InputTooLarge`] if `input.remaining()`
    /// exceeds `max_input_len`, or [`BytesDecodeError::Decode`] if Base64
    /// decoding fails.
    fn decode_buf_limited<B>(
        &self,
        input: B,
        max_input_len: usize,
    ) -> Result<Bytes, BytesDecodeError>
    where
        B: Buf;

    /// Encodes all remaining bytes from `input` into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::OutputTooSmall`] if `output` does not have enough
    /// remaining mutable capacity.
    fn encode_buf_to_mut<B, M>(&self, input: B, output: &mut M) -> Result<usize, EncodeError>
    where
        B: Buf,
        M: BufMut;

    /// Encodes at most `max_input_len` remaining bytes from `input` into
    /// `output`.
    ///
    /// # Errors
    ///
    /// Returns [`BytesEncodeError::InputTooLarge`] if `input.remaining()`
    /// exceeds `max_input_len`, [`BytesEncodeError::Encode`] with
    /// [`EncodeError::OutputTooSmall`] if `output` is too small, or another
    /// wrapped [`EncodeError`] if Base64 encoding fails.
    fn encode_buf_to_mut_limited<B, M>(
        &self,
        input: B,
        output: &mut M,
        max_input_len: usize,
    ) -> Result<usize, BytesEncodeError>
    where
        B: Buf,
        M: BufMut;

    /// Decodes all remaining Base64 bytes from `input` into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::OutputTooSmall`] if `output` does not have enough
    /// remaining mutable capacity, or another [`DecodeError`] if decoding
    /// fails.
    fn decode_buf_to_mut<B, M>(&self, input: B, output: &mut M) -> Result<usize, DecodeError>
    where
        B: Buf,
        M: BufMut;

    /// Decodes at most `max_input_len` remaining Base64 bytes from `input`
    /// into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`BytesDecodeError::InputTooLarge`] if `input.remaining()`
    /// exceeds `max_input_len`, [`BytesDecodeError::Decode`] with
    /// [`DecodeError::OutputTooSmall`] if `output` is too small, or another
    /// wrapped [`DecodeError`] if Base64 decoding fails.
    fn decode_buf_to_mut_limited<B, M>(
        &self,
        input: B,
        output: &mut M,
        max_input_len: usize,
    ) -> Result<usize, BytesDecodeError>
    where
        B: Buf,
        M: BufMut;
}

impl<A, const PAD: bool> EngineBytesExt<A, PAD> for Engine<A, PAD>
where
    A: Alphabet,
{
    fn encode_bytes(&self, input: impl AsRef<[u8]>) -> Result<Bytes, EncodeError> {
        self.encode_vec(input.as_ref()).map(Bytes::from)
    }

    fn decode_bytes(&self, input: impl AsRef<[u8]>) -> Result<Bytes, DecodeError> {
        self.decode_vec(input.as_ref()).map(Bytes::from)
    }

    fn encode_buf<B>(&self, input: B) -> Result<Bytes, EncodeError>
    where
        B: Buf,
    {
        self.encode_bytes(collect_buf(input))
    }

    fn encode_buf_limited<B>(
        &self,
        input: B,
        max_input_len: usize,
    ) -> Result<Bytes, BytesEncodeError>
    where
        B: Buf,
    {
        let input =
            collect_buf_limited(input, max_input_len).map_err(|(input_len, max_input_len)| {
                BytesEncodeError::InputTooLarge {
                    input_len,
                    max_input_len,
                }
            })?;
        self.encode_bytes(input).map_err(BytesEncodeError::Encode)
    }

    fn decode_buf<B>(&self, input: B) -> Result<Bytes, DecodeError>
    where
        B: Buf,
    {
        self.decode_bytes(collect_buf(input))
    }

    fn decode_buf_limited<B>(
        &self,
        input: B,
        max_input_len: usize,
    ) -> Result<Bytes, BytesDecodeError>
    where
        B: Buf,
    {
        let input =
            collect_buf_limited(input, max_input_len).map_err(|(input_len, max_input_len)| {
                BytesDecodeError::InputTooLarge {
                    input_len,
                    max_input_len,
                }
            })?;
        self.decode_bytes(input).map_err(BytesDecodeError::Decode)
    }

    fn encode_buf_to_mut<B, M>(&self, input: B, output: &mut M) -> Result<usize, EncodeError>
    where
        B: Buf,
        M: BufMut,
    {
        let encoded = self.encode_buf(input)?;
        let len = encoded.len();
        if output.remaining_mut() < len {
            return Err(EncodeError::OutputTooSmall {
                required: len,
                available: output.remaining_mut(),
            });
        }
        output.put_slice(&encoded);
        Ok(len)
    }

    fn encode_buf_to_mut_limited<B, M>(
        &self,
        input: B,
        output: &mut M,
        max_input_len: usize,
    ) -> Result<usize, BytesEncodeError>
    where
        B: Buf,
        M: BufMut,
    {
        let encoded = self.encode_buf_limited(input, max_input_len)?;
        let len = encoded.len();
        if output.remaining_mut() < len {
            return Err(BytesEncodeError::Encode(EncodeError::OutputTooSmall {
                required: len,
                available: output.remaining_mut(),
            }));
        }
        output.put_slice(&encoded);
        Ok(len)
    }

    fn decode_buf_to_mut<B, M>(&self, input: B, output: &mut M) -> Result<usize, DecodeError>
    where
        B: Buf,
        M: BufMut,
    {
        let decoded = self.decode_buf(input)?;
        let len = decoded.len();
        if output.remaining_mut() < len {
            return Err(DecodeError::OutputTooSmall {
                required: len,
                available: output.remaining_mut(),
            });
        }
        output.put_slice(&decoded);
        Ok(len)
    }

    fn decode_buf_to_mut_limited<B, M>(
        &self,
        input: B,
        output: &mut M,
        max_input_len: usize,
    ) -> Result<usize, BytesDecodeError>
    where
        B: Buf,
        M: BufMut,
    {
        let decoded = self.decode_buf_limited(input, max_input_len)?;
        let len = decoded.len();
        if output.remaining_mut() < len {
            return Err(BytesDecodeError::Decode(DecodeError::OutputTooSmall {
                required: len,
                available: output.remaining_mut(),
            }));
        }
        output.put_slice(&decoded);
        Ok(len)
    }
}

fn collect_buf<B>(mut input: B) -> Vec<u8>
where
    B: Buf,
{
    let mut output = Vec::with_capacity(input.remaining());
    while input.has_remaining() {
        let chunk = input.chunk();
        output.extend_from_slice(chunk);
        let len = chunk.len();
        input.advance(len);
    }
    output
}

fn collect_buf_limited<B>(input: B, max_input_len: usize) -> Result<Vec<u8>, (usize, usize)>
where
    B: Buf,
{
    let input_len = input.remaining();
    if input_len > max_input_len {
        return Err((input_len, max_input_len));
    }

    Ok(collect_buf(input))
}
