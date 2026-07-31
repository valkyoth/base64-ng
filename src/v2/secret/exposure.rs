//! Borrowed secret input, output, exposure, and declassification views.

use super::super::bounded::BufferLengthError;

/// Explicit borrowed interoperability view of secret bytes.
///
/// Construct this value only through an `expose_secret` method. Formatting
/// remains redacted, but the explicit [`Self::as_bytes`] method and standard
/// slice coercion traits make deliberate interoperability convenient.
pub struct ExposedSecret<'a> {
    bytes: &'a [u8],
}

impl<'a> ExposedSecret<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Returns the deliberately exposed bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Returns the public byte length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the exposed view is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl AsRef<[u8]> for ExposedSecret<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl core::ops::Deref for ExposedSecret<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.bytes
    }
}

redacted_formatting!(ExposedSecret<'_>, "ExposedSecret");

/// Explicit mutable interoperability view of secret bytes.
///
/// The originating secret owner retains cleanup responsibility.
pub struct ExposedSecretMut<'a> {
    pub(super) bytes: &'a mut [u8],
}

impl ExposedSecretMut<'_> {
    /// Returns the deliberately exposed bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Returns the deliberately exposed mutable bytes.
    #[must_use]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.bytes
    }

    /// Returns the public byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the exposed view is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl AsRef<[u8]> for ExposedSecretMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl AsMut<[u8]> for ExposedSecretMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.bytes
    }
}

impl core::ops::Deref for ExposedSecretMut<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.bytes
    }
}

impl core::ops::DerefMut for ExposedSecretMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bytes
    }
}

redacted_formatting!(ExposedSecretMut<'_>, "ExposedSecretMut");

/// Non-Clone borrowed input classified as secret-bearing.
///
/// This wrapper does not own or wipe the borrowed bytes. It prevents implicit
/// passage into ordinary codecs; callers must deliberately call
/// [`Self::expose_secret`] to obtain an interoperability view.
pub struct SecretInput<'a> {
    bytes: &'a [u8],
}

impl<'a> SecretInput<'a> {
    /// Classifies caller-owned bytes as secret input.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Creates an explicit borrowed interoperability view.
    #[must_use]
    pub const fn expose_secret(&self) -> ExposedSecret<'_> {
        ExposedSecret::new(self.bytes)
    }

    /// Returns the public input length without exposing bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the input is empty without exposing bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

redacted_formatting!(SecretInput<'_>, "SecretInput");

/// Borrowed secret output with full-range cleanup on drop.
///
/// Construction wipes unused tail bytes. Drop wipes the complete borrowed
/// range. A consuming [`Self::declassify`] call deliberately transfers the
/// bytes into an ordinary non-wiping view.
pub struct SecretOutput<'a> {
    storage: &'a mut [u8],
    len: usize,
}

impl<'a> SecretOutput<'a> {
    /// Wraps one initialized prefix and assumes cleanup responsibility.
    ///
    /// Invalid lengths wipe the complete range before returning an error.
    pub fn from_initialized(storage: &'a mut [u8], len: usize) -> Result<Self, BufferLengthError> {
        let capacity = storage.len();
        if len > capacity {
            crate::wipe_bytes(storage);
            return Err(BufferLengthError::new(len, capacity));
        }
        crate::wipe_tail(storage, len);
        Ok(Self { storage, len })
    }

    /// Creates an empty output guard after wiping the complete range.
    #[must_use]
    pub fn empty(storage: &'a mut [u8]) -> Self {
        crate::wipe_bytes(storage);
        Self { storage, len: 0 }
    }

    /// Creates an explicit borrowed interoperability view.
    #[must_use]
    pub fn expose_secret(&self) -> ExposedSecret<'_> {
        ExposedSecret::new(&self.storage[..self.len])
    }

    /// Creates an explicit mutable interoperability view.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> ExposedSecretMut<'_> {
        ExposedSecretMut {
            bytes: &mut self.storage[..self.len],
        }
    }

    /// Deliberately transfers the initialized prefix into ordinary storage.
    ///
    /// The returned view does not wipe on drop. Its tail remains zeroed, and
    /// the caller becomes responsible for any later cleanup requirement.
    #[must_use = "declassification transfers cleanup responsibility to the caller"]
    pub fn declassify(mut self) -> DeclassifiedOutput<'a> {
        let storage = core::mem::take(&mut self.storage);
        let len = self.len;
        self.len = 0;
        DeclassifiedOutput { storage, len }
    }

    /// Returns the public initialized length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the public borrowed capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    /// Returns whether the initialized prefix is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Wipes the complete range and resets the initialized length.
    pub fn clear(&mut self) {
        crate::wipe_bytes(self.storage);
        self.len = 0;
    }
}

impl Drop for SecretOutput<'_> {
    fn drop(&mut self) {
        self.clear();
    }
}

redacted_formatting!(SecretOutput<'_>, "SecretOutput");

/// Ordinary borrowed output created by explicit declassification.
///
/// This value deliberately performs no cleanup on drop.
pub struct DeclassifiedOutput<'a> {
    storage: &'a mut [u8],
    len: usize,
}

impl<'a> DeclassifiedOutput<'a> {
    /// Returns the ordinary initialized prefix.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.storage[..self.len]
    }

    /// Returns the ordinary initialized prefix mutably.
    #[must_use]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.storage[..self.len]
    }

    /// Returns the public initialized length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the initialized prefix is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the complete caller-owned range and initialized length.
    #[must_use]
    pub fn into_parts(self) -> (&'a mut [u8], usize) {
        (self.storage, self.len)
    }
}

impl AsRef<[u8]> for DeclassifiedOutput<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsMut<[u8]> for DeclassifiedOutput<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_bytes_mut()
    }
}
