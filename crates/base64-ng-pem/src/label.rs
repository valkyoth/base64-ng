use alloc::string::String;

/// A syntactically valid RFC 7468 label.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PemLabel(String);

impl PemLabel {
    /// Validates and owns one RFC 7468 label.
    ///
    /// The RFC ABNF permits an empty label. Canonical generation additionally
    /// requires ASCII uppercase for alphabetic characters.
    ///
    /// # Errors
    ///
    /// Returns [`PemLabelError`] for non-ASCII bytes, disallowed control
    /// bytes, invalid space/hyphen separator placement, or allocation
    /// failure.
    pub fn new(label: &str) -> Result<Self, PemLabelError> {
        validate_label(label.as_bytes())?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(label.len())
            .map_err(|_| PemLabelError::AllocationFailed)?;
        owned.push_str(label);
        Ok(Self(owned))
    }

    /// Returns the validated label text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether every alphabetic byte is uppercase.
    #[must_use]
    pub fn is_canonical_uppercase(&self) -> bool {
        !self.0.bytes().any(|byte| byte.is_ascii_lowercase())
    }
}

impl AsRef<str> for PemLabel {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Label validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PemLabelError {
    /// A byte is outside printable US-ASCII or violates separator placement.
    InvalidSyntax,
    /// Owning the validated label could not reserve memory.
    AllocationFailed,
}

impl core::fmt::Display for PemLabelError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSyntax => formatter.write_str("invalid RFC 7468 label syntax"),
            Self::AllocationFailed => formatter.write_str("RFC 7468 label allocation failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PemLabelError {}

fn validate_label(bytes: &[u8]) -> Result<(), PemLabelError> {
    if bytes.is_empty() {
        return Ok(());
    }
    let mut previous_separator = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let separator = matches!(byte, b'-' | b' ');
        let label_char = matches!(byte, 0x21..=0x2c | 0x2e..=0x7e);
        if (!separator && !label_char)
            || (separator && (index == 0 || index + 1 == bytes.len() || previous_separator))
        {
            return Err(PemLabelError::InvalidSyntax);
        }
        previous_separator = separator;
    }
    Ok(())
}
