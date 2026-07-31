//! Accurately scoped Base64 body and alphabet presets.

use super::{
    alphabet::{BCRYPT_ALPHABET, CRYPT_ALPHABET, IMAP_MUTF7_ALPHABET, PBKDF2_ALPHABET},
    specifications::{
        Base64, DecodePadding, EncodePadding, RuntimeSpec, STRICT_STANDARD_PADDED,
        StrictStandardPadded, TrailingBits, runtime_codec,
    },
    wrapping::LineWrap,
};

/// One Base64 codec paired with an encoded-body line layout.
///
/// This value describes body bytes only. It does not parse MIME headers, PEM
/// labels or boundaries, or any surrounding protocol container.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BodyCodec<S> {
    codec: Base64<S>,
    wrapping: LineWrap,
}

impl<S> BodyCodec<S> {
    const fn new(codec: Base64<S>, wrapping: LineWrap) -> Self {
        Self { codec, wrapping }
    }

    /// Returns the complete Base64 alphabet and padding policy.
    #[must_use]
    pub const fn codec(&self) -> &Base64<S> {
        &self.codec
    }

    /// Returns the encoded-body wrapping policy.
    #[must_use]
    pub const fn wrapping(&self) -> LineWrap {
        self.wrapping
    }
}

/// Strict Standard Base64 with MIME's 76-column CRLF body layout.
///
/// This is not a MIME message, header, or content-transfer parser.
pub const MIME_BODY_STRICT: BodyCodec<StrictStandardPadded> =
    BodyCodec::new(STRICT_STANDARD_PADDED, LineWrap::MIME_BODY_WRAP);

/// Strict Standard Base64 with a 64-column LF PEM body layout.
///
/// This does not parse PEM labels, boundaries, headers, or surrounding text.
pub const PEM_BODY_LF: BodyCodec<StrictStandardPadded> =
    BodyCodec::new(STRICT_STANDARD_PADDED, LineWrap::PEM_BODY_LF_WRAP);

/// Strict Standard Base64 with a 64-column CRLF PEM body layout.
///
/// This does not parse PEM labels, boundaries, headers, or surrounding text.
pub const PEM_BODY_CRLF: BodyCodec<StrictStandardPadded> =
    BodyCodec::new(STRICT_STANDARD_PADDED, LineWrap::PEM_BODY_CRLF_WRAP);

/// Standard bit grouping with the bcrypt alphabet and no padding.
///
/// This does not parse or construct bcrypt password records.
pub const BCRYPT_ALPHABET_NO_PAD: Base64<RuntimeSpec> = runtime_codec(
    BCRYPT_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::RequireCanonical,
);

/// Standard bit grouping with the `crypt(3)` alphabet and no padding.
///
/// This does not implement the little-endian field transforms used by several
/// password-record formats and does not parse a `crypt(3)` record.
pub const CRYPT_ALPHABET_NO_PAD: Base64<RuntimeSpec> = runtime_codec(
    CRYPT_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::RequireCanonical,
);

/// Standard bit grouping with the PBKDF2-adapted alphabet and no padding.
///
/// The alphabet replaces RFC 4648 Standard `+` with `.`. This value does not
/// parse Passlib records or enforce salt and checksum lengths.
pub const PBKDF2_ALPHABET_NO_PAD: Base64<RuntimeSpec> = runtime_codec(
    PBKDF2_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::RequireCanonical,
);

/// Standard bit grouping with the IMAP modified-UTF-7 alphabet and no padding.
///
/// This value performs alphabet-level Base64 only. It does not shift UTF-16BE,
/// delimit modified-UTF-7 runs, or encode mailbox names.
pub const IMAP_MUTF7_ALPHABET_NO_PAD: Base64<RuntimeSpec> = runtime_codec(
    IMAP_MUTF7_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid,
    TrailingBits::RequireCanonical,
);
