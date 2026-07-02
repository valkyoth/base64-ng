#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

//! `base64-ng` is a `no_std`-first Base64 encoder and decoder.
//!
//! The core API provides strict RFC 4648-style behavior, caller-owned output
//! buffers, and an audited scalar fallback. The `1.2.x` line admits selected
//! SIMD encode acceleration while keeping decode on the scalar foundation.
//! Any accelerated backend must match the scalar module byte-for-byte and pass
//! the documented admission evidence before dispatch can select it.
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
//! validation may branch or return early based on malformed input, and strict
//! [`DecodeError`] values can include input-derived bytes and indexes. Do not
//! log strict decode errors verbatim for secret-bearing input; log
//! [`DecodeError::kind`] instead. Use [`ct::STANDARD`],
//! [`crate::ct::URL_SAFE_NO_PAD`], or [`Engine::ct_decoder`] for secret-bearing
//! payloads where decode timing posture matters more than exact error indexes.
//!
//! Recommended heap-owning pattern for secret-bearing standard Base64:
//!
//! ```
//! # #[cfg(feature = "alloc")]
//! # {
//! use base64_ng::ct;
//!
//! let expected = b"session-key";
//! let decoded = ct::STANDARD.decode_secret(b"c2Vzc2lvbi1rZXk=").unwrap();
//!
//! assert!(decoded.constant_time_eq_public_len(expected));
//! # }
//! ```
//!
//! For shared-memory, enclave-adjacent, HSM-style, or multi-principal
//! deployments where even transient writes into caller-owned output are
//! unacceptable, use [`ct::CtEngine::decode_slice_staged_clear_tail`] with a
//! private staging buffer.
//! CT behavior is best-effort and build-profile specific. Link-Time
//! Optimization can change generated code shape across crate boundaries, so
//! high-assurance deployments must rerun the dudect and generated-assembly
//! evidence scripts for their exact compiler, target, feature set, and release
//! profile before treating CT decode as acceptable.
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
mod decode_backend;
mod encode_backend;
mod engine;
mod errors;
mod length;
mod profiles;
mod scalar;
mod scalar_encode_in_place;
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
pub use engine::Engine;
pub use errors::{DecodeError, DecodeErrorKind, EncodeError};
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
/// This module exposes backend posture so callers can log, assert, or audit
/// whether execution is scalar-only, using an admitted encode backend, or
/// merely detecting future SIMD candidates.
pub mod runtime;

#[cfg(feature = "stream")]
pub mod stream;

/// Best-effort dependency-free wipe for caller-owned byte slices.
///
/// This is the same hardened cleanup primitive used internally by the core
/// crate: byte-wise volatile zero writes followed by the crate's
/// architecture-gated wipe barrier and a compiler fence. It is exposed so
/// companion crates and integrations can reuse the audited cleanup boundary
/// without duplicating unsafe code.
///
/// This is not a formal zeroization guarantee. It cannot clear historical
/// copies, registers, caches, swap, hibernation images, core dumps, or
/// OS-level memory snapshots. High-assurance applications still need their own
/// platform-approved memory hygiene controls.
pub fn secure_wipe(bytes: &mut [u8]) {
    cleanup::wipe_bytes(bytes);
}

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

/// Encodes `input` as strict standard padded Base64.
///
/// This is a convenience wrapper around [`Engine::encode_string`] on
/// [`STANDARD`] for callers migrating from simpler Base64 APIs. It requires
/// the `alloc` feature because it returns an owned string.
///
/// # Examples
///
/// ```
/// assert_eq!(base64_ng::encode(b"hello").unwrap(), "aGVsbG8=");
/// ```
#[cfg(feature = "alloc")]
pub fn encode(input: &[u8]) -> Result<alloc::string::String, EncodeError> {
    STANDARD.encode_string(input)
}

