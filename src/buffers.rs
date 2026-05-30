//! Stack-backed and owned buffer wrappers.

use crate::{
    DecodeError, EncodeError, STANDARD, constant_time_eq_public_len, wipe_bytes, wipe_tail,
};
#[cfg(feature = "alloc")]
use crate::{wipe_vec_all, wipe_vec_spare_capacity};
#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

/// Stack-backed encoded Base64 output.
///
/// This type is intended for short values where heap allocation would be
/// unnecessary but manually sizing and passing a separate output slice is
/// noisy. Its visible bytes are produced by crate encoders, so [`Self::as_str`]
/// can return `&str` without exposing a fallible UTF-8 conversion to callers.
/// [`core::fmt::Display`] intentionally writes the full encoded text; use
/// `SecretBuffer` for encoded secrets that may reach logs or error messages.
///
/// The backing array is cleared when the value is dropped. This is best-effort
/// data-retention reduction and is not a formal zeroization guarantee.
///
/// On `wasm32` targets, the wipe barrier uses only a compiler fence. The wasm
/// runtime JIT may still optimize or retain cleared bytes in ways this crate
/// cannot control. `wasm32` builds fail closed by default; enable
/// `allow-wasm32-best-effort-wipe` only when the deployment explicitly accepts
/// this limitation and applies its own memory strategy around stack-backed
/// buffers.
pub struct EncodedBuffer<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

/// Owned stack array extracted from [`EncodedBuffer`].
///
/// This wrapper keeps the extracted encoded bytes on the crate's best-effort
/// drop-time cleanup path. Use
/// [`Self::into_exposed_unprotected_array_caller_must_zeroize`] only when a
/// bare array is unavoidable and the caller will handle cleanup.
pub struct ExposedEncodedArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> ExposedEncodedArray<CAP> {
    /// Wraps an encoded backing array and visible length.
    ///
    /// # Panics
    ///
    /// Panics if `len` is greater than `CAP`.
    #[must_use]
    pub const fn from_array(bytes: [u8; CAP], len: usize) -> Self {
        assert!(len <= CAP, "visible length exceeds array capacity");
        Self { bytes, len }
    }

