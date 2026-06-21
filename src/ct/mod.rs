//! Constant-time-oriented scalar decoding APIs.
//!
//! This module is separate from the default decoder so callers can opt into a
//! slower path with a narrower timing target. It avoids lookup tables indexed
//! by secret input bytes while mapping Base64 symbols and reports malformed
//! content through one opaque error. It is not documented as a formally
//! verified cryptographic constant-time API.
//!
//! # Security
//!
//! Input length, decoded length, selected alphabet, and final success or
//! failure remain public. The clear-tail methods wipe caller-owned output on
//! error, but decoded bytes are written during the fixed-shape decode loop
//! before final validation is reported. In shared-memory, enclave, or HSM-style
//! threat models where another component can observe the output buffer during
//! the call, prefer [`crate::ct::CtEngine::decode_slice_staged_clear_tail`]
//! with a private staging buffer. In those deployments,
//! [`crate::ct::CtEngine::decode_slice_clear_tail`] is not sufficient by
//! itself because it wipes caller-owned output only after the internal decode
//! loop reaches the final error gate. Treat
//! [`crate::ct::CtEngine::decode_slice_staged_clear_tail`] as the default for
//! shared-memory, enclave, HSM-adjacent, or multi-principal deployments;
//! [`crate::ct::CtEngine::decode_slice_clear_tail`] is appropriate only when
//! the output buffer is not observable during the call.
//!
//! Applications that already admit the optional `base64-ng-sanitization`
//! companion can use its `CtDecodeSanitizationExt` helpers to decode into
//! `sanitization` secret containers. With that companion's `high-assurance`
//! feature enabled, supported native targets can decode directly into
//! `sanitization::LockedSecretBytes` or `sanitization::LockedSecretVec`.
//!
//! # Platform Posture
//!
//! The CT result gate uses architecture-specific best-effort barriers where
//! stable Rust exposes them. On `AArch64`, the emitted CSDB hint is reported as
//! `hardware-speculation-barrier-unattested` because older cores may treat it
//! as a no-op; deployments must attest the exact core behavior before relying
//! on it for high assurance. On RISC-V, `fence rw, rw` is an ordering fence,
//! not a Spectre-v1 speculation barrier, and the built-in high-assurance
//! runtime policy intentionally rejects that posture. RISC-V deployments on
//! speculative cores need platform-level mitigations and startup policy checks
//! that make the gap explicit.
//!
//! The dependency-free comparison helpers on redacted buffers are
//! constant-time-oriented best effort, not formally audited MAC or token
//! comparison primitives. Applications that can admit dependencies and need a
//! reviewed comparison primitive should use one at the protocol boundary.
//!
//! The CT decoder exposes only clear-tail and stack-backed decode APIs. The
//! former non-clear-tail methods were removed before the `1.0` stable boundary
//! because they could leave decoded plaintext in caller-owned buffers after
//! malformed input errors.
//!
//! ```compile_fail
//! use base64_ng::ct;
//!
//! let mut output = [0u8; 8];
//! let _ = ct::STANDARD.decode_slice(b"aGk=", &mut output);
//! ```
//!
//! ```compile_fail
//! use base64_ng::ct;
//!
//! let mut buffer = *b"aGk=";
//! let _ = ct::STANDARD.decode_in_place(&mut buffer);
//! ```
#[cfg(feature = "alloc")]
use crate::SecretBuffer;
use crate::{Alphabet, DecodeError, DecodedBuffer, Standard, UrlSafe};
use core::marker::PhantomData;

/// Standard Base64 constant-time-oriented decoder with padding.
pub const STANDARD: CtEngine<Standard, true> = CtEngine::new();

/// Standard Base64 constant-time-oriented decoder without padding.
pub const STANDARD_NO_PAD: CtEngine<Standard, false> = CtEngine::new();

/// URL-safe Base64 constant-time-oriented decoder with padding.
pub const URL_SAFE: CtEngine<UrlSafe, true> = CtEngine::new();

/// URL-safe Base64 constant-time-oriented decoder without padding.
pub const URL_SAFE_NO_PAD: CtEngine<UrlSafe, false> = CtEngine::new();

/// A zero-sized constant-time-oriented Base64 decoder.
///
/// # Security
///
/// For ordinary secret-bearing inputs, prefer
/// [`Self::decode_slice_clear_tail`], [`Self::decode_buffer`], or
/// [`Self::decode_in_place_clear_tail`]. For shared-memory,
/// enclave-adjacent, HSM-style, or multi-principal deployments where
/// another component can observe caller-owned output during the call, use
/// [`Self::decode_slice_staged_clear_tail`] with a private staging buffer
/// so malformed input cannot transiently write decoded bytes into the
/// public output buffer before the final error gate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CtEngine<A, const PAD: bool> {
    alphabet: PhantomData<A>,
}

