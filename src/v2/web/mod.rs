//! Exact WHATWG forgiving Base64 decoding for ordinary web-compatible input.
//!
//! [`FORGIVING`] follows the WHATWG Infra Standard algorithm: it removes
//! ASCII whitespace, accepts omitted canonical padding, rejects impossible
//! lengths and non-Standard symbols, and discards unused trailing bits.
//! This is deliberately separate from strict RFC 4648 and `secret::*` APIs.
//! It is not a constant-time or zeroizing interface.

mod decoder;
mod one_shot;
#[cfg(feature = "alloc")]
mod one_shot_alloc;

pub use decoder::{ForgivingDecoder, ForgivingError};

/// Exact WHATWG forgiving Base64 decoder.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ForgivingBase64;

/// Exact WHATWG forgiving Base64 decoder.
pub const FORGIVING: ForgivingBase64 = ForgivingBase64;
