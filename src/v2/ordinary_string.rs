//! Policy-carrying ordinary Base64 strings.

use alloc::string::String;

use super::{
    ordinary::OneShotError,
    specifications::{Base64, Codec, CodecSettings},
};

/// An owned ordinary Base64 string validated by one exact codec policy.
///
/// The value retains the [`Base64<S>`] used to encode or validate its text, so
/// its [`decode`](Self::decode) methods cannot accidentally select a different
/// alphabet, padding, or trailing-bit policy. Construction either encodes
/// bytes through that codec or validates complete encoded text before
/// ownership is returned. There is deliberately no mutable string access.
///
/// This is an ordinary, visibly printable, cloneable value. It performs no
/// cleanup and is not suitable for keys, tokens, passwords, or other secret
/// material. Use the `secret` module for secret-bearing data.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Base64String<S: Codec> {
    codec: Base64<S>,
    encoded: String,
}

impl<S: Codec> Base64String<S> {
    /// Encodes bytes and retains the exact codec policy with the result.
    ///
    /// Allocation and length errors use the same contract as
    /// [`Base64::encode_to_string`].
    pub fn encode(codec: Base64<S>, input: &[u8]) -> Result<Self, OneShotError> {
        let encoded = codec.encode_to_string(input)?;
        Ok(Self { codec, encoded })
    }

    /// Validates and adopts an existing owned string without copying it.
    ///
    /// The complete string must satisfy the supplied codec's decode policy.
    /// On error, this function consumes and drops the supplied ordinary
    /// string.
    pub fn from_string(codec: Base64<S>, encoded: String) -> Result<Self, OneShotError> {
        codec.validate(encoded.as_bytes())?;
        Ok(Self { codec, encoded })
    }

    /// Validates and copies an encoded string slice into owned storage.
    ///
    /// Validation completes before allocation. The copy uses
    /// `try_reserve_exact`, returning [`OneShotError::AllocationFailed`] if the
    /// reservation cannot be made.
    pub fn parse(codec: Base64<S>, encoded: &str) -> Result<Self, OneShotError> {
        Self::parse_with_reserver(codec, encoded, |output, required| {
            output
                .try_reserve_exact(required)
                .map_err(|_| OneShotError::AllocationFailed {
                    requested: required,
                })
        })
    }

    fn parse_with_reserver<F>(
        codec: Base64<S>,
        encoded: &str,
        reserve: F,
    ) -> Result<Self, OneShotError>
    where
        F: FnOnce(&mut String, usize) -> Result<(), OneShotError>,
    {
        codec.validate(encoded.as_bytes())?;
        let mut owned = String::new();
        reserve(&mut owned, encoded.len())?;
        owned.push_str(encoded);
        Ok(Self {
            codec,
            encoded: owned,
        })
    }

    #[cfg(test)]
    pub(super) fn parse_with_injected_reserver<F>(
        codec: Base64<S>,
        encoded: &str,
        reserve: F,
    ) -> Result<Self, OneShotError>
    where
        F: FnOnce(&mut String, usize) -> Result<(), OneShotError>,
    {
        Self::parse_with_reserver(codec, encoded, reserve)
    }

    /// Returns the exact codec retained by this string.
    #[must_use]
    pub const fn codec(&self) -> &Base64<S> {
        &self.codec
    }

    /// Returns the retained codec settings.
    #[must_use]
    pub fn settings(&self) -> CodecSettings {
        self.codec.settings()
    }

    /// Returns the validated encoded text.
    ///
    /// Passing this ordinary view to another codec can deliberately discard
    /// the retained policy. Use [`Self::decode`] to preserve it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.encoded.as_str()
    }

    /// Returns the validated encoded bytes.
    ///
    /// Passing this ordinary view to another codec can deliberately discard
    /// the retained policy. Use [`Self::decode`] to preserve it.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.encoded.as_bytes()
    }

    /// Returns the encoded byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.encoded.len()
    }

    /// Returns whether the encoded text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.encoded.is_empty()
    }

    /// Decodes the validated text with its retained codec.
    pub fn decode(&self) -> Result<alloc::vec::Vec<u8>, OneShotError> {
        self.codec.decode_to_vec(self.encoded.as_bytes())
    }

    /// Decodes with an exact maximum output length.
    pub fn decode_with_limit(
        &self,
        max_output_len: usize,
    ) -> Result<alloc::vec::Vec<u8>, OneShotError> {
        self.codec
            .decode_to_vec_with_limit(self.encoded.as_bytes(), max_output_len)
    }

    /// Consumes the wrapper and returns the validated ordinary string.
    ///
    /// The returned `String` no longer carries the codec policy.
    #[must_use]
    pub fn into_string(self) -> String {
        self.encoded
    }
}

impl<S: Codec> AsRef<str> for Base64String<S> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<S: Codec> AsRef<[u8]> for Base64String<S> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<S: Codec> core::fmt::Display for Base64String<S> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