impl<A, const PAD: bool> CtEngine<A, PAD>
where
    A: Alphabet,
{
    /// Creates a new constant-time-oriented decoder engine.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alphabet: PhantomData,
        }
    }

    /// Returns whether this constant-time-oriented decoder expects padded
    /// input.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Validates `input` without writing decoded bytes.
    ///
    /// This uses the same constant-time-oriented symbol mapping and opaque
    /// malformed-input error behavior as
    /// [`Self::decode_slice_clear_tail`]. Input length, padding length, and
    /// final success or failure remain public.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// ct::STANDARD.validate_result(b"aGVsbG8=").unwrap();
    /// assert!(ct::STANDARD.validate_result(b"aGVsbG8").is_err());
    /// ```
    pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError> {
        ct_validate_decode::<A, PAD>(input)
    }

    /// Returns whether `input` is valid for this constant-time-oriented
    /// decoder.
    ///
    /// This is a convenience wrapper around [`Self::validate_result`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// assert!(ct::URL_SAFE_NO_PAD.validate(b"-_8"));
    /// assert!(!ct::URL_SAFE_NO_PAD.validate(b"+/8"));
    /// ```
    #[must_use]
    pub fn validate(&self, input: &[u8]) -> bool {
        self.validate_result(input).is_ok()
    }

    /// Returns the exact decoded length for valid input.
    ///
    /// This uses the same constant-time-oriented validation policy as
    /// [`Self::validate_result`] before returning a length. Input length,
    /// padding length, and final success or failure remain public.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        ct_decoded_len::<A, PAD>(input)
    }

    /// Decodes `input` into `output` and clears all bytes after the
    /// decoded prefix.
    ///
    /// If decoding fails, the entire output buffer is cleared before the
    /// error is returned. Use this variant for sensitive payloads where
    /// partially decoded bytes from rejected input should not remain in the
    /// caller-owned output buffer.
    ///
    /// # Security: Transient Plaintext Window
    ///
    /// Decoded bytes are written to `output` progressively during the
    /// fixed-shape decode loop before malformed-input detection is
    /// complete. On error, the entire `output` is wiped before returning,
    /// but a concurrent same-process observer with access to `output`
    /// during the call may observe transient partial plaintext from valid
    /// leading quanta. For shared-memory, enclave-adjacent, HSM-style, or
    /// multi-principal deployments where even transient writes are
    /// unacceptable, use [`Self::decode_slice_staged_clear_tail`] with a
    /// private staging buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// let mut output = [0xff; 8];
    /// let written = ct::STANDARD
    ///     .decode_slice_clear_tail(b"aGk=", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    #[must_use = "handle decode errors; use decode_slice_staged_clear_tail for shared-memory or HSM-style threat models"]
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match ct_decode_slice::<A, PAD>(input, output) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(output);
                return Err(err);
            }
        };
        crate::wipe_tail(output, written);
        Ok(written)
    }

    /// Decodes through caller-provided private staging before copying into
    /// `output`.
    ///
    /// This variant is for shared-memory or sandboxed deployments where
    /// the caller-owned `output` buffer must not contain transient decoded
    /// bytes from malformed input. The `staging` buffer must be at least
    /// the decoded length of `input` and must not be shared with
    /// untrusted concurrent observers. On success, decoded bytes are
    /// copied from `staging` into `output`; on error, both buffers are
    /// cleared before returning.
    ///
    /// Input length, final success or failure, and decoded length remain
    /// public.
    #[must_use = "handle decode errors; staged decode is for shared-memory or HSM-style threat models"]
    pub fn decode_slice_staged_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
        staging: &mut [u8],
    ) -> Result<usize, DecodeError> {
        ct_decode_slice_staged_clear_tail::<A, PAD>(input, output, staging)
    }

    /// Decodes `input` into a stack-backed buffer.
    ///
    /// This uses the same constant-time-oriented scalar decoder as
    /// [`Self::decode_slice_clear_tail`] and clears the internal backing
    /// array before returning an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// let decoded = ct::STANDARD.decode_buffer::<5>(b"aGVsbG8=").unwrap();
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

    /// Decodes `input` into an owned byte vector.
    ///
    /// This uses the same constant-time-oriented scalar decoder as
    /// [`Self::decode_slice_clear_tail`]. If decoding fails, the allocated
    /// output buffer is cleared before the error is returned.
    ///
    /// Use [`Self::decode_secret`] for secret-bearing payloads that should stay
    /// on the crate's redacted, drop-wiping buffer path. Use
    /// [`Self::decode_secret_staged`] for shared-memory, enclave-adjacent,
    /// HSM-style, or multi-principal deployments where even transient writes
    /// into the final heap allocation are unacceptable.
    #[cfg(feature = "alloc")]
    #[must_use = "for secret-bearing payloads use decode_secret, which returns a redacted buffer with drop-time cleanup"]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = self.decoded_len(input)?;
        let mut output = alloc::vec![0; required];
        // decode_slice_clear_tail wipes output on error.
        let written = self.decode_slice_clear_tail(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a redacted owned secret buffer.
    ///
    /// This is the recommended heap-owning CT decode path for secret-bearing
    /// payloads. It decodes with [`Self::decode_vec`] and then wraps the result
    /// in [`SecretBuffer`], which redacts formatting and clears initialized
    /// bytes plus spare vector capacity on drop.
    ///
    /// # Security: Transient Plaintext Window
    ///
    /// This function uses the non-staged CT decode path. Decoded bytes are
    /// written transiently into the heap allocation before the final error
    /// gate. On error, the allocation is wiped before returning, but a
    /// concurrent same-process observer with access to that allocation during
    /// the call may observe transient partial plaintext. For shared-memory,
    /// enclave-adjacent, HSM-style, or multi-principal deployments where even
    /// transient writes into the final heap allocation are unacceptable, use
    /// [`Self::decode_secret_staged`] with a stack-backed private staging
    /// capacity large enough for the decoded value.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
    /// assert!(decoded.constant_time_eq_public_len(b"hello"));
    /// ```
    #[cfg(feature = "alloc")]
    pub fn decode_secret(&self, input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        self.decode_vec(input).map(SecretBuffer::from_vec)
    }

    /// Decodes `input` into a redacted owned secret buffer through private
    /// stack staging.
    ///
    /// `STAGE` must be at least the decoded length of `input`. Decoded bytes
    /// are written to a stack-backed staging buffer first and copied into the
    /// returned heap buffer only after the full constant-time-oriented decode
    /// succeeds. On error, both staging and heap output buffers are wiped before
    /// returning.
    ///
    /// This is the preferred owned decode API for shared-memory,
    /// enclave-adjacent, HSM-style, or multi-principal deployments where the
    /// final heap allocation must not contain transient partial plaintext from
    /// rejected input.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::StagingTooSmall`] if `STAGE` is smaller than the
    /// decoded length of `input`. `STAGE` is checked at runtime because the
    /// encoded input length is not a compile-time value.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// let decoded = ct::STANDARD
    ///     .decode_secret_staged::<5>(b"aGVsbG8=")
    ///     .unwrap();
    /// assert!(decoded.constant_time_eq_public_len(b"hello"));
    /// ```
    #[cfg(feature = "alloc")]
    pub fn decode_secret_staged<const STAGE: usize>(
        &self,
        input: &[u8],
    ) -> Result<SecretBuffer, DecodeError> {
        let required = self.decoded_len(input)?;
        let mut staging = DecodedBuffer::<STAGE>::new();
        let mut output = alloc::vec![0; required];
        let written =
            self.decode_slice_staged_clear_tail(input, &mut output, staging.as_mut_capacity())?;
        output.truncate(written);
        Ok(SecretBuffer::from_vec(output))
    }

    /// Decodes `buffer` in place and clears all bytes after the decoded
    /// prefix.
    ///
    /// If decoding fails, the entire buffer is cleared before the error is
    /// returned.
    ///
    /// # Security: Transient Plaintext Window
    ///
    /// This in-place API writes decoded bytes into `buffer` during the
    /// fixed-shape decode loop before malformed-input detection is
    /// complete. On error, the entire buffer is wiped before returning,
    /// but concurrent same-process observers with access to the same memory
    /// can observe transient partial plaintext. Use
    /// [`Self::decode_slice_staged_clear_tail`] with a private staging
    /// buffer when shared-memory or enclave-adjacent deployments cannot
    /// tolerate that window.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::ct;
    ///
    /// let mut buffer = *b"aGk=";
    /// let decoded = ct::STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
    ///
    /// assert_eq!(decoded, b"hi");
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let len = match ct_decode_in_place::<A, PAD>(buffer) {
            Ok(len) => len,
            Err(err) => {
                crate::wipe_bytes(buffer);
                return Err(err);
            }
        };
        crate::wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }
}

impl<A, const PAD: bool> core::fmt::Display for CtEngine<A, PAD> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "ct padded={PAD}")
    }
}

mod decode;
mod equality;
mod padded;
mod unpadded;

use decode::{
    ct_decode_in_place, ct_decode_slice, ct_decode_slice_staged_clear_tail, ct_decoded_len,
    ct_validate_decode,
};
#[cfg(test)]
pub(crate) use equality::report_ct_error;
pub(crate) use equality::{
    constant_time_eq_fixed_width_array, constant_time_eq_public_len, ct_mask_eq_u8, ct_mask_lt_u8,
};
#[cfg(test)]
pub(crate) use padded::ct_padded_final_quantum;
