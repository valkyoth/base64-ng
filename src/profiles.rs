//! Named Base64 profiles with optional strict line wrapping.

use crate::{
    Alphabet, BCRYPT_NO_PAD, Bcrypt, CRYPT_NO_PAD, Crypt, DecodeError, DecodedBuffer, EncodeError,
    EncodedBuffer, Engine, LineEnding, LineWrap, STANDARD, Standard, checked_encoded_len,
    checked_wrapped_encoded_len, encoded_len, wrapped_encoded_len,
};

#[cfg(feature = "alloc")]
use crate::SecretBuffer;

/// A named Base64 profile with an engine and optional strict line wrapping.
///
/// Profiles are convenience values for protocol-shaped Base64. They keep the
/// same strict alphabet, padding, canonical-bit, and output-buffer rules as
/// [`Engine`], while carrying the wrapping policy for MIME/PEM-like formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile<A, const PAD: bool> {
    engine: Engine<A, PAD>,
    wrap: Option<LineWrap>,
}

impl<A, const PAD: bool> Profile<A, PAD>
where
    A: Alphabet,
{
    /// Creates a profile from an engine and optional strict line wrapping.
    #[must_use]
    pub const fn new(engine: Engine<A, PAD>, wrap: Option<LineWrap>) -> Self {
        Self { engine, wrap }
    }

    /// Creates a profile, returning `None` when the wrapping policy is invalid.
    ///
    /// This is useful when a profile is assembled from configuration or other
    /// untrusted metadata. Use [`Self::new`] for compile-time constants where
    /// the wrapping policy is known to be valid.
    #[must_use]
    pub const fn checked_new(engine: Engine<A, PAD>, wrap: Option<LineWrap>) -> Option<Self> {
        match wrap {
            Some(wrap) if !wrap.is_valid() => None,
            _ => Some(Self::new(engine, wrap)),
        }
    }

    /// Returns whether this profile can be used by encoders and decoders.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        match self.wrap {
            Some(wrap) => wrap.is_valid(),
            None => true,
        }
    }

    /// Returns the underlying engine.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this profile uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns whether this profile carries a strict line-wrapping policy.
    #[must_use]
    pub const fn is_wrapped(&self) -> bool {
        self.wrap.is_some()
    }

    /// Returns the strict wrapping policy carried by this profile, if any.
    #[must_use]
    pub const fn line_wrap(&self) -> Option<LineWrap> {
        self.wrap
    }

    /// Returns the encoded line length for wrapped profiles.
    #[must_use]
    pub const fn line_len(&self) -> Option<usize> {
        match self.wrap {
            Some(wrap) => Some(wrap.line_len()),
            None => None,
        }
    }

    /// Returns the line ending for wrapped profiles.
    #[must_use]
    pub const fn line_ending(&self) -> Option<LineEnding> {
        match self.wrap {
            Some(wrap) => Some(wrap.line_ending()),
            None => None,
        }
    }

    /// Returns the encoded length for this profile.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => wrapped_encoded_len(input_len, PAD, wrap),
            None => encoded_len(input_len, PAD),
        }
    }

    /// Returns the encoded length for this profile, or `None` on overflow or
    /// invalid line wrapping.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        match self.wrap {
            Some(wrap) => checked_wrapped_encoded_len(input_len, PAD, wrap),
            None => checked_encoded_len(input_len, PAD),
        }
    }

    /// Returns the exact decoded length for this profile.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decoded_len_wrapped(input, wrap),
            None => self.engine.decoded_len(input),
        }
    }

    /// Validates input according to this profile without writing decoded bytes.
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.validate_wrapped_result(input, wrap),
            None => self.engine.validate_result(input),
        }
    }

    /// Returns whether `input` is valid for this profile.
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Encodes `input` into `output` according to this profile.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_slice_wrapped(input, output, wrap),
            None => self.engine.encode_slice(input, output),
        }
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        match self.wrap {
            Some(wrap) => self
                .engine
                .encode_slice_wrapped_clear_tail(input, output, wrap),
            None => self.engine.encode_slice_clear_tail(input, output),
        }
    }

    /// Encodes `input` into a stack-backed buffer.
    ///
    /// This is useful for short values where heap allocation is unnecessary.
    /// If encoding fails, the internal backing array is cleared before the
    /// error is returned.
    pub fn encode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written = match self.encode_slice_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` into `output` according to this profile.
    ///
    /// # Security
    ///
    /// Profile decoders use the normal strict decode path. They may branch or
    /// return early based on malformed input, padding position, wrapping, and
    /// output capacity in order to return precise [`DecodeError`] diagnostics,
    /// including exact invalid-byte values and positions.
    /// Do not use this method for token comparison, key-material decoding, or
    /// secret-bearing validation where malformed-input timing matters. Use
    /// [`DecodeError::kind`] instead of logging full strict errors when input
    /// may be secret-bearing or secret-adjacent. Use
    /// [`crate::ct`] with a matching unwrapped engine for constant-time-oriented
    /// secret decoding.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_slice_wrapped(input, output, wrap),
            None => self.engine.decode_slice(input, output),
        }
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        match self.wrap {
            Some(wrap) => self
                .engine
                .decode_slice_wrapped_clear_tail(input, output, wrap),
            None => self.engine.decode_slice_clear_tail(input, output),
        }
    }

    /// Decodes `input` into a stack-backed buffer according to this profile.
    ///
    /// This is useful for short decoded values where heap allocation is
    /// unnecessary. If decoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn decode_buffer<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `buffer` in place according to this profile.
    ///
    /// For wrapped profiles, configured line endings are compacted out before
    /// decoding. If validation fails, the buffer contents are unspecified.
    /// On success, bytes after the returned decoded prefix may retain compacted
    /// encoded input. Use [`Self::decode_in_place_clear_tail`] when the buffer
    /// may be reused or freed without a caller-managed wipe.
    ///
    /// # Security
    ///
    /// Profile in-place decoders use the normal strict decode path. They may
    /// branch or return early based on malformed input, padding position,
    /// wrapping, and output capacity in order to return precise
    /// [`DecodeError`] diagnostics. Do not use this method for token
    /// comparison, key-material decoding, or secret-bearing validation where
    /// malformed-input timing matters. Use [`DecodeError::kind`] instead of
    /// logging full strict errors when input may be secret-bearing or
    /// secret-adjacent.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, Profile, STANDARD};
    ///
    /// let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let decoded = profile.decode_in_place(&mut buffer).unwrap();
    ///
    /// assert_eq!(decoded, b"hello");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_in_place_wrapped(buffer, wrap),
            None => self.engine.decode_in_place(buffer),
        }
    }

    /// Decodes `buffer` in place according to this profile and clears all
    /// bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, Profile, STANDARD};
    ///
    /// let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let len = profile.decode_in_place_clear_tail(&mut buffer).unwrap().len();
    ///
    /// assert_eq!(&buffer[..len], b"hello");
    /// assert!(buffer[len..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_in_place_wrapped_clear_tail(buffer, wrap),
            None => self.engine.decode_in_place_clear_tail(buffer),
        }
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use encode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_wrapped_vec(input, wrap),
            None => self.engine.encode_vec(input),
        }
    }

    /// Encodes `input` into a newly allocated byte vector.
    ///
    /// This is a convenience wrapper around [`Self::encode_vec`] for ordinary
    /// byte-to-Base64 paths where encoding failure would indicate an internal
    /// length/allocation invariant failure rather than invalid input.
    ///
    /// Prefer [`Self::encode_vec`] when handling untrusted length metadata,
    /// constrained allocation environments, or code paths that must return a
    /// recoverable error instead of panicking.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::encode_vec`] returns an error.
    #[cfg(feature = "alloc")]
    #[must_use]
    pub fn encode_vec_infallible(&self, input: &[u8]) -> alloc::vec::Vec<u8> {
        self.encode_vec(input)
            .expect("base64-ng profile encode_vec failed for byte input")
    }

    /// Encodes `input` into a redacted owned secret buffer.
    #[cfg(feature = "alloc")]
    pub fn encode_secret(&self, input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        self.encode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        match self.wrap {
            Some(wrap) => self.engine.encode_wrapped_string(input, wrap),
            None => self.engine.encode_string(input),
        }
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    ///
    /// This is a convenience wrapper around [`Self::encode_string`] for
    /// ordinary byte-to-Base64 paths where encoding failure would indicate an
    /// internal length/allocation invariant failure rather than invalid input.
    ///
    /// Prefer [`Self::encode_string`] when handling untrusted length metadata,
    /// constrained allocation environments, or code paths that must return a
    /// recoverable error instead of panicking.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::encode_string`] returns an error.
    #[cfg(feature = "alloc")]
    #[must_use]
    pub fn encode_string_infallible(&self, input: &[u8]) -> alloc::string::String {
        self.encode_string(input)
            .expect("base64-ng profile encode_string failed for byte input")
    }

    /// Decodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        match self.wrap {
            Some(wrap) => self.engine.decode_wrapped_vec(input, wrap),
            None => self.engine.decode_vec(input),
        }
    }

    /// Decodes `input` into a redacted owned secret buffer.
    ///
    /// # Security
    ///
    /// This uses the profile's normal strict decoder, not the
    /// constant-time-oriented [`crate::ct`] module. It may branch or return
    /// early on malformed input and reports localized decode errors. For
    /// secret-bearing payloads where malformed-input timing matters, use the
    /// matching [`crate::ct::CtEngine`] explicitly and wrap successful output in
    /// [`SecretBuffer`].
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }
}