    /// Returns the visible encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the number of visible encoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether there are no visible encoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the backing array capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Consumes the wrapper and returns a bare array plus visible length.
    ///
    /// This is an unprotected escape hatch. The returned array will not be
    /// cleared by this crate on drop. Callers must clear it with their own
    /// approved zeroization policy.
    ///
    /// # Security
    ///
    /// Treat this as a cleanup-boundary API. Failing to clear the returned
    /// array leaves the encoded bytes in ordinary caller-owned memory until
    /// overwritten by later stack or heap activity.
    #[must_use = "caller must zeroize the returned array"]
    pub fn into_exposed_unprotected_array_caller_must_zeroize(mut self) -> ([u8; CAP], usize) {
        let len = self.len;
        self.len = 0;
        (core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }
}

impl<const CAP: usize> Drop for ExposedEncodedArray<CAP> {
    fn drop(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }
}

impl<const CAP: usize> core::fmt::Debug for ExposedEncodedArray<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExposedEncodedArray")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> EncodedBuffer<CAP> {
    /// Creates an empty encoded buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; CAP],
            len: 0,
        }
    }

    /// Returns the full backing array as an output slice for crate-internal
    /// encode paths.
    pub(crate) fn as_mut_capacity(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Sets the visible length after a crate-internal encode path succeeds.
    pub(crate) fn set_filled(&mut self, written: usize) -> Result<(), EncodeError> {
        debug_assert!(
            written <= CAP,
            "encoder wrote past stack-backed buffer capacity"
        );
        if written > CAP {
            self.clear();
            return Err(EncodeError::OutputTooSmall {
                required: written,
                available: CAP,
            });
        }
        self.len = written;
        Ok(())
    }

    /// Returns the number of visible encoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the buffer has no visible encoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns whether the visible encoded bytes fill the stack backing array.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Returns the stack capacity in bytes.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the number of unused bytes in the stack backing array.
    #[must_use]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.len
    }

    /// Returns the visible encoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the visible encoded bytes as UTF-8 text.
    ///
    /// Encoded Base64 output is produced as ASCII by this crate, so this
    /// method should not fail unless an internal invariant has been broken.
    /// It is provided for callers that prefer a fallible accessor over
    /// [`Self::as_str`].
    pub fn as_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_bytes())
    }

    /// Returns the visible encoded bytes as UTF-8.
    ///
    /// # Panics
    ///
    /// Panics only if the crate's internal invariant is broken and the buffer
    /// contains non-UTF-8 bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self.as_utf8() {
            Ok(output) => output,
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Compares this encoded output to `other` without short-circuiting on the
    /// first differing byte.
    ///
    /// Length and the final equality result remain public. Different lengths
    /// return `false` immediately; use this helper only when the compared
    /// lengths are public protocol facts or have been normalized by the
    /// caller. For equal-length inputs, this helper scans every byte before
    /// returning. It is constant-time-oriented best effort, not a formal
    /// cryptographic constant-time guarantee. This comparison is deliberately
    /// explicit: redacted buffer types do not implement [`PartialEq`] because
    /// `==` would make a best-effort helper look like a formal token/MAC
    /// comparison primitive.
    ///
    /// Do not use this helper as the sole MAC, bearer-token, password-hash, or
    /// authentication-secret comparison primitive in high-assurance systems.
    /// Applications that can admit dependencies should use a reviewed
    /// constant-time comparison primitive, such as `subtle`, at the protocol
    /// boundary.
    #[doc(alias = "constant_time_eq")]
    #[must_use]
    pub fn constant_time_eq_public_len(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.as_bytes(), other)
    }

    /// Consumes the wrapper and returns the backing array plus visible length
    /// inside a drop-wiping exposed wrapper.
    ///
    /// This is an explicit escape hatch for no-alloc interop with APIs that
    /// require ownership of a fixed array. The returned
    /// [`ExposedEncodedArray`] remains redacted by formatting and clears its
    /// backing array on drop.
    #[must_use]
    pub fn into_exposed_array(mut self) -> ExposedEncodedArray<CAP> {
        let len = self.len;
        self.len = 0;
        ExposedEncodedArray::from_array(core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }

    /// Clears the visible bytes and the full backing array.
    pub fn clear(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }

    /// Clears bytes after the visible prefix.
    pub fn clear_tail(&mut self) {
        wipe_tail(&mut self.bytes, self.len);
    }
}

impl<const CAP: usize> AsRef<[u8]> for EncodedBuffer<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const CAP: usize> Clone for EncodedBuffer<CAP> {
    /// Clones the visible encoded bytes into a second stack-backed buffer.
    ///
    /// Security note: cloning duplicates the visible bytes in memory. Both the
    /// original and the clone must be dropped or explicitly cleared before the
    /// duplicated bytes are gone on the crate's best-effort cleanup path. The
    /// compiler may also create temporary stack copies while performing the
    /// copy; those intermediates are outside this crate's cleanup boundary.
    /// Avoid cloning encoded secret material; use `SecretBuffer` when redacted
    /// formatting and heap-owned secret handling are required.
    fn clone(&self) -> Self {
        let mut output = Self::new();
        output.bytes[..self.len].copy_from_slice(self.as_bytes());
        output.len = self.len;
        output
    }
}

impl<const CAP: usize> core::fmt::Debug for EncodedBuffer<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncodedBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> core::fmt::Display for EncodedBuffer<CAP> {
    /// Writes the full Base64 text.
    ///
    /// Security note: this is intentionally not redacted. Do not use
    /// `EncodedBuffer` for encoded secrets that may reach logs or error
    /// messages; use `SecretBuffer` for redacted formatting.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<const CAP: usize> Default for EncodedBuffer<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Drop for EncodedBuffer<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for EncodedBuffer<CAP> {
    type Error = EncodeError;

    /// Encodes bytes into strict standard padded Base64 in a stack-backed
    /// buffer.
    ///
    /// Use [`crate::Engine::encode_buffer`] or [`crate::Profile::encode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.encode_buffer(input)
    }
}

impl<const CAP: usize, const N: usize> TryFrom<&[u8; N]> for EncodedBuffer<CAP> {
    type Error = EncodeError;

    /// Encodes a byte array into strict standard padded Base64 in a
    /// stack-backed buffer.
    ///
    /// Use [`crate::Engine::encode_buffer`] or [`crate::Profile::encode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &[u8; N]) -> Result<Self, Self::Error> {
        Self::try_from(&input[..])
    }
}

