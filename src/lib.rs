#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

//! `base64-ng` is a `no_std`-first Base64 encoder and decoder.
//!
//! This initial release provides strict scalar RFC 4648-style behavior and
//! caller-owned output buffers. Future SIMD fast paths, including AVX, NEON,
//! and wasm `simd128` candidates, will be required to match this scalar module
//! byte-for-byte.
//!
//! # Examples
//!
//! Encode and decode with caller-owned buffers:
//!
//! ```
//! use base64_ng::{STANDARD, checked_encoded_len};
//!
//! let input = b"hello";
//! const ENCODED_CAPACITY: usize = match checked_encoded_len(5, true) {
//!     Some(len) => len,
//!     None => panic!("encoded length overflow"),
//! };
//! let mut encoded = [0u8; ENCODED_CAPACITY];
//! let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"aGVsbG8=");
//!
//! let mut decoded = [0u8; 5];
//! let decoded_len = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
//! assert_eq!(&decoded[..decoded_len], input);
//! ```
//!
//! Use the URL-safe no-padding engine:
//!
//! ```
//! use base64_ng::URL_SAFE_NO_PAD;
//!
//! let mut encoded = [0u8; 3];
//! let encoded_len = URL_SAFE_NO_PAD.encode_slice(b"\xfb\xff", &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"-_8");
//! ```
//!
//! # Sensitive Decode Policy
//!
//! The default engines such as [`STANDARD`] and [`URL_SAFE_NO_PAD`] are strict
//! scalar encoders/decoders with localized diagnostics. They are not
//! constant-time token validators or key-material decoders: strict decode and
//! validation may branch or return early based on malformed input. Use
//! [`ct::STANDARD`], [`ct::URL_SAFE_NO_PAD`], or [`Engine::ct_decoder`] for
//! secret-bearing payloads where decode timing posture matters more than exact
//! error indexes.
//!
//! # Zeroization Caveat
//!
//! Cleanup APIs and redacted buffers use dependency-free best-effort wiping:
//! byte-wise volatile zero writes followed by an architecture-gated inline
//! assembly barrier plus a hardware store-ordering fence where stable Rust
//! supports it, and a compiler fence on all targets. This resists common
//! compiler dead-store elimination and orders the issued zero stores on native
//! supported architectures, but it is not a formal zeroization guarantee and
//! cannot clear historical copies, registers, cache lines, write buffers, swap,
//! hibernation images, core dumps, cold-boot remanence, or OS-level memory
//! snapshots.
//! High-assurance applications should apply their own approved zeroization
//! policy to caller-owned buffers at the protocol boundary. Architectures
//! without a native wipe barrier fail closed by default unless
//! `allow-compiler-fence-only-wipe` is enabled after platform review. On
//! `wasm32`, the wipe barrier is compiler-fence-only and cannot constrain
//! downstream wasm runtime JITs. For that reason, `wasm32` builds fail closed
//! by default. Enable `allow-wasm32-best-effort-wipe` only when the deployment
//! explicitly accepts compiler-fence-only cleanup and applies its own memory
//! strategy.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(target_arch = "wasm32", not(feature = "allow-wasm32-best-effort-wipe")))]
compile_error!(
    "base64-ng: wasm32 builds use a compiler-fence-only wipe barrier that cannot \
     constrain downstream wasm runtime JITs. Enable \
     `allow-wasm32-best-effort-wipe` to accept this limitation and use \
     caller-owned, platform-approved zeroization for high-assurance wasm deployments."
);

#[cfg(all(
    not(miri),
    not(feature = "allow-compiler-fence-only-wipe"),
    not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64",
    ))
))]
compile_error!(
    "base64-ng: this architecture has no native hardware wipe barrier in \
     base64-ng. Enable `allow-compiler-fence-only-wipe` only after reviewing \
     docs/UNSAFE.md and applying platform-approved memory hygiene controls."
);

mod alphabet;
mod buffers;
mod cleanup;
pub mod ct;
mod errors;
mod length;
mod profiles;
mod scalar;
mod wrap;

pub use alphabet::{
    Alphabet, AlphabetError, Bcrypt, Crypt, Standard, UrlSafe, decode_alphabet_byte,
    validate_alphabet,
};
pub(crate) use alphabet::{encode_base64_value, encode_base64_value_runtime};
pub use buffers::{DecodedBuffer, EncodedBuffer, ExposedDecodedArray, ExposedEncodedArray};
#[cfg(feature = "alloc")]
pub use buffers::{ExposedSecretString, ExposedSecretVec, SecretBuffer};
pub(crate) use cleanup::{wipe_bytes, wipe_tail};
#[cfg(feature = "alloc")]
pub(crate) use cleanup::{wipe_vec_all, wipe_vec_spare_capacity};
pub(crate) use ct::{
    constant_time_eq_fixed_width_array, constant_time_eq_public_len, ct_mask_eq_u8, ct_mask_lt_u8,
};
#[cfg(test)]
pub(crate) use ct::{ct_padded_final_quantum, report_ct_error};
pub use errors::{DecodeError, EncodeError};
pub use length::{
    LineEnding, LineWrap, checked_encoded_len, checked_wrapped_encoded_len, decoded_capacity,
    decoded_len, encoded_len, wrapped_encoded_len,
};
pub(crate) use length::{decoded_len_padded, decoded_len_unpadded};
pub use profiles::{BCRYPT, CRYPT, MIME, PEM, PEM_CRLF, Profile};
#[cfg(kani)]
pub(crate) use scalar::decode_byte;
pub(crate) use scalar::{
    decode_chunk, decode_tail_unpadded, read_quad, validate_chunk, validate_decode,
    validate_tail_unpadded,
};
pub(crate) use wrap::{
    compact_wrapped_input, decode_legacy_to_slice, decode_wrapped_to_slice, is_legacy_whitespace,
    validate_legacy_decode, validate_wrapped_decode, write_wrapped_byte, write_wrapped_bytes,
};

#[cfg(feature = "simd")]
mod simd;

