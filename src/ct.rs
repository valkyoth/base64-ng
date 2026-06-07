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
use crate::{
    Alphabet, DecodeError, DecodedBuffer, Standard, UrlSafe, decoded_capacity, read_quad,
    wipe_bytes, wipe_tail,
};
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

#[inline]
pub(crate) const fn ct_mask_bit(bit: u8) -> u8 {
    0u8.wrapping_sub(bit & 1)
}

#[inline]
pub(crate) const fn ct_mask_nonzero_u8(value: u8) -> u8 {
    let wide = value as u16;
    let negative = 0u16.wrapping_sub(wide);
    let nonzero = ((wide | negative) >> 8) as u8;
    ct_mask_bit(nonzero)
}

#[inline]
pub(crate) const fn ct_mask_eq_u8(left: u8, right: u8) -> u8 {
    !ct_mask_nonzero_u8(left ^ right)
}

#[inline]
pub(crate) const fn ct_mask_lt_u8(left: u8, right: u8) -> u8 {
    let diff = (left as u16).wrapping_sub(right as u16);
    ct_mask_bit((diff >> 8) as u8)
}

#[inline(never)]
pub(crate) fn constant_time_eq_public_len(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    constant_time_eq_same_len(left, right)
}

#[inline(never)]
pub(crate) fn constant_time_eq_fixed_width_array<const N: usize>(
    left: &[u8; N],
    right: &[u8; N],
) -> bool {
    constant_time_eq_same_len(left, right)
}

#[inline(never)]
#[allow(unsafe_code)]
fn constant_time_eq_same_len(left: &[u8], right: &[u8]) -> bool {
    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff = ct_accumulate_u8(diff, *left ^ *right);
    }
    ct_error_gate_barrier(diff, 0);
    // SAFETY: `diff` is an initialized local `u8`; this final volatile read
    // keeps the public equality comparison dependent on a post-barrier load of
    // the accumulated value.
    let result = unsafe { core::ptr::read_volatile(&raw const diff) };
    result == 0
}

#[inline(never)]
#[allow(unsafe_code)]
fn ct_accumulate_u8(accumulator: u8, value: u8) -> u8 {
    let result = core::hint::black_box(accumulator) | core::hint::black_box(value);
    // SAFETY: `result` is an initialized local `u8`; the volatile read is a
    // dependency-free optimizer barrier for the accumulation value and does not
    // access caller memory.
    unsafe { core::ptr::read_volatile(&raw const result) }
}

fn ct_decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded::<A>(input, output)
    } else {
        ct_decode_unpadded::<A>(input, output)
    }
}

fn ct_decode_slice_staged_clear_tail<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    staging: &mut [u8],
) -> Result<usize, DecodeError> {
    let required = match ct_decoded_len::<A, PAD>(input) {
        Ok(required) => required,
        Err(err) => {
            wipe_bytes(output);
            wipe_bytes(staging);
            return Err(err);
        }
    };

    if output.len() < required {
        wipe_bytes(output);
        wipe_bytes(staging);
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    if staging.len() < required {
        wipe_bytes(output);
        wipe_bytes(staging);
        return Err(DecodeError::StagingTooSmall {
            required,
            available: staging.len(),
        });
    }

    let written = match ct_decode_slice::<A, PAD>(input, &mut staging[..required]) {
        Ok(written) => written,
        Err(err) => {
            wipe_bytes(output);
            wipe_bytes(staging);
            return Err(err);
        }
    };

    output[..written].copy_from_slice(&staging[..written]);
    wipe_bytes(staging);
    wipe_tail(output, written);
    Ok(written)
}

fn ct_decode_in_place<A: Alphabet, const PAD: bool>(
    buffer: &mut [u8],
) -> Result<usize, DecodeError> {
    if buffer.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded_in_place::<A>(buffer)
    } else {
        ct_decode_unpadded_in_place::<A>(buffer)
    }
}

