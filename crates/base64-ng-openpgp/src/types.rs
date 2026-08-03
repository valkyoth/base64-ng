use alloc::{string::String, vec::Vec};

use crate::{OpenPgpError, OpenPgpErrorKind};

/// One ordinary RFC 9580 ASCII armor type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArmorType {
    /// `OpenPGP` message packets.
    Message,
    /// Transferable public key packets.
    PublicKey,
    /// Transferable private key packets.
    PrivateKey,
    /// Detached signature packets.
    Signature,
}

impl ArmorType {
    /// Returns the exact RFC 9580 boundary label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Message => "PGP MESSAGE",
            Self::PublicKey => "PGP PUBLIC KEY BLOCK",
            Self::PrivateKey => "PGP PRIVATE KEY BLOCK",
            Self::Signature => "PGP SIGNATURE",
        }
    }

    pub(crate) fn from_label(label: &[u8]) -> Option<Self> {
        match label {
            b"PGP MESSAGE" => Some(Self::Message),
            b"PGP PUBLIC KEY BLOCK" => Some(Self::PublicKey),
            b"PGP PRIVATE KEY BLOCK" => Some(Self::PrivateKey),
            b"PGP SIGNATURE" => Some(Self::Signature),
            _ => None,
        }
    }
}

/// One validated `OpenPGP` armor header.
#[derive(Clone, Eq, PartialEq)]
pub struct ArmorHeader {
    key: String,
    value: String,
}

impl ArmorHeader {
    /// Builds a header using RFC 9580's exact `Key: Value` grammar.
    ///
    /// Header names are printable ASCII excluding colon and whitespace.
    /// Values are UTF-8 and may contain horizontal tab but no other control.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for invalid syntax or allocation failure.
    pub fn new(key: &str, value: &str) -> Result<Self, OpenPgpError> {
        if !valid_key(key.as_bytes()) || !valid_value(value) {
            return Err(OpenPgpError::new(OpenPgpErrorKind::InvalidHeader));
        }
        let mut owned_key = String::new();
        owned_key
            .try_reserve_exact(key.len())
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
        owned_key.push_str(key);
        let mut owned_value = String::new();
        owned_value
            .try_reserve_exact(value.len())
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
        owned_value.push_str(value);
        Ok(Self {
            key: owned_key,
            value: owned_value,
        })
    }

    /// Returns the case-sensitive header name.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Returns the UTF-8 header value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn wire_len(&self) -> Option<usize> {
        self.key.len().checked_add(2)?.checked_add(self.value.len())
    }
}

impl core::fmt::Debug for ArmorHeader {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ArmorHeader")
            .field("key", &self.key)
            .field("value_len", &self.value.len())
            .finish()
    }
}

pub(crate) fn valid_key(key: &[u8]) -> bool {
    !key.is_empty()
        && key
            .iter()
            .all(|byte| byte.is_ascii_graphic() && *byte != b':')
}

pub(crate) fn valid_value(value: &str) -> bool {
    value
        .chars()
        .all(|character| character == '\t' || !character.is_control())
}

/// CRC-24 parsing policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumPolicy {
    /// Follow RFC 9580: report but do not reject checksum problems.
    Rfc9580,
    /// Require one well-formed CRC-24 matching the decoded payload.
    RequireValidCrc24,
}

/// CRC-24 generation policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumGeneration {
    /// Omit the legacy checksum as recommended by RFC 9580.
    Omit,
    /// Emit the legacy RFC 4880-compatible CRC-24 footer.
    LegacyCrc24,
}

/// Observed CRC-24 state for one parsed block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecksumStatus {
    /// No checksum footer was present.
    Absent,
    /// A checksum footer was present and matched.
    Valid,
    /// A footer marker was present but malformed.
    Malformed,
    /// A well-formed checksum did not match the payload.
    Mismatch,
}

/// Generated line ending.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    /// Internet CRLF line endings.
    CrLf,
    /// Unix LF line endings.
    Lf,
}

impl LineEnding {
    pub(crate) const fn bytes(self) -> &'static [u8] {
        match self {
            Self::CrLf => b"\r\n",
            Self::Lf => b"\n",
        }
    }
}