/// Runtime backend reporting for security-sensitive deployments.
///
/// This module does not enable acceleration. It exposes the backend posture so
/// callers can log, assert, or audit whether execution is scalar-only or merely
/// detecting future SIMD candidates.
pub mod runtime;

#[cfg(feature = "stream")]
pub mod stream;

/// Standard Base64 engine with padding.
///
/// This default strict engine is not a constant-time token validator or
/// key-material decoder. Use [`ct::STANDARD`] or [`Engine::ct_decoder`] for the
/// matching constant-time-oriented decoder when timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const STANDARD: Engine<Standard, true> = Engine::new();

/// Standard Base64 engine without padding.
///
/// This default strict engine is not a constant-time token validator or
/// key-material decoder. Use [`ct::STANDARD_NO_PAD`] or [`Engine::ct_decoder`]
/// for the matching constant-time-oriented decoder when timing posture
/// matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const STANDARD_NO_PAD: Engine<Standard, false> = Engine::new();

/// URL-safe Base64 engine with padding.
///
/// This default strict engine is not a constant-time token validator or
/// key-material decoder. Use [`ct::URL_SAFE`] or [`Engine::ct_decoder`] for the
/// matching constant-time-oriented decoder when timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const URL_SAFE: Engine<UrlSafe, true> = Engine::new();

/// URL-safe Base64 engine without padding.
///
/// This default strict engine is not a constant-time token validator or
/// key-material decoder. Use [`ct::URL_SAFE_NO_PAD`] or [`Engine::ct_decoder`]
/// for the matching constant-time-oriented decoder when timing posture
/// matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const URL_SAFE_NO_PAD: Engine<UrlSafe, false> = Engine::new();

/// bcrypt-style Base64 engine without padding.
///
/// This uses the bcrypt alphabet with the crate's normal Base64 bit packing.
/// It does not parse complete bcrypt password-hash strings. This default strict
/// engine is not a constant-time token validator or key-material decoder; use
/// [`Engine::ct_decoder`] for the matching constant-time-oriented decoder when
/// timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const BCRYPT_NO_PAD: Engine<Bcrypt, false> = Engine::new();

/// Unix `crypt(3)`-style Base64 engine without padding.
///
/// This uses the `crypt(3)` alphabet with the crate's normal Base64 bit
/// packing. It does not parse complete password-hash strings. This default
/// strict engine is not a constant-time token validator or key-material
/// decoder; use [`Engine::ct_decoder`] for the matching constant-time-oriented
/// decoder when timing posture matters.
#[doc(alias = "ct")]
#[doc(alias = "constant_time")]
#[doc(alias = "sensitive")]
pub const CRYPT_NO_PAD: Engine<Crypt, false> = Engine::new();

/// Compares two fixed-width byte arrays without a length-mismatch branch.
///
/// Use this helper when the value length itself should not be represented as a
/// timing-distinct branch in the comparison API. The array length `N` is a
/// compile-time public type fact, and the helper scans exactly `N` bytes before
/// returning. The final equality result remains public. This is still a
/// dependency-free, constant-time-oriented best-effort helper, not a formally
/// verified cryptographic comparison primitive.
///
/// # Examples
///
/// ```
/// use base64_ng::constant_time_eq_fixed_width;
///
/// assert!(constant_time_eq_fixed_width(b"token", b"token"));
/// assert!(!constant_time_eq_fixed_width(b"token", b"Token"));
/// ```
#[must_use]
pub fn constant_time_eq_fixed_width<const N: usize>(left: &[u8; N], right: &[u8; N]) -> bool {
    constant_time_eq_fixed_width_array(left, right)
}

/// A zero-sized Base64 engine parameterized by alphabet and padding policy.
pub struct Engine<A, const PAD: bool> {
    alphabet: core::marker::PhantomData<A>,
}

impl<A, const PAD: bool> Clone for Engine<A, PAD> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const PAD: bool> Copy for Engine<A, PAD> {}

impl<A, const PAD: bool> core::fmt::Debug for Engine<A, PAD> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("padded", &PAD)
            .finish()
    }
}

impl<A, const PAD: bool> core::fmt::Display for Engine<A, PAD> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "padded={PAD}")
    }
}

impl<A, const PAD: bool> Default for Engine<A, PAD> {
    fn default() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }
}

impl<A, const PAD: bool> Eq for Engine<A, PAD> {}

