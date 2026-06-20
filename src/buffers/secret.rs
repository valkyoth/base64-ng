use crate::{constant_time_eq_public_len, wipe_vec_all, wipe_vec_spare_capacity};
use alloc::{string::String, vec::Vec};

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

    fn into_validated_secret_string(
        mut self,
    ) -> Result<alloc::string::String, alloc::vec::Vec<u8>> {
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
        let text = match string_from_validated_secret_bytes(bytes) {
            Ok(text) => text,
            Err(mut bytes) => {
                wipe_vec_all(&mut bytes);
                alloc::string::String::new()
            }
        };
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
        match guard.into_validated_secret_string() {
            Ok(text) => Ok(ExposedSecretString::from_string(text)),
            Err(bytes) => Err(SecretBuffer::from_vec(bytes)),
        }
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
fn string_from_validated_secret_bytes(bytes: Vec<u8>) -> Result<String, Vec<u8>> {
    String::from_utf8(bytes).map_err(alloc::string::FromUtf8Error::into_bytes)
}
