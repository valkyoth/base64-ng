#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Fragment-preserving `bytes` integration for the `base64-ng` 2.0 codec.
//!
//! Transactional helpers return crate-owned [`Bytes`] values. Stateful
//! [`BytesEncoder`] and [`BytesDecoder`] adapters write directly to arbitrary
//! [`bytes::BufMut`] destinations and report exactly committed prefixes. No API in
//! this crate coalesces the complete input into a temporary buffer.

extern crate alloc;

mod driver;
mod error;
mod owned;

use base64_ng::{Base64, Codec};
use bytes::{Buf, Bytes};

pub use driver::{BytesDecoder, BytesEncoder};
pub use error::{BytesError, BytesErrorKind, BytesLimits, BytesProgress, BytesStep};

/// Extension methods for a sealed `base64-ng` 2.0 codec.
pub trait Base64BytesExt<S>
where
    S: Codec,
{
    /// Constructs an unbounded prefix-committing encoder state.
    fn bytes_encoder(&self) -> BytesEncoder;

    /// Constructs a prefix-committing encoder with cumulative limits.
    fn bytes_encoder_with_limits(&self, limits: BytesLimits) -> BytesEncoder;

    /// Constructs an unbounded prefix-committing strict decoder state.
    fn bytes_decoder(&self) -> BytesDecoder;

    /// Constructs a prefix-committing strict decoder with cumulative limits.
    fn bytes_decoder_with_limits(&self, limits: BytesLimits) -> BytesDecoder;

    /// Encodes fragmented input transactionally into crate-owned storage.
    ///
    /// The input is traversed through [`Buf::chunk`] and [`Buf::advance`]. An
    /// error discards the private output allocation before returning.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] for length overflow, allocation failure, an
    /// invalid `Buf` contract, or an underlying transform failure.
    fn encode_buf<B>(&self, input: B) -> Result<Bytes, BytesError>
    where
        B: Buf;

    /// Encodes fragmented input transactionally under explicit limits.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] if either limit is exceeded or encoding cannot
    /// complete.
    fn encode_buf_with_limits<B>(&self, input: B, limits: BytesLimits) -> Result<Bytes, BytesError>
    where
        B: Buf;

    /// Strictly decodes fragmented input transactionally into crate-owned
    /// storage.
    ///
    /// Malformed input may populate the private destination before rejection,
    /// but no bytes are returned to the caller unless final validation passes.
    /// This is an ordinary, non-secret API.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] for malformed input, allocation failure, an
    /// invalid `Buf` contract, or an underlying transform failure.
    fn decode_buf<B>(&self, input: B) -> Result<Bytes, BytesError>
    where
        B: Buf;

    /// Strictly decodes fragmented input transactionally under explicit
    /// limits.
    ///
    /// # Errors
    ///
    /// Returns [`BytesError`] if either limit is exceeded or decoding cannot
    /// complete.
    fn decode_buf_with_limits<B>(&self, input: B, limits: BytesLimits) -> Result<Bytes, BytesError>
    where
        B: Buf;
}

impl<S> Base64BytesExt<S> for Base64<S>
where
    S: Codec,
{
    fn bytes_encoder(&self) -> BytesEncoder {
        BytesEncoder::new(self.encoder(), BytesLimits::UNBOUNDED)
    }

    fn bytes_encoder_with_limits(&self, limits: BytesLimits) -> BytesEncoder {
        BytesEncoder::new(self.encoder(), limits)
    }

    fn bytes_decoder(&self) -> BytesDecoder {
        BytesDecoder::new(self.decoder(), BytesLimits::UNBOUNDED)
    }

    fn bytes_decoder_with_limits(&self, limits: BytesLimits) -> BytesDecoder {
        BytesDecoder::new(self.decoder(), limits)
    }

    fn encode_buf<B>(&self, input: B) -> Result<Bytes, BytesError>
    where
        B: Buf,
    {
        owned::encode(self, input, BytesLimits::UNBOUNDED)
    }

    fn encode_buf_with_limits<B>(&self, input: B, limits: BytesLimits) -> Result<Bytes, BytesError>
    where
        B: Buf,
    {
        owned::encode(self, input, limits)
    }

    fn decode_buf<B>(&self, input: B) -> Result<Bytes, BytesError>
    where
        B: Buf,
    {
        owned::decode(self, input, BytesLimits::UNBOUNDED)
    }

    fn decode_buf_with_limits<B>(&self, input: B, limits: BytesLimits) -> Result<Bytes, BytesError>
    where
        B: Buf,
    {
        owned::decode(self, input, limits)
    }
}