impl<A, const PAD: bool> PartialEq for Engine<A, PAD> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Creates a new engine value.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }

    /// Returns whether this engine uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns this engine as an unwrapped profile.
    ///
    /// Use [`Profile::new`] or [`Profile::checked_new`] when a strict
    /// line-wrapping policy should travel with the profile.
    #[must_use]
    pub const fn profile(&self) -> Profile<A, PAD> {
        Profile::new(*self, None)
    }

    /// Returns the matching constant-time-oriented decoder for this engine's
    /// alphabet and padding policy.
    ///
    /// The returned decoder is still an explicit opt-in to the [`ct`] module's
    /// slower, opaque-error, constant-time-oriented scalar path.
    #[must_use]
    pub const fn ct_decoder(&self) -> ct::CtEngine<A, PAD> {
        ct::CtEngine::new()
    }

    /// Wraps a `std::io::Write` value in a streaming Base64 encoder.
    ///
    /// This is a convenience constructor for [`stream::Encoder::new`] that
    /// keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Write;
    /// use base64_ng::STANDARD;
    ///
    /// let mut encoder = STANDARD.encoder_writer(Vec::new());
    /// encoder.write_all(b"hello").unwrap();
    /// assert_eq!(encoder.finish().unwrap(), b"aGVsbG8=");
    /// ```
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn encoder_writer<W>(&self, inner: W) -> stream::Encoder<W, A, PAD> {
        stream::Encoder::new(inner, *self)
    }

    /// Wraps a `std::io::Write` value in a streaming Base64 decoder.
    ///
    /// This is a convenience constructor for [`stream::Decoder::new`] that
    /// keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Write;
    /// use base64_ng::STANDARD;
    ///
    /// let mut decoder = STANDARD.decoder_writer(Vec::new());
    /// decoder.write_all(b"aGVsbG8=").unwrap();
    /// assert_eq!(decoder.finish().unwrap(), b"hello");
    /// ```
    ///
    /// # Security
    ///
    /// Streaming decoders use the normal strict decode path, not the
    /// [`crate::ct`] module. Do not use this adapter for secret-bearing
    /// payloads when malformed-input timing matters.
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn decoder_writer<W>(&self, inner: W) -> stream::Decoder<W, A, PAD> {
        stream::Decoder::new(inner, *self)
    }

    /// Wraps a `std::io::Read` value in a streaming Base64 encoder.
    ///
    /// This is a convenience constructor for [`stream::EncoderReader::new`]
    /// that keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Read;
    /// use base64_ng::STANDARD;
    ///
    /// let mut reader = STANDARD.encoder_reader(&b"hello"[..]);
    /// let mut encoded = String::new();
    /// reader.read_to_string(&mut encoded).unwrap();
    /// assert_eq!(encoded, "aGVsbG8=");
    /// ```
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn encoder_reader<R>(&self, inner: R) -> stream::EncoderReader<R, A, PAD> {
        stream::EncoderReader::new(inner, *self)
    }

    /// Wraps a `std::io::Read` value in a streaming Base64 decoder.
    ///
    /// This is a convenience constructor for [`stream::DecoderReader::new`]
    /// that keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Read;
    /// use base64_ng::STANDARD;
    ///
    /// let mut reader = STANDARD.decoder_reader(&b"aGVsbG8="[..]);
    /// let mut decoded = Vec::new();
    /// reader.read_to_end(&mut decoded).unwrap();
    /// assert_eq!(decoded, b"hello");
    /// ```
    ///
    /// # Security
    ///
    /// Streaming decoder readers use the normal strict decode path, not the
    /// [`crate::ct`] module. Do not use this adapter for secret-bearing
    /// payloads when malformed-input timing matters.
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn decoder_reader<R>(&self, inner: R) -> stream::DecoderReader<R, A, PAD> {
        stream::DecoderReader::new(inner, *self)
    }

    /// Returns the encoded length for this engine's padding policy.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        encoded_len(input_len, PAD)
    }

    /// Returns the encoded length for this engine, or `None` on overflow.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        checked_encoded_len(input_len, PAD)
    }

    /// Returns the encoded length after applying a line wrapping policy.
    ///
    /// The returned length includes inserted line endings but does not include
    /// a trailing line ending after the final encoded line.
    pub const fn wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the encoded length after line wrapping, or `None` on overflow or
    /// invalid line wrapping.
    #[must_use]
    pub const fn checked_wrapped_encoded_len(
        &self,
        input_len: usize,
        wrap: LineWrap,
    ) -> Option<usize> {
        checked_wrapped_encoded_len(input_len, PAD, wrap)
    }

    /// Returns the exact decoded length implied by input length and padding.
    ///
    /// This validates padding placement and impossible lengths, but it does not
    /// validate alphabet membership or non-canonical trailing bits.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        decoded_len(input, PAD)
    }

    /// Returns the exact decoded length for the explicit legacy profile.
    ///
    /// The legacy profile ignores ASCII space, tab, carriage return, and line
    /// feed bytes before applying the same alphabet, padding, and canonical-bit
    /// checks as strict decoding.
    pub fn decoded_len_legacy(&self, input: &[u8]) -> Result<usize, DecodeError> {
        validate_legacy_decode::<A, PAD>(input)
    }

    /// Returns the exact decoded length for a line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    pub fn decoded_len_wrapped(&self, input: &[u8], wrap: LineWrap) -> Result<usize, DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap)
    }

    /// Validates strict Base64 input without writing decoded bytes.
    ///
    /// This applies the same alphabet, padding, and canonical-bit checks as
    /// [`Self::decode_slice`]. Use this method when malformed-input
    /// diagnostics matter; use [`Self::validate`] when a boolean is enough.
    /// This default validator is not constant-time; use
    /// [`crate::ct::CtEngine::validate_result`] through [`Self::ct_decoder`]
    /// for secret-bearing payloads where timing posture matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_result(b"aGVsbG8=").unwrap();
    /// assert!(STANDARD.validate_result(b"aGVsbG8").is_err());
    /// ```
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid strict Base64 for this engine.
    ///
    /// This is a convenience wrapper around [`Self::validate_result`] and is
    /// not constant-time. Use [`crate::ct::CtEngine::validate`] through
    /// [`Self::ct_decoder`] for secret-bearing payloads where timing posture
    /// matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::URL_SAFE_NO_PAD;
    ///
    /// assert!(URL_SAFE_NO_PAD.validate(b"-_8"));
    /// assert!(!URL_SAFE_NO_PAD.validate(b"+/8"));
    /// ```
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Validates input using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored
    /// before applying the same alphabet, padding, and canonical-bit checks as
    /// strict decoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// STANDARD.validate_legacy_result(b" aG\r\nVsbG8= ").unwrap();
    /// assert!(STANDARD.validate_legacy_result(b" aG-=").is_err());
    /// ```
    pub fn validate_legacy_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        validate_legacy_decode::<A, PAD>(input).map(|_| ())
    }

    /// Returns whether `input` is valid for the explicit legacy whitespace
    /// profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_legacy_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// assert!(STANDARD.validate_legacy(b" aG\r\nVsbG8= "));
    /// assert!(!STANDARD.validate_legacy(b"aG-V"));
    /// ```
    #[must_use]
    pub fn validate_legacy(&self, input: &[u8]) -> bool {
        self.validate_legacy_result(input).is_ok()
    }

    /// Validates input using a strict line-wrapped profile.
    ///
    /// This is stricter than [`Self::validate_legacy_result`]: it accepts only
    /// the configured line ending and enforces the configured line length for
    /// every non-final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// STANDARD.validate_wrapped_result(b"aGVs\nbG8=", wrap).unwrap();
    /// assert!(STANDARD.validate_wrapped_result(b"aG\nVsbG8=", wrap).is_err());
    /// ```
    pub fn validate_wrapped_result(&self, input: &[u8], wrap: LineWrap) -> Result<(), DecodeError> {
        validate_wrapped_decode::<A, PAD>(input, wrap).map(|_| ())
    }

    /// Returns whether `input` is valid for a strict line-wrapped profile.
    ///
    /// This is a convenience wrapper around [`Self::validate_wrapped_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// assert!(STANDARD.validate_wrapped(b"aGVs\nbG8=", wrap));
    /// assert!(!STANDARD.validate_wrapped(b"aG\nVsbG8=", wrap));
    /// ```
    #[must_use]
    pub fn validate_wrapped(&self, input: &[u8], wrap: LineWrap) -> bool {
        self.validate_wrapped_result(input, wrap).is_ok()
    }

    /// Encodes a fixed-size input into a fixed-size output array in const contexts.
    ///
    /// Stable Rust does not yet allow this API to return an array whose length
    /// is computed from `INPUT_LEN` directly. Instead, the caller supplies the
    /// output length through the destination type and this function panics
    /// during const evaluation if the length is wrong.
    ///
    /// # Panics
    ///
    /// Panics if `OUTPUT_LEN` is not exactly the encoded length for `INPUT_LEN`
    /// and this engine's padding policy, or if that length overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// const HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    /// const URL_SAFE: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    ///
    /// assert_eq!(&HELLO, b"aGVsbG8=");
    /// assert_eq!(&URL_SAFE, b"-_8");
    /// ```
    ///
    /// Incorrect output lengths fail during const evaluation:
    ///
    /// ```compile_fail
    /// use base64_ng::STANDARD;
    ///
    /// const TOO_SHORT: [u8; 7] = STANDARD.encode_array(b"hello");
    /// ```
    #[must_use]
    pub const fn encode_array<const INPUT_LEN: usize, const OUTPUT_LEN: usize>(
        &self,
        input: &[u8; INPUT_LEN],
    ) -> [u8; OUTPUT_LEN] {
        let Some(required) = checked_encoded_len(INPUT_LEN, PAD) else {
            panic!("encoded base64 length overflows usize");
        };
        assert!(
            required == OUTPUT_LEN,
            "base64 output array has incorrect length"
        );

        let mut output = [0u8; OUTPUT_LEN];
        let mut read = 0;
        let mut write = 0;
        while INPUT_LEN - read >= 3 {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = encode_base64_value::<A>(b0 >> 2);
            output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

            read += 3;
            write += 4;
        }

        match INPUT_LEN - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                if PAD {
                    output[write + 3] = b'=';
                }
            }
            _ => unreachable!(),
        }

        output
    }

    /// Encodes `input` into `output`, returning the number of bytes written.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        scalar::encode_slice::<A, PAD>(input, output)
    }

    /// Encodes `input` into `output` with line wrapping.
    ///
    /// The wrapping policy inserts line endings between encoded lines and does
    /// not append a trailing line ending after the final line.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let wrap = LineWrap::new(4, LineEnding::Lf);
    /// let mut output = [0u8; 9];
    /// let written = STANDARD
    ///     .encode_slice_wrapped(b"hello", &mut output, wrap)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVs\nbG8=");
    /// ```
    pub fn encode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let encoded_len =
            checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if encoded_len == 0 {
            return Ok(0);
        }

        // If the temporary in-buffer layout size overflows, fall back to the
        // fixed scratch buffer path rather than relying on saturated arithmetic.
        let combined_required = match required.checked_add(encoded_len) {
            Some(len) => len,
            None => usize::MAX,
        };
        if output.len() < combined_required {
            let mut scratch = [0u8; 1024];
            let mut input_offset = 0;
            let mut output_offset = 0;
            let mut column = 0;

            while input_offset < input.len() {
                let remaining = input.len() - input_offset;
                let mut take = remaining.min(768);
                if remaining > take {
                    take -= take % 3;
                }
                if take == 0 {
                    take = remaining;
                }

                let encoded = match self
                    .encode_slice(&input[input_offset..input_offset + take], &mut scratch)
                {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        wipe_bytes(&mut scratch);
                        return Err(err);
                    }
                };
                if let Err(err) = write_wrapped_bytes(
                    &scratch[..encoded],
                    output,
                    &mut output_offset,
                    &mut column,
                    wrap,
                ) {
                    wipe_bytes(&mut scratch);
                    return Err(err);
                }
                wipe_bytes(&mut scratch[..encoded]);
                input_offset += take;
            }

            Ok(output_offset)
        } else {
            let encoded =
                self.encode_slice(input, &mut output[required..required + encoded_len])?;
            let mut output_offset = 0;
            let mut column = 0;
            let mut read = required;
            while read < required + encoded {
                let byte = output[read];
                write_wrapped_byte(byte, output, &mut output_offset, &mut column, wrap)?;
                read += 1;
            }
            wipe_bytes(&mut output[required..required + encoded]);
            Ok(output_offset)
        }
    }

    /// Encodes `input` with line wrapping and clears all bytes after the
    /// encoded prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    pub fn encode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` with line wrapping into a stack-backed buffer.
    ///
    /// This is useful for MIME/PEM-style protocols where heap allocation is
    /// unnecessary. If encoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn encode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<EncodedBuffer<CAP>, EncodeError> {
        let mut output = EncodedBuffer::new();
        let written =
            match self.encode_slice_wrapped_clear_tail(input, output.as_mut_capacity(), wrap) {
                Ok(written) => written,
                Err(err) => {
                    output.clear();
                    return Err(err);
                }
            };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use encode_wrapped_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn encode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = self.wrapped_encoded_len(input.len(), wrap)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice_wrapped(input, &mut output, wrap)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` with line wrapping into a newly allocated UTF-8 string.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_string(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_wrapped_vec(input, wrap)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes `input` with line wrapping into a redacted owned secret buffer.
    ///
    /// This is useful when the wrapped encoded representation itself is
    /// sensitive and should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, EncodeError> {
        self.encode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 12];
    /// let written = STANDARD
    ///     .encode_slice_clear_tail(b"hello", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVsbG8=");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Encodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let encoded = STANDARD.encode_buffer::<8>(b"hello").unwrap();
    ///
    /// assert_eq!(encoded.as_str(), "aGVsbG8=");
    /// ```
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

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use encode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` into a redacted owned secret buffer.
    ///
    /// This is useful when the encoded representation itself is sensitive and
    /// should not be accidentally logged through formatting.
    #[cfg(feature = "alloc")]
    pub fn encode_secret(&self, input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        self.encode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    ///
    /// Base64 output is ASCII by construction. This helper is available with
    /// the `alloc` feature and has the same encoding semantics as
    /// [`Self::encode_slice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// assert_eq!(STANDARD.encode_string(b"hello").unwrap(), "aGVsbG8=");
    /// assert_eq!(URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap(), "-_8");
    /// ```
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_vec(input)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0u8; 8];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        if input_len > buffer.len() {
            return Err(EncodeError::InputTooLarge {
                input_len,
                buffer_len: buffer.len(),
            });
        }

        let required = checked_encoded_len(input_len, PAD).ok_or(EncodeError::LengthOverflow)?;
        if buffer.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: buffer.len(),
            });
        }

        let mut read = input_len;
        let mut write = required;

        match input_len % 3 {
            0 => {}
            1 => {
                read -= 1;
                let b0 = buffer[read];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                    buffer[write + 2] = b'=';
                    buffer[write + 3] = b'=';
                } else {
                    write -= 2;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value_runtime::<A>((b0 & 0b0000_0011) << 4);
                }
            }
            2 => {
                read -= 2;
                let b0 = buffer[read];
                let b1 = buffer[read + 1];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                    buffer[write + 3] = b'=';
                } else {
                    write -= 3;
                    buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value_runtime::<A>((b1 & 0b0000_1111) << 2);
                }
            }
            _ => unreachable!(),
        }

        while read > 0 {
            read -= 3;
            write -= 4;
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let b2 = buffer[read + 2];

            buffer[write] = encode_base64_value_runtime::<A>(b0 >> 2);
            buffer[write + 1] =
                encode_base64_value_runtime::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            buffer[write + 2] =
                encode_base64_value_runtime::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            buffer[write + 3] = encode_base64_value_runtime::<A>(b2 & 0b0011_1111);
        }

        // The right-to-left loop consumes exactly three input bytes for every
        // four output bytes. If this invariant changes, returning a shifted
        // slice would silently corrupt the in-place output.
        debug_assert_eq!(write, 0);
        Ok(&mut buffer[..required])
    }

    /// Encodes the first `input_len` bytes of `buffer` in place and clears all
    /// bytes after the encoded prefix.
    ///
    /// If encoding fails because `input_len` is too large, the output buffer is
    /// too small, or the encoded length overflows `usize`, the entire buffer is
    /// cleared before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0xff; 12];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place_clear_tail(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        let len = match self.encode_in_place(buffer, input_len) {
            Ok(encoded) => encoded.len(),
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes `input` into `output`, returning the number of bytes written.
    ///
    /// This is strict decoding. Whitespace, mixed alphabets, malformed padding,
    /// and trailing non-padding data are rejected.
    ///
    /// # Security
    ///
    /// This default scalar decoder prioritizes strict validation, exact error
    /// reporting, and ordinary throughput. It may branch or return early based
    /// on byte validity, malformed input, padding position, and output
    /// capacity. It also reports exact failure positions and invalid byte
    /// values through [`DecodeError`]. Do not use this method for token
    /// comparison, key-material decoding, or secret-bearing validation where
    /// malformed-input timing matters. Use [`crate::ct`],
    /// [`crate::ct::STANDARD`], [`crate::ct::URL_SAFE_NO_PAD`], or
    /// [`Self::ct_decoder`] with `decode_slice_clear_tail` for
    /// constant-time-oriented secret decoding.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        scalar::decode_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    ///
    /// If decoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_clear_tail(b"aGk=", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer.
    ///
    /// This helper is useful for short decoded values where callers want the
    /// convenience of an owned result without enabling `alloc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let decoded = STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
    ///
    /// assert_eq!(decoded.as_bytes(), b"hello");
    /// ```
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

    /// Decodes `input` using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    ///
    /// # Security
    ///
    /// This method uses the normal strict decode path after legacy whitespace
    /// handling. It may branch or return early based on malformed input and is
    /// not a constant-time token validator or key-material decoder. Use
    /// [`crate::ct`] for secret-bearing payloads.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice_legacy(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_legacy_to_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_legacy_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_legacy(input, output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` into a stack-backed buffer using the explicit legacy
    /// whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict. If decoding fails, the
    /// internal backing array is cleared before the error is returned.
    pub fn decode_buffer_legacy<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written = match self.decode_slice_legacy_clear_tail(input, output.as_mut_capacity()) {
            Ok(written) => written,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    ///
    /// # Security
    ///
    /// This method uses the normal strict decode path after line-profile
    /// validation. It may branch or return early based on malformed input and
    /// is not a constant-time token validator or key-material decoder. Use
    /// [`crate::ct`] for secret-bearing payloads.
    #[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
    pub fn decode_slice_wrapped(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_wrapped_to_slice::<A, PAD>(input, output, wrap)
    }

    /// Decodes `input` using a strict line-wrapped profile and clears all bytes
    /// after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    pub fn decode_slice_wrapped_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        wrap: LineWrap,
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_wrapped(input, output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(output);
                return Err(err);
            }
        };
        wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes `input` using a strict line-wrapped profile into a stack-backed
    /// buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted. If decoding fails, the internal backing array is cleared
    /// before the error is returned.
    pub fn decode_wrapped_buffer<const CAP: usize>(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<DecodedBuffer<CAP>, DecodeError> {
        let mut output = DecodedBuffer::new();
        let written =
            match self.decode_slice_wrapped_clear_tail(input, output.as_mut_capacity(), wrap) {
                Ok(written) => written,
                Err(err) => {
                    output.clear();
                    return Err(err);
                }
            };
        output.set_filled(written)?;
        Ok(output)
    }

    /// Decodes `input` into a newly allocated byte vector.
    ///
    /// This is strict decoding with the same semantics as [`Self::decode_slice`].
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer.
    ///
    /// On malformed input, the intermediate output buffer is cleared before the
    /// error is returned by [`Self::decode_vec`].
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Decodes `input` into a newly allocated byte vector using the explicit
    /// legacy whitespace profile.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret_legacy, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec_legacy(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_legacy(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer using the explicit
    /// legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    #[cfg(feature = "alloc")]
    pub fn decode_secret_legacy(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec_legacy(input).map(SecretBuffer::from_vec)
    }

    /// Decodes line-wrapped input into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_wrapped_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_wrapped_vec(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_wrapped_decode::<A, PAD>(input, wrap)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_wrapped(input, &mut output, wrap) {
            Ok(written) => written,
            Err(err) => {
                wipe_bytes(&mut output);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes line-wrapped input into a redacted owned secret buffer.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    #[cfg(feature = "alloc")]
    pub fn decode_wrapped_secret(
        &self,
        input: &[u8],
        wrap: LineWrap,
    ) -> Result<SecretBuffer, DecodeError> {
        self.decode_wrapped_vec(input, wrap)
            .map(SecretBuffer::from_vec)
    }

    /// Decodes `buffer` in place using a strict line-wrapped profile.
    ///
    /// The wrapped profile accepts only the configured line ending. Non-final
    /// lines must contain exactly `wrap.line_len` encoded bytes; the final line
    /// may be shorter. A single trailing line ending after the final line is
    /// accepted.
    ///
    /// # Security
    ///
    /// This method compacts line endings in place before decoding. If
    /// validation or decoding fails, the buffer contents are unspecified and
    /// may contain a compacted encoded prefix. On success, bytes after the
    /// returned decoded prefix may retain the compacted encoded representation.
    /// Use
    /// [`Self::decode_in_place_wrapped_clear_tail`] when the buffer may be
    /// reused or freed without a caller-managed wipe; treat that clear-tail
    /// variant as the default for secret-bearing wrapped payloads.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let decoded = STANDARD
    ///     .decode_in_place_wrapped(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap();
    ///
    /// assert_eq!(decoded, b"hello");
    /// ```
    pub fn decode_in_place_wrapped<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_wrapped_decode::<A, PAD>(buffer, wrap)?;
        let compacted = compact_wrapped_input(buffer, wrap)?;
        let len = Self::decode_slice_to_start(&mut buffer[..compacted])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using a strict line-wrapped profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{LineEnding, LineWrap, STANDARD};
    ///
    /// let mut buffer = *b"aGVs\nbG8=";
    /// let len = STANDARD
    ///     .decode_in_place_wrapped_clear_tail(&mut buffer, LineWrap::new(4, LineEnding::Lf))
    ///     .unwrap()
    ///     .len();
    ///
    /// assert_eq!(&buffer[..len], b"hello");
    /// assert!(buffer[len..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_in_place_wrapped_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        wrap: LineWrap,
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_wrapped_decode::<A, PAD>(buffer, wrap) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let compacted = match compact_wrapped_input(buffer, wrap) {
            Ok(compacted) => compacted,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };

        let len = match Self::decode_slice_to_start(&mut buffer[..compacted]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and returns the decoded prefix.
    ///
    /// On success, bytes after the returned decoded prefix may retain encoded
    /// input bytes. Use [`Self::decode_in_place_clear_tail`] when the buffer
    /// may be reused or freed without a caller-managed wipe.
    ///
    /// # Security
    ///
    /// This default scalar decoder prioritizes strict validation, exact error
    /// reporting, and ordinary throughput. It may branch or return early based
    /// on malformed input and reports exact failure positions and invalid byte
    /// values through [`DecodeError`]. Do not use this method for token
    /// comparison, key-material decoding, or secret-bearing validation where
    /// malformed-input timing matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD_NO_PAD;
    ///
    /// let mut buffer = *b"Zm9vYmFy";
    /// let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"foobar");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        let len = Self::decode_slice_to_start(buffer)?;
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and clears all bytes after the decoded prefix.
    ///
    /// If decoding fails, the entire buffer is cleared before the error is
    /// returned. Use this variant when the encoded or partially decoded data is
    /// sensitive and the caller wants best-effort cleanup without adding a
    /// dependency.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = *b"aGk=";
    /// let decoded = STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"hi");
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let len = match Self::decode_slice_to_start(buffer) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile.
    ///
    /// Ignored whitespace is compacted out before decoding. If validation
    /// fails, the buffer contents are unspecified. On success, bytes after the
    /// returned decoded prefix may retain the compacted encoded
    /// representation. Use [`Self::decode_in_place_legacy_clear_tail`] when the
    /// buffer may be reused or freed without a caller-managed wipe.
    pub fn decode_in_place_legacy<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_legacy_decode::<A, PAD>(buffer)?;
        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }
        let len = Self::decode_slice_to_start(&mut buffer[..write])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile
    /// and clears all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    pub fn decode_in_place_legacy_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_legacy_decode::<A, PAD>(buffer) {
            wipe_bytes(buffer);
            return Err(err);
        }

        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }

        let len = match Self::decode_slice_to_start(&mut buffer[..write]) {
            Ok(len) => len,
            Err(err) => {
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }

    fn decode_slice_to_start(buffer: &mut [u8]) -> Result<usize, DecodeError> {
        let _required = validate_decode::<A, PAD>(buffer)?;
        let input_len = buffer.len();
        let mut read = 0;
        let mut write = 0;
        while read + 4 <= input_len {
            let chunk = read_quad(buffer, read)?;
            let available = buffer.len();
            let output_tail = buffer.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| err.with_index_offset(read))?;
            read += 4;
            write += written;
            if written < 3 {
                if read != input_len {
                    return Err(DecodeError::InvalidPadding { index: read - 4 });
                }
                return Ok(write);
            }
        }

        let rem = input_len - read;
        if rem == 0 {
            return Ok(write);
        }
        if PAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut tail = [0u8; 3];
        tail[..rem].copy_from_slice(&buffer[read..input_len]);
        decode_tail_unpadded::<A>(&tail[..rem], &mut buffer[write..])
            .map_err(|err| err.with_index_offset(read))
            .map(|n| write + n)
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{
        STANDARD, Standard, checked_encoded_len, ct, decode_byte, decode_chunk,
        decode_tail_unpadded, decoded_capacity, validate_tail_unpadded,
    };

    #[kani::proof]
    fn checked_encoded_len_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let padded = kani::any::<bool>();
        let encoded = checked_encoded_len(len, padded).expect("u8 input length cannot overflow");

        assert!(encoded >= len);
        assert!(encoded <= len / 3 * 4 + 4);
    }

    #[kani::proof]
    fn decoded_capacity_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let capacity = decoded_capacity(len);

        assert!(capacity <= len / 4 * 3 + 2);
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_in_place_decode_returns_prefix_within_buffer() {
        let mut buffer = kani::any::<[u8; 8]>();
        let result = STANDARD.decode_in_place(&mut buffer);

        if let Ok(decoded) = result {
            assert!(decoded.len() <= 8);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = STANDARD.decode_slice(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_chunk_returns_written_within_output() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = decode_chunk::<Standard, true>(input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
            assert!(written <= 3);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_chunk_bit_packing_matches_decoded_values() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = decode_chunk::<Standard, true>(input, &mut output);

        if let Ok(written) = result {
            let v0 = decode_byte::<Standard>(input[0], 0).expect("successful chunk has v0");
            let v1 = decode_byte::<Standard>(input[1], 1).expect("successful chunk has v1");

            assert!(output[0] == ((v0 << 2) | (v1 >> 4)));

            if written >= 2 {
                let v2 = decode_byte::<Standard>(input[2], 2).expect("successful chunk has v2");
                assert!(output[1] == ((v1 << 4) | (v2 >> 2)));
            }

            if written == 3 {
                let v2 = decode_byte::<Standard>(input[2], 2).expect("successful chunk has v2");
                let v3 = decode_byte::<Standard>(input[3], 3).expect("successful chunk has v3");
                assert!(output[2] == ((v2 << 6) | v3));
            }
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_validate_tail_unpadded_accepts_or_rejects_without_panic() {
        let input = kani::any::<[u8; 3]>();
        let len = usize::from(kani::any::<u8>() % 4);
        let result = validate_tail_unpadded::<Standard>(&input[..len]);

        if result.is_ok() {
            assert!(len == 0 || len == 2 || len == 3);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_two_byte_tail_returns_written_within_output() {
        let input = kani::any::<[u8; 2]>();
        let mut output = kani::any::<[u8; 1]>();
        let result = decode_tail_unpadded::<Standard>(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
            assert!(written == 1);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_three_byte_tail_returns_written_within_output() {
        let input = kani::any::<[u8; 3]>();
        let mut output = kani::any::<[u8; 2]>();
        let result = decode_tail_unpadded::<Standard>(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
            assert!(written == 2);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_decode_slice_clear_tail_clears_output_on_error() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = STANDARD.decode_slice_clear_tail(&input, &mut output);

        if result.is_err() {
            assert!(output.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_encode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 3]>();
        let mut output = kani::any::<[u8; 4]>();
        let result = STANDARD.encode_slice(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(4)]
    fn standard_encode_in_place_returns_prefix_within_buffer() {
        let mut buffer = kani::any::<[u8; 8]>();
        let input_len = usize::from(kani::any::<u8>() % 9);
        let result = STANDARD.encode_in_place(&mut buffer, input_len);

        if let Ok(encoded) = result {
            assert!(encoded.len() <= 8);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_clear_tail_decode_clears_buffer_on_error() {
        let mut buffer = kani::any::<[u8; 4]>();
        let result = STANDARD.decode_in_place_clear_tail(&mut buffer);

        if result.is_err() {
            assert!(buffer.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_slice_returns_written_within_output() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = ct::STANDARD.decode_slice_clear_tail(&input, &mut output);

        if let Ok(written) = result {
            assert!(written <= output.len());
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_slice_clear_tail_clears_output_on_error() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();
        let result = ct::STANDARD.decode_slice_clear_tail(&input, &mut output);

        if result.is_err() {
            assert!(output.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_decode_in_place_clear_tail_clears_buffer_on_error() {
        let mut buffer = kani::any::<[u8; 4]>();
        let result = ct::STANDARD.decode_in_place_clear_tail(&mut buffer);

        if result.is_err() {
            assert!(buffer.iter().all(|byte| *byte == 0));
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn ct_standard_validate_matches_decode_for_one_quantum() {
        let input = kani::any::<[u8; 4]>();
        let mut output = kani::any::<[u8; 3]>();

        let validate_ok = ct::STANDARD.validate_result(&input).is_ok();
        let decode_ok = ct::STANDARD
            .decode_slice_clear_tail(&input, &mut output)
            .is_ok();

        assert!(validate_ok == decode_ok);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_pattern(output: &mut [u8], seed: usize) {
        for (index, byte) in output.iter_mut().enumerate() {
            let value = (index * 73 + seed * 19) % 256;
            *byte = u8::try_from(value).unwrap();
        }
    }

    fn assert_encode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 256];
        let mut scalar = [0xaa; 256];

        let dispatched_result = engine.encode_slice(input, &mut dispatched);
        let scalar_result = scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);
        }

        let required = checked_encoded_len(input.len(), PAD).unwrap();
        if required > 0 {
            let mut dispatched_short = [0x55; 256];
            let mut scalar_short = [0xaa; 256];
            let available = required - 1;

            assert_eq!(
                engine.encode_slice(input, &mut dispatched_short[..available]),
                scalar::scalar_reference_encode_slice::<A, PAD>(
                    input,
                    &mut scalar_short[..available],
                )
            );
        }
    }

    fn assert_decode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 128];
        let mut scalar = [0xaa; 128];

        let dispatched_result = engine.decode_slice(input, &mut dispatched);
        let scalar_result = scalar::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);

            if written > 0 {
                let mut dispatched_short = [0x55; 128];
                let mut scalar_short = [0xaa; 128];
                let available = written - 1;

                assert_eq!(
                    engine.decode_slice(input, &mut dispatched_short[..available]),
                    scalar::scalar_reference_decode_slice::<A, PAD>(
                        input,
                        &mut scalar_short[..available],
                    )
                );
            }
        }
    }

    fn assert_backend_round_trip_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        assert_encode_backend_matches_scalar::<A, PAD>(input);

        let mut encoded = [0; 256];
        let encoded_len =
            scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut encoded).unwrap();
        assert_decode_backend_matches_scalar::<A, PAD>(&encoded[..encoded_len]);
    }

    fn assert_standard_decode_chunk_matches_input(input: &[u8]) {
        let mut encoded = [0u8; 4];
        let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
        assert_eq!(encoded_len, 4);

        let chunk = [encoded[0], encoded[1], encoded[2], encoded[3]];
        let mut decoded = [0u8; 3];
        let decoded_len = decode_chunk::<Standard, true>(chunk, &mut decoded).unwrap();

        assert_eq!(decoded_len, input.len());
        assert_eq!(&decoded[..decoded_len], input);
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_canonical_inputs() {
        let mut input = [0; 128];

        for input_len in 0..=input.len() {
            fill_pattern(&mut input[..input_len], input_len);
            let input = &input[..input_len];

            assert_backend_round_trip_matches_scalar::<Standard, true>(input);
            assert_backend_round_trip_matches_scalar::<Standard, false>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, true>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, false>(input);
        }
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_malformed_inputs() {
        for input in [
            &b"Z"[..],
            b"====",
            b"AA=A",
            b"Zh==",
            b"Zm9=",
            b"Zm9v$g==",
            b"Zm9vZh==",
        ] {
            assert_decode_backend_matches_scalar::<Standard, true>(input);
        }

        for input in [&b"Z"[..], b"AA=A", b"Zh", b"Zm9", b"Zm9vYg$"] {
            assert_decode_backend_matches_scalar::<Standard, false>(input);
        }

        assert_decode_backend_matches_scalar::<UrlSafe, true>(b"AA+A");
        assert_decode_backend_matches_scalar::<UrlSafe, false>(b"AA/A");
        assert_decode_backend_matches_scalar::<Standard, true>(b"AA-A");
        assert_decode_backend_matches_scalar::<Standard, false>(b"AA_A");
    }

    #[test]
    pub(crate) fn decode_chunk_bit_packing_matches_exhaustive_small_inputs() {
        for byte in u8::MIN..=u8::MAX {
            assert_standard_decode_chunk_matches_input(&[byte]);
        }

        for first in u8::MIN..=u8::MAX {
            for second in u8::MIN..=u8::MAX {
                assert_standard_decode_chunk_matches_input(&[first, second]);
            }
        }
    }

    #[test]
    pub(crate) fn decode_chunk_bit_packing_matches_representative_full_quanta() {
        const SAMPLES: [u8; 16] = [
            0, 1, 2, 15, 16, 31, 32, 63, 64, 95, 127, 128, 191, 192, 254, 255,
        ];

        for first in SAMPLES {
            for second in SAMPLES {
                for third in SAMPLES {
                    assert_standard_decode_chunk_matches_input(&[first, second, third]);
                }
            }
        }
    }

    #[test]
    fn ct_padded_final_quantum_fails_closed_for_invalid_padding_count() {
        let (_, invalid_byte, invalid_padding, written) =
            ct_padded_final_quantum::<Standard>(*b"ABCD", 3);

        assert_ne!(invalid_byte, 0);
        assert_ne!(invalid_padding, 0);
        assert_eq!(written, 0);
        assert_eq!(
            report_ct_error(invalid_byte, invalid_padding),
            Err(DecodeError::InvalidInput)
        );
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_dispatch_scaffold_keeps_scalar_active() {
        assert_eq!(simd::active_backend(), simd::ActiveBackend::Scalar);
        let _candidate = simd::detected_candidate();
    }

    #[test]
    fn encodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"f"[..], &b"Zg=="[..]),
            (&b"fo"[..], &b"Zm8="[..]),
            (&b"foo"[..], &b"Zm9v"[..]),
            (&b"foob"[..], &b"Zm9vYg=="[..]),
            (&b"fooba"[..], &b"Zm9vYmE="[..]),
            (&b"foobar"[..], &b"Zm9vYmFy"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.encode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn decodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"Zg=="[..], &b"f"[..]),
            (&b"Zm8="[..], &b"fo"[..]),
            (&b"Zm9v"[..], &b"foo"[..]),
            (&b"Zm9vYg=="[..], &b"foob"[..]),
            (&b"Zm9vYmE="[..], &b"fooba"[..]),
            (&b"Zm9vYmFy"[..], &b"foobar"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.decode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn supports_unpadded_url_safe() {
        let mut encoded = [0u8; 16];
        let written = URL_SAFE_NO_PAD
            .encode_slice(b"\xfb\xff", &mut encoded)
            .unwrap();
        assert_eq!(&encoded[..written], b"-_8");

        let mut decoded = [0u8; 2];
        let written = URL_SAFE_NO_PAD
            .decode_slice(&encoded[..written], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..written], b"\xfb\xff");
    }

    #[test]
    fn decodes_in_place() {
        let mut buffer = *b"Zm9vYmFy";
        let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
        assert_eq!(decoded, b"foobar");
    }

    #[test]
    fn rejects_non_canonical_padding_bits() {
        let mut output = [0u8; 4];
        assert_eq!(
            STANDARD.decode_slice(b"Zh==", &mut output),
            Err(DecodeError::InvalidPadding { index: 1 })
        );
        assert_eq!(
            STANDARD.decode_slice(b"Zm9=", &mut output),
            Err(DecodeError::InvalidPadding { index: 2 })
        );
    }
}
