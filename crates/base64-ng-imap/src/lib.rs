#![no_std]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded RFC 3501 modified-Base64 payload transforms.
//!
//! This crate operates only on the payload bytes inside one modified UTF-7
//! shifted run. Inputs to encoding are already-converted UTF-16BE octets;
//! decoded outputs remain UTF-16BE octets. The crate does not convert Unicode,
//! emit or parse `&` and `-` shift delimiters, apply the printable-ASCII rule,
//! or claim complete IMAP mailbox-name support.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod alloc_api;
mod error;
mod incremental;
mod limits;
mod one_shot;
mod types;

#[cfg(feature = "alloc")]
pub use alloc_api::{decode_modified_utf7_payload_to_vec, encode_modified_utf7_payload_to_string};
pub use error::{ImapPayloadError, ImapPayloadErrorKind};
pub use incremental::{ModifiedUtf7PayloadDecoder, ModifiedUtf7PayloadEncoder};
pub use limits::ImapPayloadLimits;
pub use one_shot::{
    decode_modified_utf7_payload_into, encode_modified_utf7_payload_into,
    modified_utf7_payload_decoded_len, modified_utf7_payload_encoded_len,
    validate_modified_utf7_payload,
};
pub use types::{ImapPayloadStatus, ImapPayloadStep};
