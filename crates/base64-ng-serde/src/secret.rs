//! Fixed-capacity secret Serde field adapters.
//!
//! These adapters begin the fixed-work secret policy after the upstream
//! deserializer delivers encoded bytes. General Serde parsing remains
//! timing-variable and may allocate before that boundary. Successful output
//! is returned in wiping, non-Clone [`base64_ng::secret::SecretArray`] storage.
//! Errors are opaque and never include rejected bytes or positions.
//!
//! Serialization explicitly exposes secret bytes to the serializer as Base64
//! text. Use `deserialize_with` instead of `with` when a field must never be
//! serializable.

use base64_ng::{
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, secret::SecretArray,
};
use serde::{Deserializer, Serializer};

macro_rules! secret_codec_module {
    ($name:ident, $codec:expr, $description:literal) => {
        #[doc = $description]
        pub mod $name {
            use super::*;
            use serde::de::Error as _;

            /// Explicitly serializes secret bytes as visible Base64 text.
            ///
            /// # Errors
            ///
            /// Returns the serializer's error if encoding or value delivery
            /// fails.
            pub fn serialize<S, const CAP: usize>(
                bytes: &SecretArray<CAP>,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let exposed = bytes.expose_secret();
                crate::adapter::serialize_secret_codec(&$codec, exposed.as_bytes(), serializer)
            }

            /// Deserializes through a fixed-work frame into wiping storage.
            ///
            /// # Errors
            ///
            /// Returns the opaque `invalid secret base64 input` error for all
            /// malformed, oversized, unsupported, or internal failures.
            pub fn deserialize<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<SecretArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_secret(&$codec, deserializer)
            }

            /// Deserializes and requires exactly `CAP` secret bytes.
            ///
            /// # Errors
            ///
            /// Returns the same opaque error for malformed, oversized, and
            /// wrong-length input. Any rejected decoded value is wiped.
            pub fn deserialize_exact<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<SecretArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let decoded = deserialize(deserializer)?;
                if decoded.len() == CAP {
                    Ok(decoded)
                } else {
                    Err(D::Error::custom("invalid secret base64 input"))
                }
            }
        }
    };
}

secret_codec_module!(
    standard,
    STRICT_STANDARD_PADDED,
    "Fixed-work strict Standard padded secret fields."
);
secret_codec_module!(
    standard_no_pad,
    STRICT_STANDARD_UNPADDED,
    "Fixed-work strict Standard unpadded secret fields."
);
secret_codec_module!(
    url_safe,
    STRICT_URL_SAFE_PADDED,
    "Fixed-work strict URL-safe padded secret fields."
);
secret_codec_module!(
    url_safe_no_pad,
    STRICT_URL_SAFE_UNPADDED,
    "Fixed-work strict URL-safe unpadded secret fields."
);