#[inline(never)]
#[allow(unsafe_code)]
fn ct_error_gate_barrier(invalid_byte: u8, invalid_padding: u8) {
    core::hint::black_box(invalid_byte | invalid_padding);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

    #[cfg(all(not(miri), not(kani), any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // SAFETY: `lfence` does not access memory and is used as a speculation
        // barrier before the public success/failure branch is observed.
        unsafe {
            core::arch::asm!("lfence", options(nostack, preserves_flags, nomem));
        }
    }

    #[cfg(all(not(miri), not(kani), target_arch = "aarch64"))]
    {
        // Older cores may treat CSDB as a no-op; runtime reporting marks this
        // as unattested until the deployment provides platform evidence.
        // SAFETY: these barriers do not access memory.
        unsafe {
            core::arch::asm!("isb sy", "hint #20", options(nostack, preserves_flags));
        }
    }

    #[cfg(all(not(miri), not(kani), target_arch = "arm"))]
    {
        // SAFETY: `isb sy` does not access memory and is used as the best
        // available stable ARM speculation boundary for this crate.
        unsafe {
            core::arch::asm!("isb sy", options(nostack, preserves_flags));
        }
    }

    #[cfg(all(
        not(miri),
        not(kani),
        any(target_arch = "riscv32", target_arch = "riscv64")
    ))]
    {
        // RISC-V base ISA does not provide a canonical speculation barrier.
        // `fence rw, rw` is the available ordering primitive for the CT public
        // result gate and is reported separately as `ordering-fence`; callers
        // on speculative RISC-V cores must use platform mitigations because
        // this does not satisfy `BackendPolicy::HighAssuranceScalarOnly`.
        // SAFETY: the assembly block does not access memory.
        unsafe {
            core::arch::asm!("fence rw, rw", options(nostack, preserves_flags));
        }
    }
}

fn ct_validate_decode<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<(), DecodeError> {
    if input.is_empty() {
        return Ok(());
    }

    if PAD {
        ct_validate_padded::<A>(input)
    } else {
        ct_validate_unpadded::<A>(input)
    }
}

fn ct_decoded_len<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    ct_validate_decode::<A, PAD>(input)?;
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        Ok(input.len() / 4 * 3 - ct_padding_len(input))
    } else {
        let full_quads = input.len() / 4 * 3;
        match input.len() % 4 {
            0 => Ok(full_quads),
            2 => Ok(full_quads + 1),
            3 => Ok(full_quads + 2),
            _ => Err(DecodeError::InvalidLength),
        }
    }
}

fn ct_validate_padded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
    let (_, final_invalid_byte, final_invalid_padding, _) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;

    report_ct_error(invalid_byte, invalid_padding)
}

