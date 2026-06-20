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

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "alloc")]
use base64_ng::{DecodeError, Engine, STANDARD, URL_SAFE_NO_PAD};
#[cfg(feature = "alloc")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Owned bytes serialized as strict standard padded Base64.
#[cfg(feature = "alloc")]
#[derive(Clone, Eq, PartialEq)]
pub struct Base64Standard(Vec<u8>);

/// Owned bytes serialized as URL-safe unpadded Base64.
#[cfg(feature = "alloc")]
#[derive(Clone, Eq, PartialEq)]
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
    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.0
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
    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

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

/// Serde helpers for URL-safe unpadded Base64 fields.
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