impl<A, const PAD: bool> Default for Profile<A, PAD>
where
    A: Alphabet,
{
    fn default() -> Self {
        Self::new(Engine::new(), None)
    }
}

impl<A, const PAD: bool> core::fmt::Display for Profile<A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.wrap {
            Some(wrap) => write!(formatter, "padded={PAD} wrap={wrap}"),
            None => write!(formatter, "padded={PAD} wrap=none"),
        }
    }
}

impl<A, const PAD: bool> From<Engine<A, PAD>> for Profile<A, PAD>
where
    A: Alphabet,
{
    fn from(engine: Engine<A, PAD>) -> Self {
        Self::new(engine, None)
    }
}

/// MIME Base64 profile: standard alphabet, padding, 76-column CRLF wrapping.
///
/// This profile uses the default strict decoder and is not a constant-time
/// token validator or key-material decoder. Use
/// [`ct::STANDARD`](crate::ct::STANDARD) with an application-level wrapping
/// policy for sensitive fixed-shape protocols.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const MIME: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::MIME));

/// PEM Base64 profile: standard alphabet, padding, 64-column LF wrapping.
///
/// This profile uses the default strict decoder and is not a constant-time
/// token validator or key-material decoder. Use
/// [`ct::STANDARD`](crate::ct::STANDARD) with an application-level wrapping
/// policy for sensitive fixed-shape protocols.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const PEM: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::PEM));

