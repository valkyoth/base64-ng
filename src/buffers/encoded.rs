use crate::{EncodeError, STANDARD, constant_time_eq_public_len, wipe_bytes, wipe_tail};

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