/// Encodes `input` as strict standard padded Base64.
///
/// This is a convenience wrapper around [`Engine::encode_string_infallible`] on
/// [`STANDARD`] for ordinary byte-to-Base64 paths where encoding failure would
/// indicate an internal length/allocation invariant failure rather than invalid
/// input.
///
/// Prefer [`encode`] when handling untrusted length metadata, constrained
/// allocation environments, or code paths that must return a recoverable error
/// instead of panicking.
///
/// # Panics
///
/// Panics if [`encode`] returns an error. This includes encoded length
/// overflow; on 32-bit targets, inputs larger than roughly 1.5 GiB can
/// overflow the encoded length. For attacker-controlled or externally sized
/// buffers, use [`encode`], which returns a recoverable
/// [`EncodeError::LengthOverflow`].
///
/// # Examples
///
/// ```
/// assert_eq!(base64_ng::encode_infallible(b"hello"), "aGVsbG8=");
/// ```
#[cfg(feature = "alloc")]
#[must_use]
pub fn encode_infallible(input: &[u8]) -> alloc::string::String {
    STANDARD.encode_string_infallible(input)
}

/// Decodes strict standard padded Base64 into an owned byte vector.
///
/// This is a convenience wrapper around [`Engine::decode_vec`] on
/// [`STANDARD`].
/// It uses the normal strict decoder, not the [`crate::ct`] module, and may
/// branch or return early on malformed input. For secret-bearing payloads where
/// malformed-input timing matters, use
/// [`crate::ct::CtEngine::decode_secret`] through [`crate::ct::STANDARD`]
/// instead.
///
/// # Examples
///
/// ```
/// assert_eq!(base64_ng::decode("aGVsbG8=").unwrap(), b"hello");
/// ```
#[cfg(feature = "alloc")]
#[must_use = "handle decode errors; use crate::ct for secret-bearing payloads"]
pub fn decode(input: impl AsRef<[u8]>) -> Result<alloc::vec::Vec<u8>, DecodeError> {
    STANDARD.decode_vec(input.as_ref())
}

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

/// Compares two byte slices with a public length-mismatch branch.
///
/// Equal-length inputs are scanned fully before returning. Different lengths
/// return `false` immediately because length is treated as public. This is a
/// dependency-free, constant-time-oriented best-effort helper, not a formally
/// verified cryptographic MAC, password, or bearer-token comparison primitive.
///
/// # Security
///
/// This helper is intended to avoid ordinary early-exit equality on values
/// whose length is public. It is not a formal constant-time guarantee and
/// should not be the sole primitive admitted at MAC, password, or bearer-token
/// protocol boundaries in high-assurance systems. Use a reviewed comparison
/// primitive at that boundary when your dependency policy allows one.
///
/// # Examples
///
/// ```
/// assert!(base64_ng::constant_time_eq(b"token", b"token"));
/// assert!(!base64_ng::constant_time_eq(b"token", b"Token"));
/// assert!(!base64_ng::constant_time_eq(b"token", b"token2"));
/// ```
#[must_use]
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    constant_time_eq_public_len(left, right)
}

/// Clears caller-owned bytes with this crate's best-effort cleanup primitive.
///
/// This helper exposes the same dependency-free cleanup path used by
/// `base64-ng` stack-backed buffers: byte-wise volatile zero writes followed by
/// the target-specific wipe barrier documented in the crate-level
/// zeroization caveat. It is intended for companion crates and applications
/// that need a small reviewed cleanup primitive without pulling cleanup logic
/// into generated code.
///
/// # Security
///
/// This is data-retention reduction, not a formal zeroization guarantee. It
/// cannot clear historical copies, registers, cache lines, swap, hibernation
/// images, core dumps, or platform snapshots. High-assurance deployments
/// should pair it with their approved platform memory controls.
pub fn clear_bytes(bytes: &mut [u8]) {
    wipe_bytes(bytes);
}

#[cfg(kani)]
mod kani_proofs;

#[cfg(test)]
mod encode_surface_tests;
#[cfg(test)]
mod tests;