/// PEM Base64 profile with CRLF line endings.
///
/// This profile uses the default strict decoder and is not a constant-time
/// token validator or key-material decoder. Use
/// [`ct::STANDARD`](crate::ct::STANDARD) with an application-level wrapping
/// policy for sensitive fixed-shape protocols.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const PEM_CRLF: Profile<Standard, true> = Profile::new(STANDARD, Some(LineWrap::PEM_CRLF));

/// bcrypt-style no-padding Base64 profile.
///
/// This profile carries the bcrypt alphabet and no padding. It does not parse
/// complete bcrypt password-hash strings. Its default strict decoder is not a
/// constant-time token validator or key-material decoder; use
/// [`Profile::engine`] with [`Engine::ct_decoder`] for the matching
/// constant-time-oriented decoder when timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const BCRYPT: Profile<Bcrypt, false> = Profile::new(BCRYPT_NO_PAD, None);

/// Unix `crypt(3)`-style no-padding Base64 profile.
///
/// This profile carries the `crypt(3)` alphabet and no padding. It does not
/// parse complete password-hash strings. Its default strict decoder is not a
/// constant-time token validator or key-material decoder; use
/// [`Profile::engine`] with [`Engine::ct_decoder`] for the matching
/// constant-time-oriented decoder when timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const CRYPT: Profile<Crypt, false> = Profile::new(CRYPT_NO_PAD, None);
