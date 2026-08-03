#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional `serde` integration for the `base64-ng` 2.0 codec model.
//!
//! Human-readable serializers receive Base64 text through `serialize_str`.
//! Binary serializers receive the same Base64 text bytes through
//! `serialize_bytes`; this crate never silently substitutes raw plaintext.
//! Deserializers are asked for borrowed strings or bytes where their data
//! format permits it, avoiding a Base64-owned encoded-input copy.
//!
//! The compatibility field modules return an exactly sized `Vec<u8>`. The
//! [`bounded`] modules decode into fixed-capacity ordinary arrays, while the
//! optional [`secret`] modules use the fixed-work, wiping secret decoder.
//!
//! # Bounds and secrecy
//!
//! Limits in this crate cover Base64-owned decoded output and intermediate
//! materialization only. A data format may allocate while parsing before its
//! Serde deserializer delivers a string or byte slice; that allocation is
//! outside this crate's control.
//!
//! JSON and general Serde parsing are not constant-time-oriented. Secret
//! timing policy begins only after encoded input reaches this crate. Secret
//! adapters prefer borrowed input, allocate no decoded heap output, return
//! opaque errors, and do not release plaintext before final validation. When
//! a deserializer transfers an owned string or byte vector, the secret visitor
//! wipes its complete allocation on return, error, or unwind. Borrowed input
//! and copies retained by serializers remain outside this crate's ownership.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod adapter;
#[cfg(feature = "secrets")]
mod adapter_secret;
#[cfg(all(test, feature = "secrets"))]
mod adapter_tests;
#[cfg(feature = "alloc")]
pub mod bounded;
#[cfg(feature = "alloc")]
mod fields;
#[cfg(feature = "secrets")]
pub mod secret;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED, clear_bytes, constant_time_eq};
#[cfg(feature = "alloc")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "alloc")]
pub use fields::{mime, pem, standard, standard_no_pad, url_safe, url_safe_no_pad};

/// Owned bytes serialized as strict Standard padded Base64.
///
/// This is an interoperability type, not a secret container. It clears its
/// initialized bytes on drop, but clones are independent copies and
/// serialization deliberately exposes Base64 text to the serializer.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Base64Standard(Vec<u8>);

/// Owned bytes serialized as strict URL-safe unpadded Base64.
///
/// This is an interoperability type, not a secret container. It clears its
/// initialized bytes on drop, but clones are independent copies and
/// serialization deliberately exposes Base64 text to the serializer.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Base64UrlSafeNoPad(Vec<u8>);

#[cfg(feature = "alloc")]
macro_rules! owned_wrapper {
    ($name:ident, $codec:expr, $label:literal) => {
        impl $name {
            /// Wraps ordinary bytes for Base64 serialization.
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
            /// The returned vector is no longer cleared by this wrapper.
            #[must_use]
            pub fn into_inner(mut self) -> Vec<u8> {
                core::mem::take(&mut self.0)
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                clear_bytes(&mut self.0);
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                constant_time_eq(self.as_bytes(), other.as_bytes())
            }
        }

        impl Eq for $name {}

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .debug_struct($label)
                    .field("bytes", &"<redacted>")
                    .field("len", &self.0.len())
                    .finish()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                adapter::serialize_codec(&$codec, &self.0, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                adapter::deserialize_vec(&$codec, deserializer).map(Self)
            }
        }
    };
}

#[cfg(feature = "alloc")]
owned_wrapper!(Base64Standard, STRICT_STANDARD_PADDED, "Base64Standard");
#[cfg(feature = "alloc")]
owned_wrapper!(
    Base64UrlSafeNoPad,
    STRICT_URL_SAFE_UNPADDED,
    "Base64UrlSafeNoPad"
);
