//! Owned validated alphabets for the 2.0 codec core.
//!
//! This module is internal until Commit 6 exposes validated codec
//! specifications. The value contains only its immutable 64-byte table; it
//! cannot carry caller-provided mapping functions.

/// The number of symbols in every Base64 alphabet.
pub(crate) const ALPHABET_LEN: usize = 64;

/// The source-locked RFC 4648 Standard alphabet.
pub(super) const STANDARD_ALPHABET: ValidatedAlphabet = ValidatedAlphabet {
    table: *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
};
/// The source-locked RFC 4648 URL-safe alphabet.
pub(super) const URL_SAFE_ALPHABET: ValidatedAlphabet = ValidatedAlphabet {
    table: *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
};

/// Failure returned while constructing a [`ValidatedAlphabet`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedAlphabetError {
    /// A byte slice did not contain exactly 64 bytes.
    InvalidLength {
        /// Observed byte length.
        actual: usize,
    },
    /// An alphabet position contains a byte outside visible ASCII.
    InvalidByte {
        /// Byte index in the alphabet table.
        index: usize,
        /// Rejected byte value.
        byte: u8,
    },
    /// An alphabet position contains the reserved padding byte `=`.
    PaddingByte {
        /// Byte index in the alphabet table.
        index: usize,
    },
    /// Two alphabet positions contain the same byte.
    DuplicateByte {
        /// First index containing the byte.
        first: usize,
        /// Second index containing the byte.
        second: usize,
        /// Duplicated byte value.
        byte: u8,
    },
}

impl core::fmt::Display for ValidatedAlphabetError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength { actual } => {
                write!(
                    formatter,
                    "base64 alphabet has length {actual}; expected {ALPHABET_LEN}"
                )
            }
            Self::InvalidByte { index, byte } => {
                write!(
                    formatter,
                    "invalid base64 alphabet byte 0x{byte:02x} at index {index}"
                )
            }
            Self::PaddingByte { index } => {
                write!(
                    formatter,
                    "base64 alphabet contains padding byte at index {index}"
                )
            }
            Self::DuplicateByte {
                first,
                second,
                byte,
            } => write!(
                formatter,
                "base64 alphabet byte 0x{byte:02x} is duplicated at indexes \
                 {first} and {second}"
            ),
        }
    }
}

/// An owned, immutable, validated 64-byte Base64 alphabet.
///
/// Encode and decode mappings are both derived from `table`. The type has no
/// executable callback or overridable mapping method.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ValidatedAlphabet {
    table: [u8; ALPHABET_LEN],
}

impl ValidatedAlphabet {
    /// Validates and owns a 64-byte alphabet.
    pub const fn new(table: [u8; ALPHABET_LEN]) -> Result<Self, ValidatedAlphabetError> {
        match validate_table(&table) {
            Ok(()) => Ok(Self { table }),
            Err(error) => Err(error),
        }
    }

    /// Copies, validates, and owns a 64-byte alphabet slice.
    pub const fn try_from_slice(bytes: &[u8]) -> Result<Self, ValidatedAlphabetError> {
        if bytes.len() != ALPHABET_LEN {
            return Err(ValidatedAlphabetError::InvalidLength {
                actual: bytes.len(),
            });
        }

        let mut table = [0u8; ALPHABET_LEN];
        let mut index = 0;
        while index < ALPHABET_LEN {
            table[index] = bytes[index];
            index += 1;
        }
        Self::new(table)
    }

    /// Returns the single table that defines both mappings.
    #[must_use]
    pub const fn as_array(&self) -> &[u8; ALPHABET_LEN] {
        &self.table
    }

    /// Returns the encoded symbol for one six-bit value.
    #[allow(clippy::cast_lossless)]
    #[must_use]
    pub const fn encode_value(&self, value: u8) -> Option<u8> {
        if value < 64 {
            Some(self.table[value as usize])
        } else {
            None
        }
    }

    /// Returns the six-bit value represented by `byte`.
    #[must_use]
    pub const fn decode_byte(&self, byte: u8) -> Option<u8> {
        let mut index = 0;
        let mut candidate = 0u8;
        while index < ALPHABET_LEN {
            if self.table[index] == byte {
                return Some(candidate);
            }
            index += 1;
            candidate += 1;
        }
        None
    }
}

impl TryFrom<[u8; ALPHABET_LEN]> for ValidatedAlphabet {
    type Error = ValidatedAlphabetError;

    fn try_from(table: [u8; ALPHABET_LEN]) -> Result<Self, Self::Error> {
        Self::new(table)
    }
}

impl TryFrom<&[u8]> for ValidatedAlphabet {
    type Error = ValidatedAlphabetError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(bytes)
    }
}

const fn validate_table(table: &[u8; ALPHABET_LEN]) -> Result<(), ValidatedAlphabetError> {
    let mut index = 0;
    while index < ALPHABET_LEN {
        match validate_position(table, index) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
        index += 1;
    }
    Ok(())
}

const fn validate_position(
    table: &[u8; ALPHABET_LEN],
    index: usize,
) -> Result<(), ValidatedAlphabetError> {
    let byte = table[index];
    if byte < 0x21 || byte > 0x7e {
        return Err(ValidatedAlphabetError::InvalidByte { index, byte });
    }
    if byte == b'=' {
        return Err(ValidatedAlphabetError::PaddingByte { index });
    }

    let mut duplicate = index + 1;
    while duplicate < ALPHABET_LEN {
        if table[duplicate] == byte {
            return Err(ValidatedAlphabetError::DuplicateByte {
                first: index,
                second: duplicate,
                byte,
            });
        }
        duplicate += 1;
    }
    Ok(())
}

/// Exposes one constructor validation step to the bounded-index Kani harness.
#[cfg(kani)]
pub(crate) fn validate_position_for_proof(
    table: &[u8; ALPHABET_LEN],
    index: usize,
) -> Result<(), ValidatedAlphabetError> {
    validate_position(table, index)
}
