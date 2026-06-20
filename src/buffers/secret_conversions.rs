use super::secret::SecretBuffer;
use crate::{DecodeError, DecodedBuffer, EncodedBuffer, STANDARD};

#[cfg(feature = "alloc")]
impl From<alloc::vec::Vec<u8>> for SecretBuffer {
    /// Wraps an owned vector as sensitive material.
    ///
    /// Spare capacity is cleared immediately before the vector is stored.
    /// Use [`SecretBuffer::from_slice`] when the source data is borrowed.
    fn from(bytes: alloc::vec::Vec<u8>) -> Self {
        Self::from_vec(bytes)
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::string::String> for SecretBuffer {
    /// Wraps an owned UTF-8 string as sensitive material.
    ///
    /// The string is consumed without copying its initialized bytes. Spare
    /// vector capacity is cleared immediately before the bytes are stored.
    fn from(text: alloc::string::String) -> Self {
        Self::from_vec(text.into_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> From<EncodedBuffer<CAP>> for SecretBuffer {
    /// Copies visible encoded bytes from a stack-backed buffer into an owned
    /// redacted buffer.
    ///
    /// The consumed stack-backed buffer clears its backing array when it is
    /// dropped at the end of the conversion.
    fn from(buffer: EncodedBuffer<CAP>) -> Self {
        Self::from_slice(buffer.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl<const CAP: usize> From<DecodedBuffer<CAP>> for SecretBuffer {
    /// Copies visible decoded bytes from a stack-backed buffer into an owned
    /// redacted buffer.
    ///
    /// The consumed stack-backed buffer clears its backing array when it is
    /// dropped at the end of the conversion.
    fn from(buffer: DecodedBuffer<CAP>) -> Self {
        Self::from_slice(buffer.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&[u8]> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 into a redacted owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`]
    /// when a different alphabet, padding mode, or line-wrapping profile is
    /// required. These conversions always use [`crate::STANDARD`]; URL-safe,
    /// bcrypt, crypt, MIME, PEM, and custom alphabets must use an explicit
    /// engine or profile.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::CtEngine::decode_secret`] through
    /// [`crate::ct::STANDARD`], or use staged decode and then wrap the
    /// successful output in `SecretBuffer`.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.decode_secret(input)
    }
}

#[cfg(feature = "alloc")]
impl<const N: usize> TryFrom<&[u8; N]> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes a strict standard padded Base64 byte array into a redacted
    /// owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`]
    /// when a different alphabet, padding mode, or line-wrapping profile is
    /// required. These conversions always use [`crate::STANDARD`]; URL-safe,
    /// bcrypt, crypt, MIME, PEM, and custom alphabets must use an explicit
    /// engine or profile.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::CtEngine::decode_secret`] through
    /// [`crate::ct::STANDARD`], or use staged decode and then wrap the
    /// successful output in `SecretBuffer`.
    fn try_from(input: &[u8; N]) -> Result<Self, Self::Error> {
        Self::try_from(&input[..])
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&str> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 text into a redacted owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`]
    /// when a different alphabet, padding mode, or line-wrapping profile is
    /// required. These conversions always use [`crate::STANDARD`]; URL-safe,
    /// bcrypt, crypt, MIME, PEM, and custom alphabets must use an explicit
    /// engine or profile.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::CtEngine::decode_secret`] through
    /// [`crate::ct::STANDARD`], or use staged decode and then wrap the
    /// successful output in `SecretBuffer`.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl core::str::FromStr for SecretBuffer {
    type Err = DecodeError;

    /// Decodes strict standard padded Base64 text into a redacted owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`]
    /// when a different alphabet, padding mode, or line-wrapping profile is
    /// required. These conversions always use [`crate::STANDARD`]; URL-safe,
    /// bcrypt, crypt, MIME, PEM, and custom alphabets must use an explicit
    /// engine or profile.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::CtEngine::decode_secret`] through
    /// [`crate::ct::STANDARD`], or use staged decode and then wrap the
    /// successful output in `SecretBuffer`.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::try_from(input)
    }
}