impl<const CAP: usize> TryFrom<&str> for EncodedBuffer<CAP> {
    type Error = EncodeError;

    /// Encodes UTF-8 text bytes into strict standard padded Base64 in a
    /// stack-backed buffer.
    ///
    /// This treats the string as raw input bytes. Use
    /// [`crate::Engine::encode_buffer`] or [`crate::Profile::encode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

/// Stack-backed decoded Base64 output.
///
/// This type is intended for short decoded values where heap allocation would
/// be unnecessary but manually sizing and passing a separate output slice is
/// noisy. Decoded data may be binary or secret-bearing, so formatting is
/// redacted and contents are exposed only through explicit byte accessors.
///
/// The backing array is cleared when the value is dropped. This is best-effort
/// data-retention reduction and is not a formal zeroization guarantee.
///
/// On `wasm32` targets, the wipe barrier uses only a compiler fence. The wasm
/// runtime JIT may still optimize or retain cleared bytes in ways this crate
/// cannot control. `wasm32` builds fail closed by default; enable
/// `allow-wasm32-best-effort-wipe` only when the deployment explicitly accepts
/// this limitation and applies its own memory strategy around stack-backed
/// buffers.
pub struct DecodedBuffer<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

/// Owned stack array extracted from [`DecodedBuffer`].
///
/// This wrapper keeps the extracted decoded bytes on the crate's best-effort
/// drop-time cleanup path. Use
/// [`Self::into_exposed_unprotected_array_caller_must_zeroize`] only when a
/// bare array is unavoidable and the caller will handle cleanup.
pub struct ExposedDecodedArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> ExposedDecodedArray<CAP> {
    /// Wraps a decoded backing array and visible length.
    ///
    /// # Panics
    ///
    /// Panics if `len` is greater than `CAP`.
    #[must_use]
    pub const fn from_array(bytes: [u8; CAP], len: usize) -> Self {
        assert!(len <= CAP, "visible length exceeds array capacity");
        Self { bytes, len }
    }

    /// Returns the visible decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the number of visible decoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether there are no visible decoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the backing array capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Consumes the wrapper and returns a bare array plus visible length.
    ///
    /// This is an unprotected escape hatch. The returned array will not be
    /// cleared by this crate on drop. Callers must clear it with their own
    /// approved zeroization policy.
    ///
    /// # Security
    ///
    /// Treat this as a cleanup-boundary API. Failing to clear the returned
    /// array leaves decoded bytes, which may be secret-bearing, in ordinary
    /// caller-owned memory until overwritten by later stack or heap activity.
    #[must_use = "caller must zeroize the returned array"]
    pub fn into_exposed_unprotected_array_caller_must_zeroize(mut self) -> ([u8; CAP], usize) {
        let len = self.len;
        self.len = 0;
        (core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }
}

impl<const CAP: usize> Drop for ExposedDecodedArray<CAP> {
    fn drop(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }
}

impl<const CAP: usize> core::fmt::Debug for ExposedDecodedArray<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExposedDecodedArray")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> DecodedBuffer<CAP> {
    /// Creates an empty decoded buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; CAP],
            len: 0,
        }
    }

    /// Returns the full backing array as an output slice for crate-internal
    /// decode paths.
    pub(crate) fn as_mut_capacity(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Sets the visible length after a crate-internal decode path succeeds.
    pub(crate) fn set_filled(&mut self, written: usize) -> Result<(), DecodeError> {
        debug_assert!(
            written <= CAP,
            "decoder wrote past stack-backed buffer capacity"
        );
        if written > CAP {
            self.clear();
            return Err(DecodeError::OutputTooSmall {
                required: written,
                available: CAP,
            });
        }
        self.len = written;
        Ok(())
    }

    /// Returns the number of visible decoded bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the buffer has no visible decoded bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns whether the visible decoded bytes fill the stack backing array.
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    /// Returns the stack capacity in bytes.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the number of unused bytes in the stack backing array.
    #[must_use]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.len
    }

    /// Returns the visible decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the visible decoded bytes as UTF-8 text.
    ///
    /// Decoded Base64 output is arbitrary bytes, so this method is fallible.
    /// Use [`Self::as_bytes`] when the decoded payload is binary or when text
    /// validation belongs to a higher protocol layer.
    pub fn as_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_bytes())
    }

    /// Compares this decoded output to `other` without short-circuiting on the
    /// first differing byte.
    ///
    /// Length and the final equality result remain public. Different lengths
    /// return `false` immediately; use this helper only when the compared
    /// lengths are public protocol facts or have been normalized by the
    /// caller. For equal-length inputs, this helper scans every byte before
    /// returning. It is constant-time-oriented best effort, not a formal
    /// cryptographic constant-time guarantee. This comparison is deliberately
    /// explicit: redacted buffer types do not implement [`PartialEq`] because
    /// `==` would make a best-effort helper look like a formal token/MAC
    /// comparison primitive.
    ///
    /// Do not use this helper as the sole MAC, bearer-token, password-hash, or
    /// authentication-secret comparison primitive in high-assurance systems.
    /// Applications that can admit dependencies should use a reviewed
    /// constant-time comparison primitive, such as `subtle`, at the protocol
    /// boundary.
    #[doc(alias = "constant_time_eq")]
    #[must_use]
    pub fn constant_time_eq_public_len(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.as_bytes(), other)
    }

    /// Consumes the wrapper and returns the backing array plus visible length
    /// inside a drop-wiping exposed wrapper.
    ///
    /// This is an explicit escape hatch for no-alloc interop with APIs that
    /// require ownership of a fixed array. The returned
    /// [`ExposedDecodedArray`] remains redacted by formatting and clears its
    /// backing array on drop.
    #[must_use]
    pub fn into_exposed_array(mut self) -> ExposedDecodedArray<CAP> {
        let len = self.len;
        self.len = 0;
        ExposedDecodedArray::from_array(core::mem::replace(&mut self.bytes, [0u8; CAP]), len)
    }

    /// Clears the visible bytes and the full backing array.
    pub fn clear(&mut self) {
        wipe_bytes(&mut self.bytes);
        self.len = 0;
    }

    /// Clears bytes after the visible prefix.
    pub fn clear_tail(&mut self) {
        wipe_tail(&mut self.bytes, self.len);
    }
}

