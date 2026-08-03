#![no_std]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded Base64 field and record transforms for selected password formats.
//!
//! This crate parses and generates Passlib PBKDF2 and SHA-crypt records from
//! caller-provided salt and checksum bytes. It never accepts passwords,
//! derives hashes, verifies passwords, or recommends these formats for new
//! password storage.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod alloc_api;
mod error;
mod limits;
mod pbkdf2;
mod sha_crypt;
mod types;

#[cfg(feature = "alloc")]
pub use alloc_api::{generate_pbkdf2_record, generate_sha_crypt_record};
pub use error::{PasswordRecordError, PasswordRecordErrorKind};
pub use limits::PasswordRecordLimits;
pub use pbkdf2::{
    decode_pbkdf2_field_into, encode_pbkdf2_field_into, generate_pbkdf2_record_into,
    parse_pbkdf2_record, pbkdf2_record_len,
};
pub use sha_crypt::{
    decode_sha_crypt_checksum_into, encode_sha_crypt_checksum_into, generate_sha_crypt_record_into,
    parse_sha_crypt_record, sha_crypt_record_len,
};
pub use types::{
    PasslibPbkdf2Algorithm, PasslibPbkdf2Record, ShaCryptAlgorithm, ShaCryptRecord, ShaCryptRounds,
};
