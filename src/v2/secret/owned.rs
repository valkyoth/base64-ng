//! Owned fixed and heap secret storage.

use super::{ExposedSecret, ExposedSecretMut};
use crate::v2::bounded::BufferLengthError;

/// Non-Clone bounded secret bytes with mandatory full-capacity cleanup.
pub struct SecretArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> SecretArray<CAP> {
    /// Takes ownership, checks the visible prefix, and wipes unused capacity.
    pub fn from_array(mut bytes: [u8; CAP], len: usize) -> Result<Self, BufferLengthError> {
        if len > CAP {
            crate::wipe_bytes(&mut bytes);
            return Err(BufferLengthError::new(len, CAP));
        }
        crate::wipe_tail(&mut bytes, len);
        Ok(Self { bytes, len })
    }

    /// Creates an explicit borrowed interoperability view.
    #[must_use]
    pub fn expose_secret(&self) -> ExposedSecret<'_> {
        ExposedSecret::new(&self.bytes[..self.len])
    }

    /// Creates an explicit mutable interoperability view.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> ExposedSecretMut<'_> {
        ExposedSecretMut {
            bytes: &mut self.bytes[..self.len],
        }
    }

    /// Deliberately converts this secret into ordinary non-wiping storage.
    #[must_use = "declassification transfers cleanup responsibility to the caller"]
    pub fn declassify(mut self) -> DeclassifiedArray<CAP> {
        let bytes = core::mem::replace(&mut self.bytes, [0u8; CAP]);
        let len = self.len;
        self.len = 0;
        DeclassifiedArray { bytes, len }
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

    /// Returns the public fixed capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Wipes the complete backing array and resets the visible length.
    pub fn clear(&mut self) {
        crate::wipe_bytes(&mut self.bytes);
        self.len = 0;
    }

    #[cfg(test)]
    pub(crate) const fn backing_for_test(&self) -> &[u8; CAP] {
        &self.bytes
    }
}

impl<const CAP: usize> Drop for SecretArray<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<const CAP: usize> core::fmt::Debug for SecretArray<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretArray")
            .field("bytes", &"<redacted>")
            .field("len", &self.len)
            .field("capacity", &CAP)
            .finish()
    }
}

impl<const CAP: usize> core::fmt::Display for SecretArray<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("<redacted secret array>")
    }
}

/// Ordinary fixed array created by explicit secret declassification.
///
/// This value is `Copy` and deliberately performs no cleanup on drop.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct DeclassifiedArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> DeclassifiedArray<CAP> {
    /// Returns the ordinary initialized prefix.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
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

    /// Returns the fixed capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the complete ordinary array and initialized length.
    #[must_use]
    pub const fn into_parts(self) -> ([u8; CAP], usize) {
        (self.bytes, self.len)
    }
}

impl<const CAP: usize> AsRef<[u8]> for DeclassifiedArray<CAP> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const CAP: usize> core::fmt::Debug for DeclassifiedArray<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("DeclassifiedArray")
            .field(&self.as_bytes())
            .finish()
    }
}

/// Owned heap secret bytes with initialized and spare-capacity cleanup.
#[cfg(feature = "alloc")]
pub struct SecretVec {
    bytes: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl SecretVec {
    /// Takes ownership and wipes the vector's spare capacity.
    #[must_use]
    pub fn from_vec(mut bytes: alloc::vec::Vec<u8>) -> Self {
        crate::wipe_vec_spare_capacity(&mut bytes);
        Self { bytes }
    }

    /// Copies caller-owned bytes into secret storage.
    #[must_use]
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self::from_vec(bytes.to_vec())
    }

    /// Creates an explicit borrowed interoperability view.
    #[must_use]
    pub fn expose_secret(&self) -> ExposedSecret<'_> {
        ExposedSecret::new(&self.bytes)
    }

    /// Creates an explicit mutable interoperability view.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> ExposedSecretMut<'_> {
        ExposedSecretMut {
            bytes: &mut self.bytes,
        }
    }

    /// Deliberately returns an ordinary vector without crate-managed cleanup.
    #[must_use = "caller must apply its approved cleanup policy to the returned Vec"]
    pub fn declassify_into_unprotected_vec(mut self) -> alloc::vec::Vec<u8> {
        core::mem::take(&mut self.bytes)
    }

    /// Returns the public initialized length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns whether the initialized prefix is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the public allocation capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Wipes initialized bytes and spare capacity, then resets the length.
    pub fn clear(&mut self) {
        crate::wipe_vec_all(&mut self.bytes);
        self.bytes.clear();
    }
}

#[cfg(feature = "alloc")]
impl Drop for SecretVec {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(feature = "alloc")]
redacted_formatting!(SecretVec, "SecretVec");