impl<const CAP: usize> AsRef<[u8]> for DecodedBuffer<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const CAP: usize> Clone for DecodedBuffer<CAP> {
    /// Clones the visible decoded bytes into a second stack-backed buffer.
    ///
    /// Security note: cloning duplicates decoded bytes in memory. Both the
    /// original and the clone must be dropped or explicitly cleared before the
    /// duplicated bytes are gone on the crate's best-effort cleanup path. The
    /// compiler may also create temporary stack copies while performing the
    /// copy; those intermediates are outside this crate's cleanup boundary. For
    /// high-assurance applications, avoid cloning decoded key material and use
    /// `SecretBuffer` for heap-owned secrets without a `Clone` implementation.
    fn clone(&self) -> Self {
        let mut output = Self::new();
        output.bytes[..self.len].copy_from_slice(self.as_bytes());
        output.len = self.len;
        output
    }
}

impl<const CAP: usize> core::fmt::Debug for DecodedBuffer<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DecodedBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> Default for DecodedBuffer<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> Drop for DecodedBuffer<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<const CAP: usize> TryFrom<&[u8]> for DecodedBuffer<CAP> {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 into a stack-backed buffer.
    ///
    /// Use [`crate::Engine::decode_buffer`] or [`crate::Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer::<CAP>(input)`
    /// instead.
    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        STANDARD.decode_buffer(input)
    }
}

impl<const CAP: usize, const N: usize> TryFrom<&[u8; N]> for DecodedBuffer<CAP> {
    type Error = DecodeError;

    /// Decodes a strict standard padded Base64 byte array into a stack-backed
    /// buffer.
    ///
    /// Use [`crate::Engine::decode_buffer`] or [`crate::Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer::<CAP>(input)`
    /// instead.
    fn try_from(input: &[u8; N]) -> Result<Self, Self::Error> {
        Self::try_from(&input[..])
    }
}

impl<const CAP: usize> TryFrom<&str> for DecodedBuffer<CAP> {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 text into a stack-backed buffer.
    ///
    /// Use [`crate::Engine::decode_buffer`] or [`crate::Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer::<CAP>(input)`
    /// instead.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

impl<const CAP: usize> core::str::FromStr for DecodedBuffer<CAP> {
    type Err = DecodeError;

