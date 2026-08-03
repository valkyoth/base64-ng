#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded RFC 9580 `OpenPGP` ASCII armor parsing and generation.
//!
//! This crate owns complete ordinary armor framing. It does not interpret
//! `OpenPGP` packets and does not implement the separate cleartext signature
//! framework from RFC 9580 Section 7.

extern crate alloc;

mod crc24;
mod error;
mod generator;
mod limits;
mod parser;
#[cfg(feature = "secrets")]
mod secret;
#[cfg(feature = "std")]
mod std_io;
mod types;

pub use error::{OpenPgpError, OpenPgpErrorKind};
pub use generator::{ArmorEncoder, armor_encoded_len, encode_armor_into, encode_armor_to_string};
pub use limits::OpenPgpLimits;
pub use parser::{ArmorDocumentParser, parse_armor_document};
#[cfg(feature = "secrets")]
pub use secret::{SecretArmorBlock, parse_secret_armor_block};
#[cfg(feature = "std")]
pub use std_io::{read_armor_document, write_armor_block};
pub use types::{
    ArmorBlock, ArmorDocument, ArmorHeader, ArmorType, ChecksumGeneration, ChecksumPolicy,
    ChecksumStatus, GenerationOptions, LineEnding,
};
