#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional `serde` integration for `base64-ng`.
//!
//! This crate keeps serialization support out of the core package. It provides
//! explicit wrappers and `#[serde(with = "...")]` modules so applications must
//! choose the alphabet and padding policy at the field boundary.
//!
//! # Security
//!
//! Deserialization helpers in this crate use `base64_ng::Engine::decode_vec`,
//! the strict timing-variable decoder. They map decode failures to redacted
//! error classes, but they are not constant-time-oriented secret decoders. Do
//! not use these serde modules for API keys, bearer tokens, private keys, or
//! other secret-bearing fields when malformed-input timing matters. Decode
//! those values explicitly with `base64_ng::ct` or with
//! `base64_ng_sanitization::CtDecodeSanitizationExt` instead.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "alloc")]
use base64_ng::{
    Alphabet, DecodeError, Engine, MIME, PEM, Profile, STANDARD, STANDARD_NO_PAD, URL_SAFE,
    URL_SAFE_NO_PAD, clear_bytes, constant_time_eq,
};
#[cfg(feature = "alloc")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Owned bytes serialized as strict standard padded Base64.
///
/// This wrapper is still an interoperability type, not a secret container.
/// It clears its initialized bytes on drop as a retention-reduction measure,
/// but clones are independent copies and serialization intentionally exposes
/// the Base64 text to the serializer.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Base64Standard(Vec<u8>);

/// Owned bytes serialized as URL-safe unpadded Base64.
///
/// This wrapper is still an interoperability type, not a secret container.
/// It clears its initialized bytes on drop as a retention-reduction measure,
/// but clones are independent copies and serialization intentionally exposes
/// the Base64 text to the serializer.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Base64UrlSafeNoPad(Vec<u8>);

#[cfg(feature = "alloc")]
impl Base64Standard {
    /// Wraps bytes for standard Base64 serialization.
    #[must_use]
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns the wrapped bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the wrapper and returns the owned bytes.
    ///
    /// The returned vector is no longer cleared by this wrapper on drop.
    /// Callers handling sensitive values must apply their own cleanup policy.
    #[must_use]
    pub fn into_inner(mut self) -> Vec<u8> {
        core::mem::take(&mut self.0)
    }
}

#[cfg(feature = "alloc")]
impl Base64UrlSafeNoPad {
    /// Wraps bytes for URL-safe no-padding Base64 serialization.
    #[must_use]
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns the wrapped bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the wrapper and returns the owned bytes.
    ///
    /// The returned vector is no longer cleared by this wrapper on drop.
    /// Callers handling sensitive values must apply their own cleanup policy.
    #[must_use]
    pub fn into_inner(mut self) -> Vec<u8> {
        core::mem::take(&mut self.0)
    }
}

#[cfg(feature = "alloc")]
impl Drop for Base64Standard {
    fn drop(&mut self) {
        clear_bytes(&mut self.0);
    }
}

#[cfg(feature = "alloc")]
impl Drop for Base64UrlSafeNoPad {
    fn drop(&mut self) {
        clear_bytes(&mut self.0);
    }
}

#[cfg(feature = "alloc")]
impl PartialEq for Base64Standard {
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(self.as_bytes(), other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl Eq for Base64Standard {}

#[cfg(feature = "alloc")]
impl PartialEq for Base64UrlSafeNoPad {
    fn eq(&self, other: &Self) -> bool {
        constant_time_eq(self.as_bytes(), other.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl Eq for Base64UrlSafeNoPad {}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for Base64Standard {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Base64Standard")
            .field("bytes", &"<redacted>")
            .field("len", &self.0.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for Base64UrlSafeNoPad {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Base64UrlSafeNoPad")
            .field("bytes", &"<redacted>")
            .field("len", &self.0.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl Serialize for Base64Standard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        standard::serialize(&self.0, serializer)
    }
}

#[cfg(feature = "alloc")]
impl<'de> Deserialize<'de> for Base64Standard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        standard::deserialize(deserializer).map(Self)
    }
}

#[cfg(feature = "alloc")]
impl Serialize for Base64UrlSafeNoPad {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        url_safe_no_pad::serialize(&self.0, serializer)
    }
}

#[cfg(feature = "alloc")]
impl<'de> Deserialize<'de> for Base64UrlSafeNoPad {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        url_safe_no_pad::deserialize(deserializer).map(Self)
    }
}

/// Serde helpers for strict standard padded Base64 fields.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod standard {
    use super::{STANDARD, Vec, deserialize_with_engine, serialize_with_engine};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as strict standard padded Base64 text.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_engine(STANDARD, bytes.as_ref(), serializer)
    }

    /// Deserializes strict standard padded Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid strict standard padded Base64.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_engine(STANDARD, deserializer)
    }
}