    /// Decodes strict standard padded Base64 text into a stack-backed buffer.
    ///
    /// Use [`crate::Engine::decode_buffer`] or [`crate::Profile::decode_buffer`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer::<CAP>(input)`
    /// instead.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::try_from(input)
    }
}

/// Owned sensitive bytes with redacted formatting and drop-time cleanup.
///
/// `SecretBuffer` is available with the `alloc` feature. It is intended for
/// decoded keys, tokens, and other values that should not be accidentally
/// logged. The buffer exposes contents only through explicit reveal methods.
///
/// Spare vector capacity is cleared when wrapping owned bytes. On drop,
/// initialized bytes and vector spare capacity are cleared with the crate's
/// internal best-effort wipe helpers. This is data-retention reduction, not a
/// formal zeroization guarantee, and it cannot make claims about allocator
/// behavior or historical copies outside the wrapper.
///
/// # Platform Memory Controls
///
/// `SecretBuffer` does not lock its allocation into physical memory. The OS
/// may page its contents to disk, include them in hibernation images, or expose
/// them through crash dumps. High-assurance deployments must combine
/// `SecretBuffer` with platform memory-locking where available, encrypted or
/// disabled swap, crash-dump suppression, and allocator isolation appropriate
/// for their environment.
///
/// On `wasm32` targets, the wipe barrier uses only a compiler fence. The wasm
/// runtime JIT may still optimize or retain cleared bytes in ways this crate
/// cannot control. `wasm32` builds fail closed by default; enable
/// `allow-wasm32-best-effort-wipe` only when the deployment explicitly accepts
/// this limitation and applies its own memory strategy around owned secret
/// buffers.
#[cfg(feature = "alloc")]
pub struct SecretBuffer {
    bytes: alloc::vec::Vec<u8>,
}

/// Owned secret bytes extracted from [`SecretBuffer`].
///
/// This wrapper keeps redacted formatting, best-effort spare-capacity clearing
/// at construction time, and best-effort full wipe on drop after a
/// [`SecretBuffer`] is consumed for owned interop. Use
/// [`Self::into_exposed_unprotected_vec_caller_must_zeroize`] only when a raw
/// `Vec<u8>` is unavoidable and the caller will handle cleanup.
#[cfg(feature = "alloc")]
pub struct ExposedSecretVec {
    bytes: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl ExposedSecretVec {
    /// Wraps an owned vector as exposed secret material.
    #[must_use]
    pub fn from_vec(mut bytes: alloc::vec::Vec<u8>) -> Self {
        wipe_vec_spare_capacity(&mut bytes);
        Self { bytes }
    }

    /// Returns the number of initialized secret bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the buffer contains no initialized secret bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Reveals the secret bytes.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.bytes
    }

