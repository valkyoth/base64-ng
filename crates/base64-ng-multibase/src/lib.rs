#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Bounded support for the registered Base64-family multibase encodings.
//!
//! This crate implements only `base64` (`m`), `base64pad` (`M`), `base64url`
//! (`u`), and `base64urlpad` (`U`) from the pinned multibase registry. It is
//! not a complete multibase implementation and rejects every other prefix.
//! All decoding is strict and canonical for the prefix-selected RFC 4648
//! alphabet and padding policy.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod alloc_api;
mod encoding;
mod error;
mod incremental;
mod limits;
mod one_shot;
mod types;

#[cfg(feature = "alloc")]
pub use alloc_api::{decode_base64_multibase_to_vec, encode_base64_multibase_to_string};
pub use encoding::{Base64MultibaseEncoding, MultibaseRegistryStatus};
pub use error::{Base64MultibaseError, Base64MultibaseErrorKind};
pub use incremental::{Base64MultibaseDecoder, Base64MultibaseEncoder};
pub use limits::Base64MultibaseLimits;
pub use one_shot::{
    base64_multibase_encoded_len, decode_base64_multibase_into, encode_base64_multibase_into,
    validate_base64_multibase,
};
#[cfg(feature = "alloc")]
pub use types::DecodedBase64MultibaseVec;
pub use types::{Base64MultibaseStatus, Base64MultibaseStep, DecodedBase64Multibase};
