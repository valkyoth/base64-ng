//! Fixed-capacity ordinary Serde field adapters.
//!
//! These modules keep Base64-owned decoded storage on the stack, reject
//! capacities above [`crate::MAX_SERDE_STACK_DECODED_BYTES`] at compile time,
//! and reject encoded input beyond the derived public ceiling before full
//! validation. They do not bound allocation performed by the upstream data
//! format before the encoded value reaches this crate.

use base64_ng::{
    DecodedArray, MIME_BODY_STRICT, PEM_BODY_LF, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
};
use serde::{Deserializer, Serializer};

macro_rules! bounded_codec_module {
    ($name:ident, $codec:expr, $description:literal) => {
        #[doc = $description]
        pub mod $name {
            use super::*;
            use serde::de::Error as _;

            /// Serializes the initialized ordinary prefix as Base64 text.
            ///
            /// # Errors
            ///
            /// Returns the serializer's error if encoding or value delivery
            /// fails.
            pub fn serialize<S, const CAP: usize>(
                bytes: &DecodedArray<CAP>,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                crate::adapter::serialize_codec(&$codec, bytes.as_bytes(), serializer)
            }

            /// Deserializes into fixed-capacity ordinary decoded storage.
            ///
            /// `CAP` may not exceed
            /// [`crate::MAX_SERDE_STACK_DECODED_BYTES`]. Encoded input beyond
            /// its derived public ceiling is rejected before full validation.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error when the decoded value
            /// exceeds `CAP`, input is malformed, or transformation fails.
            pub fn deserialize<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<DecodedArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_bounded(&$codec, deserializer)
            }

            /// Deserializes and requires exactly `CAP` decoded bytes.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error when decoding fails or
            /// the decoded length differs from `CAP`.
            pub fn deserialize_exact<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<DecodedArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let decoded = deserialize(deserializer)?;
                if decoded.len() == CAP {
                    Ok(decoded)
                } else {
                    Err(D::Error::custom(
                        "decoded base64 length does not match required length",
                    ))
                }
            }
        }
    };
}

macro_rules! bounded_body_module {
    ($name:ident, $body:expr, $description:literal) => {
        #[doc = $description]
        pub mod $name {
            use super::*;
            use serde::de::Error as _;

            /// Serializes the initialized ordinary prefix as wrapped Base64.
            ///
            /// # Errors
            ///
            /// Returns the serializer's error if encoding, allocation, or
            /// value delivery fails.
            pub fn serialize<S, const CAP: usize>(
                bytes: &DecodedArray<CAP>,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                crate::adapter::serialize_body(&$body, bytes.as_bytes(), serializer)
            }

            /// Deserializes wrapped Base64 into fixed-capacity ordinary storage.
            ///
            /// `CAP` may not exceed
            /// [`crate::MAX_SERDE_STACK_DECODED_BYTES`]. Encoded body input
            /// beyond its derived wrapped ceiling is rejected before layout
            /// validation.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error when the decoded value
            /// exceeds `CAP`, layout or input is malformed, or transformation
            /// fails.
            pub fn deserialize<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<DecodedArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                crate::adapter::deserialize_body_bounded(&$body, deserializer)
            }

            /// Deserializes and requires exactly `CAP` decoded bytes.
            ///
            /// # Errors
            ///
            /// Returns a redacted deserializer error when decoding fails or
            /// the decoded length differs from `CAP`.
            pub fn deserialize_exact<'de, D, const CAP: usize>(
                deserializer: D,
            ) -> Result<DecodedArray<CAP>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let decoded = deserialize(deserializer)?;
                if decoded.len() == CAP {
                    Ok(decoded)
                } else {
                    Err(D::Error::custom(
                        "decoded base64 length does not match required length",
                    ))
                }
            }
        }
    };
}

bounded_codec_module!(
    standard,
    STRICT_STANDARD_PADDED,
    "Bounded strict Standard padded Base64 fields."
);
bounded_codec_module!(
    standard_no_pad,
    STRICT_STANDARD_UNPADDED,
    "Bounded strict Standard unpadded Base64 fields."
);
bounded_codec_module!(
    url_safe,
    STRICT_URL_SAFE_PADDED,
    "Bounded strict URL-safe padded Base64 fields."
);
bounded_codec_module!(
    url_safe_no_pad,
    STRICT_URL_SAFE_UNPADDED,
    "Bounded strict URL-safe unpadded Base64 fields."
);
bounded_body_module!(
    mime,
    MIME_BODY_STRICT,
    "Bounded strict MIME Base64 body fields."
);
bounded_body_module!(pem, PEM_BODY_LF, "Bounded strict PEM Base64 body fields.");