    /// Reveals the secret bytes mutably.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Consumes the wrapper and returns a raw `Vec<u8>`.
    ///
    /// This is an unprotected escape hatch. The returned vector is no longer
    /// redacted by formatting and will not be cleared by this crate on drop.
    /// Callers must clear it with their own approved zeroization policy.
    #[must_use = "caller must zeroize the returned Vec"]
    pub fn into_exposed_unprotected_vec_caller_must_zeroize(mut self) -> alloc::vec::Vec<u8> {
        core::mem::take(&mut self.bytes)
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for ExposedSecretVec {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExposedSecretVec")
            .field("bytes", &"<redacted>")
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Display for ExposedSecretVec {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(feature = "alloc")]
impl Drop for ExposedSecretVec {
    fn drop(&mut self) {
        wipe_vec_all(&mut self.bytes);
    }
}

#[cfg(feature = "alloc")]
struct WipeVecGuard {
    bytes: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl WipeVecGuard {
    fn from_vec(bytes: alloc::vec::Vec<u8>) -> Self {
        Self { bytes }
    }

    fn into_validated_secret_string(mut self) -> alloc::string::String {
        wipe_vec_spare_capacity(&mut self.bytes);
        let bytes = core::mem::take(&mut self.bytes);
        string_from_validated_secret_bytes(bytes)
    }
}

#[cfg(feature = "alloc")]
impl Drop for WipeVecGuard {
    fn drop(&mut self) {
        wipe_vec_all(&mut self.bytes);
    }
}

#[cfg(feature = "alloc")]
impl AsRef<[u8]> for ExposedSecretVec {
    fn as_ref(&self) -> &[u8] {
        self.expose_secret()
    }
}

#[cfg(feature = "alloc")]
impl AsMut<[u8]> for ExposedSecretVec {
    fn as_mut(&mut self) -> &mut [u8] {
        self.expose_secret_mut()
    }
}

/// Owned secret UTF-8 text extracted from [`SecretBuffer`].
///
/// This wrapper keeps redacted formatting, best-effort spare-capacity clearing
/// at construction time, and best-effort full wipe on drop after a
/// [`SecretBuffer`] is consumed for string interop. Use
/// [`Self::into_exposed_unprotected_string_caller_must_zeroize`] only when a
/// raw `String` is unavoidable and the caller will handle cleanup.
#[cfg(feature = "alloc")]
pub struct ExposedSecretString {
    text: alloc::string::String,
}

#[cfg(feature = "alloc")]
impl ExposedSecretString {
    /// Wraps an owned UTF-8 string as exposed secret text.
    #[must_use]
    pub fn from_string(text: alloc::string::String) -> Self {
        let mut bytes = text.into_bytes();
        wipe_vec_spare_capacity(&mut bytes);
        let text = string_from_validated_secret_bytes(bytes);
        Self { text }
    }

    /// Returns the length of the secret text in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Returns whether the secret text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Reveals the secret text.
    ///
    /// This method is intentionally named to make secret access explicit at
    /// the call site.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.text
    }

    /// Reveals the secret text as bytes.
    ///
    /// This method is intentionally named to make secret access explicit at
    /// the call site.
    #[must_use]
    pub fn expose_secret_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    /// Consumes the wrapper and returns a raw `String`.
    ///
    /// This is an unprotected escape hatch. The returned string is no longer
    /// redacted by formatting and will not be cleared by this crate on drop.
    /// Callers must clear it with their own approved zeroization policy.
    #[must_use = "caller must zeroize the returned String"]
    pub fn into_exposed_unprotected_string_caller_must_zeroize(mut self) -> alloc::string::String {
        core::mem::take(&mut self.text)
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for ExposedSecretString {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExposedSecretString")
            .field("text", &"<redacted>")
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Display for ExposedSecretString {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(feature = "alloc")]
impl Drop for ExposedSecretString {
    fn drop(&mut self) {
        let mut bytes = core::mem::take(&mut self.text).into_bytes();
        wipe_vec_all(&mut bytes);
    }
}

#[cfg(feature = "alloc")]
impl AsRef<str> for ExposedSecretString {
    fn as_ref(&self) -> &str {
        self.expose_secret()
    }
}

#[cfg(feature = "alloc")]
impl SecretBuffer {
    /// Wraps an existing vector as sensitive material.
    #[must_use]
    pub fn from_vec(mut bytes: alloc::vec::Vec<u8>) -> Self {
        wipe_vec_spare_capacity(&mut bytes);
        Self { bytes }
    }

    /// Copies a slice into an owned sensitive buffer.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self::from_vec(bytes.to_vec())
    }

    /// Returns the number of initialized secret bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the buffer contains no initialized secret bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Reveals the secret bytes.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.bytes
    }

    /// Reveals the secret bytes as UTF-8 text.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site. Secret material may be arbitrary binary data, so this method
    /// is fallible.
    pub fn expose_secret_utf8(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.expose_secret())
    }

    /// Reveals the secret bytes mutably.
    ///
    /// This method is intentionally named to make secret access explicit at the
    /// call site.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Consumes the wrapper and returns owned secret bytes.
    ///
    /// This is an explicit escape hatch for interop with APIs that require an
    /// owned vector-like value. The returned [`ExposedSecretVec`] remains
    /// redacted by formatting and clears its vector on drop.
    #[must_use]
    pub fn into_exposed_vec(mut self) -> ExposedSecretVec {
        ExposedSecretVec::from_vec(core::mem::take(&mut self.bytes))
    }

    /// Consumes the wrapper and returns the owned secret bytes as UTF-8 text.
    ///
    /// This is an explicit escape hatch for interop with APIs that require an
    /// owned string-like value. The returned [`ExposedSecretString`] remains
    /// redacted by formatting and clears its heap allocation on drop.
    ///
    /// If the secret bytes are not valid UTF-8, the original redacted wrapper
    /// is returned unchanged.
    #[must_use = "handle invalid UTF-8 errors and keep the returned wrapper protected"]
    pub fn try_into_exposed_string(self) -> Result<ExposedSecretString, Self> {
        if core::str::from_utf8(self.expose_secret()).is_err() {
            return Err(self);
        }

        // Keep the bytes behind a wiping guard until the final infallible
        // ownership transfer into `String`.
        let mut exposed = self.into_exposed_vec();
        let guard = WipeVecGuard::from_vec(core::mem::take(&mut exposed.bytes));
        drop(exposed);
        Ok(ExposedSecretString::from_string(
            guard.into_validated_secret_string(),
        ))
    }

    /// Compares this secret to `other` without short-circuiting on the first
    /// differing byte.
    ///
    /// Length and the final equality result remain public. Different lengths
    /// return `false` immediately; use this helper only when the compared
    /// lengths are public protocol facts or have been normalized by the
    /// caller. For equal-length inputs, this helper scans every byte before
    /// returning. It is constant-time-oriented best effort, not a formal
    /// cryptographic constant-time guarantee. This comparison is deliberately
    /// explicit: redacted buffer types do not implement [`PartialEq`] because
    /// `==` would make a best-effort helper look like a formal token/MAC
    /// comparison primitive.
    ///
    /// Do not use this helper as the sole MAC, bearer-token, password-hash, or
    /// authentication-secret comparison primitive in high-assurance systems.
    /// Applications that can admit dependencies should use a reviewed
    /// constant-time comparison primitive, such as `subtle`, at the protocol
    /// boundary.
    #[doc(alias = "constant_time_eq")]
    #[must_use]
    pub fn constant_time_eq_public_len(&self, other: &[u8]) -> bool {
        constant_time_eq_public_len(self.expose_secret(), other)
    }

    /// Clears the initialized bytes and makes the buffer empty.
    pub fn clear(&mut self) {
        wipe_vec_all(&mut self.bytes);
        self.bytes.clear();
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for SecretBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretBuffer")
            .field("bytes", &"<redacted>")
            .field("len", &self.len())
            .finish()
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Display for SecretBuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(feature = "alloc")]
impl Drop for SecretBuffer {
    fn drop(&mut self) {
        wipe_vec_all(&mut self.bytes);
    }
}

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
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer(input)` or
    /// [`crate::ct::STANDARD`].`decode_slice_staged_clear_tail(...)` and then
    /// wrap the successful output in `SecretBuffer`.
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
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer(input)` or
    /// [`crate::ct::STANDARD`].`decode_slice_staged_clear_tail(...)` and then
    /// wrap the successful output in `SecretBuffer`.
    fn try_from(input: &[u8; N]) -> Result<Self, Self::Error> {
        Self::try_from(&input[..])
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&str> for SecretBuffer {
    type Error = DecodeError;

    /// Decodes strict standard padded Base64 text into a redacted owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer(input)` or
    /// [`crate::ct::STANDARD`].`decode_slice_staged_clear_tail(...)` and then
    /// wrap the successful output in `SecretBuffer`.
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::try_from(input.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl core::str::FromStr for SecretBuffer {
    type Err = DecodeError;

    /// Decodes strict standard padded Base64 text into a redacted owned buffer.
    ///
    /// Use [`crate::Engine::decode_secret`] or [`crate::Profile::decode_secret`] when a
    /// different alphabet, padding mode, or line-wrapping profile is required.
    ///
    /// # Security
    ///
    /// This idiomatic conversion uses the strict standard decoder, not the
    /// constant-time-oriented decoder. It may branch or return early on
    /// malformed input and reports exact [`DecodeError`] positions. For
    /// secret-bearing tokens or key material where malformed-input timing
    /// matters, use [`crate::ct::STANDARD`].`decode_buffer(input)` or
    /// [`crate::ct::STANDARD`].`decode_slice_staged_clear_tail(...)` and then
    /// wrap the successful output in `SecretBuffer`.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::try_from(input)
    }
}

#[cfg(feature = "alloc")]
fn string_from_validated_secret_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(string) => string,
        Err(error) => {
            let mut bytes = error.into_bytes();
            wipe_vec_all(&mut bytes);
            String::new()
        }
    }
}
