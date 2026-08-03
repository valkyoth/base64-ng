use crate::{PasswordRecordError, PasswordRecordErrorKind};

/// Exact Passlib PBKDF2 modular-crypt format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PasslibPbkdf2Algorithm {
    /// `$pbkdf2$` with a 20-byte SHA-1-derived checksum.
    Sha1,
    /// `$pbkdf2-sha256$` with a 32-byte checksum.
    Sha256,
    /// `$pbkdf2-sha512$` with a 64-byte checksum.
    Sha512,
}

impl PasslibPbkdf2Algorithm {
    pub(crate) const fn prefix(self) -> &'static [u8] {
        match self {
            Self::Sha1 => b"$pbkdf2$",
            Self::Sha256 => b"$pbkdf2-sha256$",
            Self::Sha512 => b"$pbkdf2-sha512$",
        }
    }

    /// Returns the required decoded checksum length.
    #[must_use]
    pub const fn checksum_len(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }

    /// Returns the exact encoded checksum length.
    #[must_use]
    pub const fn encoded_checksum_len(self) -> usize {
        match self {
            Self::Sha1 => 27,
            Self::Sha256 => 43,
            Self::Sha512 => 86,
        }
    }
}

/// Exact SHA-crypt modular-crypt format.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShaCryptAlgorithm {
    /// `$5$` SHA-256 crypt with a 32-byte digest and 43-character checksum.
    Sha256,
    /// `$6$` SHA-512 crypt with a 64-byte digest and 86-character checksum.
    Sha512,
}

impl ShaCryptAlgorithm {
    pub(crate) const fn prefix(self) -> &'static [u8] {
        match self {
            Self::Sha256 => b"$5$",
            Self::Sha512 => b"$6$",
        }
    }

    /// Returns the raw digest length.
    #[must_use]
    pub const fn digest_len(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha512 => 64,
        }
    }

    /// Returns the exact encoded checksum length.
    #[must_use]
    pub const fn encoded_checksum_len(self) -> usize {
        match self {
            Self::Sha256 => 43,
            Self::Sha512 => 86,
        }
    }
}

/// SHA-crypt rounds value and whether it is explicit in the record.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ShaCryptRounds {
    value: u32,
    explicit: bool,
}

impl ShaCryptRounds {
    /// The implicit SHA-crypt default of 5,000 rounds.
    #[must_use]
    pub const fn implicit() -> Self {
        Self {
            value: 5000,
            explicit: false,
        }
    }

    /// Builds an explicit canonical rounds field.
    ///
    /// # Errors
    ///
    /// Returns [`PasswordRecordErrorKind::InvalidRounds`] unless `value` is in
    /// the inclusive SHA-crypt range 1,000 through 999,999,999.
    pub const fn explicit(value: u32) -> Result<Self, PasswordRecordError> {
        if value < 1000 || value > 999_999_999 {
            Err(PasswordRecordError::new(
                PasswordRecordErrorKind::InvalidRounds,
            ))
        } else {
            Ok(Self {
                value,
                explicit: true,
            })
        }
    }

    /// Returns the effective rounds value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.value
    }

    /// Returns whether `rounds=N$` is present in the record.
    #[must_use]
    pub const fn is_explicit(self) -> bool {
        self.explicit
    }
}

/// Borrowed, validated Passlib PBKDF2 record.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PasslibPbkdf2Record<'a> {
    pub(crate) algorithm: PasslibPbkdf2Algorithm,
    pub(crate) rounds: u32,
    pub(crate) salt: &'a [u8],
    pub(crate) checksum: &'a [u8],
}

impl PasslibPbkdf2Record<'_> {
    /// Returns the exact record algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> PasslibPbkdf2Algorithm {
        self.algorithm
    }

    /// Returns the positive canonical rounds value.
    #[must_use]
    pub const fn rounds(&self) -> u32 {
        self.rounds
    }

    /// Explicitly exposes the encoded salt field.
    #[must_use]
    pub const fn expose_encoded_salt(&self) -> &[u8] {
        self.salt
    }

    /// Explicitly exposes the encoded checksum field.
    #[must_use]
    pub const fn expose_encoded_checksum(&self) -> &[u8] {
        self.checksum
    }
}

impl core::fmt::Debug for PasslibPbkdf2Record<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("PasslibPbkdf2Record")
            .field("algorithm", &self.algorithm)
            .field("rounds", &self.rounds)
            .field("salt", &"[REDACTED]")
            .field("salt_encoded_len", &self.salt.len())
            .field("checksum", &"[REDACTED]")
            .field("checksum_encoded_len", &self.checksum.len())
            .finish()
    }
}

/// Borrowed, validated SHA-crypt record.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ShaCryptRecord<'a> {
    pub(crate) algorithm: ShaCryptAlgorithm,
    pub(crate) rounds: ShaCryptRounds,
    pub(crate) salt: &'a [u8],
    pub(crate) checksum: &'a [u8],
}

impl ShaCryptRecord<'_> {
    /// Returns the exact SHA-crypt algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> ShaCryptAlgorithm {
        self.algorithm
    }

    /// Returns the rounds value and explicitness.
    #[must_use]
    pub const fn rounds(&self) -> ShaCryptRounds {
        self.rounds
    }

    /// Explicitly exposes the SHA-crypt salt text.
    #[must_use]
    pub const fn expose_salt(&self) -> &[u8] {
        self.salt
    }

    /// Explicitly exposes the encoded SHA-crypt checksum.
    #[must_use]
    pub const fn expose_encoded_checksum(&self) -> &[u8] {
        self.checksum
    }
}

impl core::fmt::Debug for ShaCryptRecord<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ShaCryptRecord")
            .field("algorithm", &self.algorithm)
            .field("rounds", &self.rounds)
            .field("salt", &"[REDACTED]")
            .field("salt_len", &self.salt.len())
            .field("checksum", &"[REDACTED]")
            .field("checksum_encoded_len", &self.checksum.len())
            .finish()
    }
}