/// Canonical armor generation options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationOptions {
    checksum: ChecksumGeneration,
    line_ending: LineEnding,
    terminal_line_ending: bool,
}

impl GenerationOptions {
    /// Builds options with CRLF output and a terminal line ending.
    #[must_use]
    pub const fn new(checksum: ChecksumGeneration) -> Self {
        Self {
            checksum,
            line_ending: LineEnding::CrLf,
            terminal_line_ending: true,
        }
    }

    /// Selects generated line endings.
    #[must_use]
    pub const fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Selects whether the closing boundary has a trailing line ending.
    #[must_use]
    pub const fn with_terminal_line_ending(mut self, enabled: bool) -> Self {
        self.terminal_line_ending = enabled;
        self
    }

    /// Returns the selected checksum behavior.
    #[must_use]
    pub const fn checksum(self) -> ChecksumGeneration {
        self.checksum
    }
    /// Returns the selected line ending.
    #[must_use]
    pub const fn line_ending(self) -> LineEnding {
        self.line_ending
    }
    /// Returns whether a terminal line ending is generated.
    #[must_use]
    pub const fn terminal_line_ending(self) -> bool {
        self.terminal_line_ending
    }
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self::new(ChecksumGeneration::Omit)
    }
}

/// One fully parsed ordinary armor block.
#[derive(Clone, Eq, PartialEq)]
pub struct ArmorBlock {
    kind: ArmorType,
    headers: Vec<ArmorHeader>,
    contents: Vec<u8>,
    checksum: ChecksumStatus,
}

impl ArmorBlock {
    pub(crate) const fn new(
        kind: ArmorType,
        headers: Vec<ArmorHeader>,
        contents: Vec<u8>,
        checksum: ChecksumStatus,
    ) -> Self {
        Self {
            kind,
            headers,
            contents,
            checksum,
        }
    }

    /// Returns the armor type.
    #[must_use]
    pub const fn kind(&self) -> ArmorType {
        self.kind
    }
    /// Returns headers in source order.
    #[must_use]
    pub fn headers(&self) -> &[ArmorHeader] {
        &self.headers
    }
    /// Returns decoded packet bytes without packet interpretation.
    #[must_use]
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }
    /// Consumes the block and returns decoded packet bytes.
    #[must_use]
    pub fn into_contents(self) -> Vec<u8> {
        self.contents
    }
    /// Returns the observed checksum state.
    #[must_use]
    pub const fn checksum_status(&self) -> ChecksumStatus {
        self.checksum
    }
}

impl core::fmt::Debug for ArmorBlock {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ArmorBlock")
            .field("kind", &self.kind)
            .field("header_count", &self.headers.len())
            .field("decoded_len", &self.contents.len())
            .field("checksum", &self.checksum)
            .finish()
    }
}

/// A bounded document containing one or more ordinary armor blocks.
#[derive(Clone, Eq, PartialEq)]
pub struct ArmorDocument {
    blocks: Vec<ArmorBlock>,
    adjacent_whitespace_bytes: usize,
}

impl ArmorDocument {
    pub(crate) const fn new(blocks: Vec<ArmorBlock>, adjacent: usize) -> Self {
        Self {
            blocks,
            adjacent_whitespace_bytes: adjacent,
        }
    }

    /// Returns blocks in source order.
    #[must_use]
    pub fn blocks(&self) -> &[ArmorBlock] {
        &self.blocks
    }
    /// Consumes the document and returns its blocks.
    #[must_use]
    pub fn into_blocks(self) -> Vec<ArmorBlock> {
        self.blocks
    }
    /// Returns bounded whitespace bytes outside blocks.
    #[must_use]
    pub const fn adjacent_whitespace_bytes(&self) -> usize {
        self.adjacent_whitespace_bytes
    }
}

impl core::fmt::Debug for ArmorDocument {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ArmorDocument")
            .field("block_count", &self.blocks.len())
            .field("adjacent_whitespace_bytes", &self.adjacent_whitespace_bytes)
            .finish()
    }
}