fn ct_validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 <= input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        read += 4;
    }

    match read_tail_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding) {
        [] => {}
        [b0, b1] => {
            let (_, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
        }
        [b0, b1, b2] => {
            let (_, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (_, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

    report_ct_error(invalid_byte, invalid_padding)
}

pub(crate) fn ct_padded_final_quantum<A: Alphabet>(
    input: [u8; 4],
    padding: usize,
) -> ([u8; 3], u8, u8, usize) {
    let [b0, b1, b2, b3] = input;
    let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
    let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
    let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
    let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

    let padding_byte = match padding {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => return ([0; 3], 0xff, 0xff, 0),
    };
    let no_padding = ct_mask_eq_u8(padding_byte, 0);
    let one_padding = ct_mask_eq_u8(padding_byte, 1);
    let two_padding = ct_mask_eq_u8(padding_byte, 2);
    let require_v2 = no_padding | one_padding;
    let require_v3 = no_padding;

    let invalid_byte = !valid0 | !valid1 | (!valid2 & require_v2) | (!valid3 & require_v3);
    let invalid_padding = (ct_mask_nonzero_u8(v1 & 0b0000_1111) & two_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_nonzero_u8(v2 & 0b0000_0011)) & one_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_eq_u8(b3, b'=')) & no_padding);

    (
        [(v0 << 2) | (v1 >> 4), (v1 << 4) | (v2 >> 2), (v2 << 6) | v3],
        invalid_byte,
        invalid_padding,
        3 - padding,
    )
}

fn ct_decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let required = input.len() / 4 * 3 - padding;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    output[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_padded_in_place<A: Alphabet>(buffer: &mut [u8]) -> Result<usize, DecodeError> {
    if !buffer.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(buffer);
    let required = buffer.len() / 4 * 3 - padding;
    if required > buffer.len() {
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < buffer.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    buffer[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    if write != required {
        ct_error_gate_barrier(invalid_byte, invalid_padding);
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }
    if let Err(err) = report_ct_error(invalid_byte, invalid_padding) {
        wipe_bytes(buffer);
        return Err(err);
    }
    Ok(write)
}

fn ct_decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(input.len());
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    match read_tail_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding) {
        [] => {}
        [b0, b1] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            output[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        [b0, b1, b2] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_unpadded_in_place<A: Alphabet>(buffer: &mut [u8]) -> Result<usize, DecodeError> {
    if buffer.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(buffer.len());
    if required > buffer.len() {
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= buffer.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    let tail = read_tail_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
    match tail {
        [] => {}
        [b0, b1] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        [b0, b1, b2] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            buffer[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

    if write != required {
        ct_error_gate_barrier(invalid_byte, invalid_padding);
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }
    if let Err(err) = report_ct_error(invalid_byte, invalid_padding) {
        wipe_bytes(buffer);
        return Err(err);
    }
    Ok(write)
}

fn read_tail(input: &[u8], offset: usize) -> Result<&[u8], DecodeError> {
    input.get(offset..).ok_or(DecodeError::InvalidLength)
}

fn read_quad_or_mark_invalid(
    input: &[u8],
    offset: usize,
    invalid_byte: &mut u8,
    invalid_padding: &mut u8,
) -> [u8; 4] {
    if let Ok(quad) = read_quad(input, offset) {
        quad
    } else {
        debug_assert!(
            false,
            "read_quad failed inside length-validated constant-time decode loop"
        );
        *invalid_byte = 0xff;
        *invalid_padding = 0xff;
        [0; 4]
    }
}

fn read_tail_or_mark_invalid<'a>(
    input: &'a [u8],
    offset: usize,
    invalid_byte: &mut u8,
    invalid_padding: &mut u8,
) -> &'a [u8] {
    if let Ok(tail) = read_tail(input, offset) {
        tail
    } else {
        debug_assert!(
            false,
            "read_tail failed inside length-validated constant-time decode loop"
        );
        *invalid_byte = 0xff;
        *invalid_padding = 0xff;
        &[]
    }
}

#[inline(never)]
#[allow(unsafe_code)]
fn ct_decode_alphabet_byte<A: Alphabet>(byte: u8) -> (u8, u8) {
    let mut decoded = 0u8;
    let mut valid = 0u8;
    let mut candidate = 0u8;

    while candidate < 64 {
        let matches = core::hint::black_box(ct_mask_eq_u8(
            core::hint::black_box(byte),
            core::hint::black_box(A::ENCODE[candidate as usize]),
        ));
        decoded = ct_accumulate_u8(decoded, candidate & matches);
        valid = ct_accumulate_u8(valid, matches);
        candidate += 1;
    }

    (decoded, valid)
}

fn ct_padding_len(input: &[u8]) -> usize {
    let Some((&last, before_last_prefix)) = input.split_last() else {
        return 0;
    };
    let Some(&before_last) = before_last_prefix.last() else {
        return 0;
    };
    usize::from(ct_mask_eq_u8(last, b'=') & 1) + usize::from(ct_mask_eq_u8(before_last, b'=') & 1)
}

pub(crate) fn report_ct_error(invalid_byte: u8, invalid_padding: u8) -> Result<(), DecodeError> {
    ct_error_gate_barrier(invalid_byte, invalid_padding);

    if (invalid_byte | invalid_padding) != 0 {
        Err(DecodeError::InvalidInput)
    } else {
        Ok(())
    }
}
