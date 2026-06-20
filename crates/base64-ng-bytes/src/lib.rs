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

    /// Decodes all remaining bytes from `input` into a [`Bytes`] value.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] if decoding fails.
    fn decode_buf<B>(&self, input: B) -> Result<Bytes, DecodeError>
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

    fn decode_buf<B>(&self, input: B) -> Result<Bytes, DecodeError>
    where
        B: Buf,
    {
        self.decode_bytes(collect_buf(input))
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