/// Serde helpers for strict standard unpadded Base64 fields.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod standard_no_pad {
    use super::{STANDARD_NO_PAD, Vec, deserialize_with_engine, serialize_with_engine};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as strict standard unpadded Base64 text.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_engine(STANDARD_NO_PAD, bytes.as_ref(), serializer)
    }

    /// Deserializes strict standard unpadded Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid strict standard unpadded Base64.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_engine(STANDARD_NO_PAD, deserializer)
    }
}

/// Serde helpers for URL-safe padded Base64 fields.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod url_safe {
    use super::{URL_SAFE, Vec, deserialize_with_engine, serialize_with_engine};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as URL-safe padded Base64 text.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_engine(URL_SAFE, bytes.as_ref(), serializer)
    }

    /// Deserializes URL-safe padded Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid URL-safe padded Base64.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_engine(URL_SAFE, deserializer)
    }
}

/// Serde helpers for URL-safe unpadded Base64 fields.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod url_safe_no_pad {
    use super::{URL_SAFE_NO_PAD, Vec, deserialize_with_engine, serialize_with_engine};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as URL-safe unpadded Base64 text.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_engine(URL_SAFE_NO_PAD, bytes.as_ref(), serializer)
    }

    /// Deserializes URL-safe unpadded Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid URL-safe unpadded Base64.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_engine(URL_SAFE_NO_PAD, deserializer)
    }
}

/// Serde helpers for MIME Base64 fields with 76-column CRLF wrapping.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod mime {
    use super::{MIME, Vec, deserialize_with_profile, serialize_with_profile};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as MIME Base64 text with 76-column CRLF wrapping.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_profile(&MIME, bytes.as_ref(), serializer)
    }

    /// Deserializes MIME Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid strict MIME Base64 for the configured wrapping
    /// profile.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_profile(&MIME, deserializer)
    }
}

/// Serde helpers for PEM Base64 fields with 64-column LF wrapping.
///
/// # Security
///
/// Deserialization uses the strict timing-variable decoder. Use this module
/// for interoperability-oriented fields, not secret-bearing fields where
/// malformed-input timing matters.
#[cfg(feature = "alloc")]
pub mod pem {
    use super::{PEM, Vec, deserialize_with_profile, serialize_with_profile};
    use serde::{Deserializer, Serializer};

    /// Serializes bytes as PEM Base64 text with 64-column LF wrapping.
    ///
    /// # Errors
    ///
    /// Returns the serializer's error if Base64 encoding fails or the
    /// serializer rejects the string value.
    pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_with_profile(&PEM, bytes.as_ref(), serializer)
    }

    /// Deserializes PEM Base64 text into owned bytes.
    ///
    /// # Errors
    ///
    /// Returns the deserializer's error if the value is not a string or if the
    /// string is not valid strict PEM Base64 for the configured wrapping
    /// profile.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_with_profile(&PEM, deserializer)
    }
}

#[cfg(feature = "alloc")]
fn serialize_with_engine<A, const PAD: bool, S>(
    engine: Engine<A, PAD>,
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    A: base64_ng::Alphabet,
    S: Serializer,
{
    let encoded = engine
        .encode_string(bytes)
        .map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&encoded)
}

#[cfg(feature = "alloc")]
fn serialize_with_profile<A, const PAD: bool, S>(
    profile: &Profile<A, PAD>,
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    A: Alphabet,
    S: Serializer,
{
    let encoded = profile
        .encode_string(bytes)
        .map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&encoded)
}

#[cfg(feature = "alloc")]
fn deserialize_with_engine<'de, A, const PAD: bool, D>(
    engine: Engine<A, PAD>,
    deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
    A: base64_ng::Alphabet,
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    engine
        .decode_vec(encoded.as_bytes())
        .map_err(|error: DecodeError| D::Error::custom(error.kind()))
}

#[cfg(feature = "alloc")]
fn deserialize_with_profile<'de, A, const PAD: bool, D>(
    profile: &Profile<A, PAD>,
    deserializer: D,
) -> Result<Vec<u8>, D::Error>
where
    A: Alphabet,
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    profile
        .decode_vec(encoded.as_bytes())
        .map_err(|error: DecodeError| D::Error::custom(error.kind()))
}
