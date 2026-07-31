//! Bounded stack storage with explicit ordinary and secret contracts.

use super::{
    ordinary::OneShotError,
    specifications::{Base64, Codec},
};

/// Error returned when a visible prefix exceeds its backing array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferLengthError {
    length: usize,
    capacity: usize,
}

impl BufferLengthError {
    /// Returns the rejected visible length.
    #[must_use]
    pub const fn length(self) -> usize {
        self.length
    }

    /// Returns the backing array capacity.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }
}

impl core::fmt::Display for BufferLengthError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "visible buffer length {} exceeds capacity {}",
            self.length, self.capacity
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BufferLengthError {}

/// Ordinary bounded bytes intended to contain encoded text.
///
/// This ordinary value is `Copy`, has visible formatting, and performs no
/// drop-time cleanup. Use [`SecretArray`] for secret-bearing storage.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EncodedArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

/// Ordinary bounded decoded bytes.
///
/// This ordinary value is `Copy` and performs no drop-time cleanup. Its name
/// describes transform direction, not secrecy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DecodedArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

macro_rules! ordinary_array {
    ($name:ident, $description:literal) => {
        impl<const CAP: usize> $name<CAP> {
            #[doc = $description]
            pub const fn from_array(
                bytes: [u8; CAP],
                len: usize,
            ) -> Result<Self, BufferLengthError> {
                if len > CAP {
                    Err(BufferLengthError {
                        length: len,
                        capacity: CAP,
                    })
                } else {
                    Ok(Self { bytes, len })
                }
            }

            pub(crate) const fn from_initialized(
                bytes: [u8; CAP],
                len: usize,
            ) -> Result<Self, BufferLengthError> {
                Self::from_array(bytes, len)
            }

            /// Returns the initialized visible prefix.
            #[must_use]
            pub fn as_bytes(&self) -> &[u8] {
                &self.bytes[..self.len]
            }

            /// Returns the initialized prefix length.
            #[must_use]
            pub const fn len(&self) -> usize {
                self.len
            }

            /// Returns whether the initialized prefix is empty.
            #[must_use]
            pub const fn is_empty(&self) -> bool {
                self.len == 0
            }

            /// Returns the fixed backing capacity.
            #[must_use]
            pub const fn capacity(&self) -> usize {
                CAP
            }

            /// Returns the unused backing capacity.
            #[must_use]
            pub const fn remaining_capacity(&self) -> usize {
                CAP - self.len
            }

            /// Consumes the wrapper and returns its backing array and length.
            #[must_use]
            pub const fn into_parts(self) -> ([u8; CAP], usize) {
                (self.bytes, self.len)
            }
        }
    };
}

ordinary_array!(
    EncodedArray,
    "Constructs ordinary encoded storage with a checked visible prefix."
);
ordinary_array!(
    DecodedArray,
    "Constructs ordinary decoded storage with a checked visible prefix."
);

impl<S: Codec> Base64<S> {
    /// Encodes into a bounded ordinary stack array.
    pub fn encode_bounded<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<EncodedArray<CAP>, OneShotError> {
        let mut bytes = [0u8; CAP];
        let len = self.encode_into(input, &mut bytes)?;
        EncodedArray::from_initialized(bytes, len)
            .map_err(|_| OneShotError::Backend(super::contracts::BackendFault::ImpossibleState))
    }

    /// Decodes into a bounded ordinary stack array transactionally.
    pub fn decode_bounded<const CAP: usize>(
        &self,
        input: &[u8],
    ) -> Result<DecodedArray<CAP>, OneShotError> {
        let mut bytes = [0u8; CAP];
        let len = self.decode_into(input, &mut bytes)?;
        DecodedArray::from_initialized(bytes, len)
            .map_err(|_| OneShotError::Backend(super::contracts::BackendFault::ImpossibleState))
    }
}

/// Non-Clone bounded secret bytes with mandatory full-capacity cleanup.
///
/// `Debug` and `Display` are redacted. The initialized prefix is available
/// only through the explicitly named [`Self::expose_secret`] method. Cleanup
/// is best effort under the crate's documented wipe boundary and cannot erase
/// historical compiler, register, swap, or snapshot copies.
pub struct SecretArray<const CAP: usize> {
    bytes: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> SecretArray<CAP> {
    /// Takes ownership of secret bytes and checks their visible prefix length.
    ///
    /// Bytes after `len` are wiped before construction. If `len` exceeds
    /// `CAP`, the complete supplied array is wiped before returning an error.
    pub fn from_array(mut bytes: [u8; CAP], len: usize) -> Result<Self, BufferLengthError> {
        if len > CAP {
            crate::wipe_bytes(&mut bytes);
            return Err(BufferLengthError {
                length: len,
                capacity: CAP,
            });
        }
        crate::wipe_tail(&mut bytes, len);
        Ok(Self { bytes, len })
    }

    /// Explicitly exposes the initialized secret prefix.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the public initialized length.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the public initialized length is zero.
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
    pub(super) const fn backing_for_test(&self) -> &[u8; CAP] {
        &self.bytes
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

impl<const CAP: usize> Drop for SecretArray<CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}
