//! Explicit expert compatibility configurations.
//!
//! These ordinary codecs match the corresponding policy combinations in
//! `base64` 0.23.0. They are neither WHATWG forgiving Base64 nor secret
//! codecs. Strict applications should use the `STRICT_*` presets instead.

use super::{
    alphabet::{STANDARD_ALPHABET, URL_SAFE_ALPHABET},
    specifications::{
        Base64, DecodePadding, EncodePadding, RuntimeSpec, TrailingBits, compatibility_codec,
    },
};

macro_rules! preset {
    ($name:ident, $alphabet:expr, $encode:expr, $decode:expr, $trailing:expr, $doc:literal) => {
        #[doc = $doc]
        pub const $name: Base64<RuntimeSpec> =
            compatibility_codec($alphabet, $encode, $decode, $trailing);
    };
}

preset!(
    STANDARD_PADDED_PADDING_INDIFFERENT,
    STANDARD_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::Indifferent,
    TrailingBits::RequireCanonical,
    "Standard encoding with padding and padding-indifferent canonical decoding."
);
preset!(
    STANDARD_UNPADDED_PADDING_INDIFFERENT,
    STANDARD_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Indifferent,
    TrailingBits::RequireCanonical,
    "Standard encoding without padding and padding-indifferent canonical decoding."
);
preset!(
    URL_SAFE_PADDED_PADDING_INDIFFERENT,
    URL_SAFE_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::Indifferent,
    TrailingBits::RequireCanonical,
    "URL-safe encoding with padding and padding-indifferent canonical decoding."
);
preset!(
    URL_SAFE_UNPADDED_PADDING_INDIFFERENT,
    URL_SAFE_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Indifferent,
    TrailingBits::RequireCanonical,
    "URL-safe encoding without padding and padding-indifferent canonical decoding."
);

preset!(
    STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
    STANDARD_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::RequireCanonical,
    TrailingBits::AllowNonCanonical,
    "Standard padded encoding and decoding that permits noncanonical trailing bits."
);
preset!(
    STANDARD_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
    STANDARD_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::AllowNonCanonical,
    "Standard unpadded encoding and decoding that permits noncanonical trailing bits."
);
preset!(
    URL_SAFE_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
    URL_SAFE_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::RequireCanonical,
    TrailingBits::AllowNonCanonical,
    "URL-safe padded encoding and decoding that permits noncanonical trailing bits."
);
preset!(
    URL_SAFE_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
    URL_SAFE_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::AllowNonCanonical,
    "URL-safe unpadded encoding and decoding that permits noncanonical trailing bits."
);

preset!(
    STANDARD_PADDED_FULL_COMPATIBILITY,
    STANDARD_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::Indifferent,
    TrailingBits::AllowNonCanonical,
    "Standard padded encoding with padding-indifferent, noncanonical-bit decoding."
);
preset!(
    STANDARD_UNPADDED_FULL_COMPATIBILITY,
    STANDARD_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Indifferent,
    TrailingBits::AllowNonCanonical,
    "Standard unpadded encoding with padding-indifferent, noncanonical-bit decoding."
);
preset!(
    URL_SAFE_PADDED_FULL_COMPATIBILITY,
    URL_SAFE_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::Indifferent,
    TrailingBits::AllowNonCanonical,
    "URL-safe padded encoding with padding-indifferent, noncanonical-bit decoding."
);
preset!(
    URL_SAFE_UNPADDED_FULL_COMPATIBILITY,
    URL_SAFE_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Indifferent,
    TrailingBits::AllowNonCanonical,
    "URL-safe unpadded encoding with padding-indifferent, noncanonical-bit decoding."
);
