use alloc::vec::Vec;

use base64_ng::{
    MIME_BODY_STRICT, PEM_BODY_LF, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
};
use serde::{Deserializer, Serializer};

macro_rules! codec_field_module {
    ($name:ident, $codec:expr, $description:literal) => {
        #[doc = $description]
        ///
        /// Human-readable formats use a string. Binary formats use a byte
        /// string containing the same Base64 text. Deserialization is strict,
        /// timing-variable, and returns redacted malformed-input classes.
        pub mod $name {
            use super::*;

            /// Serializes ordinary bytes as Base64 text.
            ///
            /// # Errors
            ///
            /// Returns the serializer's error if encoding or value delivery
            /// fails.
            pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                crate::adapter::serialize_codec(&$codec, bytes.as_ref(), serializer)
            }

            /// Deserializes Base64 text into an exactly sized owned vector.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error for malformed input,
            /// allocation failure, or an internal transform failure.
            pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_vec(&$codec, deserializer)
            }

            /// Deserializes Base64 text with a caller-selected decoded limit.
            ///
            /// Encoded input beyond the derived public ceiling is rejected
            /// before full validation or decoded-output allocation.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error for a limit violation,
            /// malformed input, allocation failure, or transform failure.
            pub fn deserialize_with_limit<'de, D, const MAX: usize>(
                deserializer: D,
            ) -> Result<Vec<u8>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_vec_with_limit(&$codec, deserializer, MAX)
            }
        }
    };
}

macro_rules! body_field_module {
    ($name:ident, $body:expr, $description:literal) => {
        #[doc = $description]
        ///
        /// This adapter handles encoded body bytes only, not the surrounding
        /// MIME or PEM container. It validates exact line layout before
        /// decoding and does not allocate a compacted encoded-input copy.
        pub mod $name {
            use super::*;

            /// Serializes ordinary bytes as wrapped Base64 body text.
            ///
            /// # Errors
            ///
            /// Returns the serializer's error if encoding, allocation, or
            /// value delivery fails.
            pub fn serialize<S>(bytes: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                crate::adapter::serialize_body(&$body, bytes.as_ref(), serializer)
            }

            /// Deserializes strict wrapped Base64 body text.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error for malformed layout or
            /// Base64, allocation failure, or an internal transform failure.
            pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_body_vec(&$body, deserializer)
            }

            /// Deserializes wrapped Base64 with a caller-selected decoded limit.
            ///
            /// Encoded body input beyond the derived wrapped ceiling is
            /// rejected before layout validation or decoded-output allocation.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error for a limit violation,
            /// malformed input or layout, allocation failure, or transform
            /// failure.
            pub fn deserialize_with_limit<'de, D, const MAX: usize>(
                deserializer: D,
            ) -> Result<Vec<u8>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_body_vec_with_limit(&$body, deserializer, MAX)
            }
        }
    };
}

codec_field_module!(
    standard,
    STRICT_STANDARD_PADDED,
    "Serde helpers for strict Standard padded Base64 fields."
);
codec_field_module!(
    standard_no_pad,
    STRICT_STANDARD_UNPADDED,
    "Serde helpers for strict Standard unpadded Base64 fields."
);
codec_field_module!(
    url_safe,
    STRICT_URL_SAFE_PADDED,
    "Serde helpers for strict URL-safe padded Base64 fields."
);
codec_field_module!(
    url_safe_no_pad,
    STRICT_URL_SAFE_UNPADDED,
    "Serde helpers for strict URL-safe unpadded Base64 fields."
);
body_field_module!(
    mime,
    MIME_BODY_STRICT,
    "Serde helpers for strict MIME Base64 body fields with 76-column CRLF wrapping."
);
body_field_module!(
    pem,
    PEM_BODY_LF,
    "Serde helpers for strict PEM Base64 body fields with 64-column LF wrapping."
);
