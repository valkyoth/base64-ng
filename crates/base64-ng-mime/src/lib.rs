#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded RFC 2045 Base64 content-transfer body support.
//!
//! This crate encodes and decodes the Base64 body bytes described by RFC 2045
//! Section 6.8. It does not parse MIME headers, multipart boundaries, media
//! types, messages, or body-part containers. Ordinary compatible decoding
//! ignores bytes outside RFC 2045 Table 1 only under explicit finite limits
//! and reports suspicious transport content to the caller.

#[cfg(feature = "alloc")]
extern crate alloc;

mod decoder;
mod encoder;
mod error;
mod limits;
mod one_shot;
mod types;

pub use decoder::MimeBodyDecoder;
pub use encoder::MimeBodyEncoder;
pub use error::{MimeBodyError, MimeBodyErrorKind};
pub use limits::MimeBodyLimits;
pub use one_shot::{
    decode_mime_content_transfer_body_into, encode_mime_content_transfer_body_into,
    mime_content_transfer_body_encoded_len, validate_mime_content_transfer_body,
};
#[cfg(feature = "alloc")]
pub use one_shot::{
    decode_mime_content_transfer_body_to_vec, encode_mime_content_transfer_body_to_string,
};
pub use types::{
    MimeBodyDecodePolicy, MimeBodyDecodeReport, MimeBodyProgress, MimeBodyStatus, MimeBodyStep,
    MimeBodyTerminalLineEnding,
};
