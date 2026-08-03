#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded RFC 7468 textual encoding parsing and generation.
//!
//! This crate owns complete encapsulation boundaries, labels, Base64 bodies,
//! adjacent text, multiple-block documents, and line-ending policy. It does
//! not parse the ASN.1 payload and deliberately rejects legacy OpenSSL
//! encapsulated headers.

extern crate alloc;

mod error;
mod generator;
mod label;
mod limits;
mod parser;
#[cfg(feature = "secrets")]
mod secret;
mod types;

pub use error::{PemError, PemErrorKind};
pub use generator::{
    PemBlockEncoder, encode_pem_block_into, encode_pem_block_to_string, pem_block_encoded_len,
};
pub use label::{PemLabel, PemLabelError};
pub use limits::PemLimits;
pub use parser::{PemDocumentParser, parse_pem_document};
#[cfg(feature = "secrets")]
pub use secret::{SecretPemBlock, parse_pem_secret_block};
pub use types::{
    PemBlock, PemDocument, PemGenerationOptions, PemLineEnding, PemParsePolicy, PemParseReport,
};
