use alloc::vec::Vec;

use crate::{
    ArmorBlock, ArmorDocument, ArmorHeader, ArmorType, ChecksumPolicy, ChecksumStatus,
    OpenPgpError, OpenPgpErrorKind, OpenPgpLimits, crc24,
};

mod lines;

use lines::{Line, Lines};

const BEGIN_PREFIX: &[u8] = b"-----BEGIN ";
const END_PREFIX: &[u8] = b"-----END ";
const BOUNDARY_SUFFIX: &[u8] = b"-----";

#[derive(Default)]
pub(crate) struct WipingBytes(Vec<u8>);

impl WipingBytes {
    #[cfg(feature = "secrets")]
    pub fn into_vec(mut self) -> Vec<u8> {
        core::mem::take(&mut self.0)
    }
}

impl core::ops::Deref for WipingBytes {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for WipingBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for WipingBytes {
    fn drop(&mut self) {
        base64_ng::secure_wipe(&mut self.0);
    }
}

#[derive(Clone, Copy)]
pub(crate) enum RawChecksum {
    Absent,
    Malformed,
    Value(u32),
}

pub(crate) struct RawArmorBlock {
    pub kind: ArmorType,
    pub headers: Vec<ArmorHeader>,
    pub body: WipingBytes,
    pub checksum: RawChecksum,
}

impl RawArmorBlock {
    #[cfg(feature = "secrets")]
    pub fn take_body(&mut self) -> WipingBytes {
        core::mem::take(&mut self.body)
    }
}

pub(crate) struct RawArmorDocument {
    pub blocks: Vec<RawArmorBlock>,
    pub adjacent: usize,
}

/// Incremental bounded armor document collector.
///
/// Input chunks are retained only up to [`OpenPgpLimits::max_input_bytes`].
/// Parsing and decoded output release occur transactionally at [`Self::finish`].
pub struct ArmorDocumentParser {
    limits: OpenPgpLimits,
    checksum: ChecksumPolicy,
    input: Vec<u8>,
    terminal: bool,
}

impl ArmorDocumentParser {
    /// Creates an empty bounded parser.
    #[must_use]
    pub const fn new(limits: OpenPgpLimits, checksum: ChecksumPolicy) -> Self {
        Self {
            limits,
            checksum,
            input: Vec::new(),
            terminal: false,
        }
    }

    /// Appends one transport chunk without parsing or releasing payload bytes.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] on terminal state, limit failure, or allocation
    /// failure. Any failure latches the parser terminal.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), OpenPgpError> {
        if self.terminal {
            return Err(OpenPgpError::new(OpenPgpErrorKind::TerminalState));
        }
        let Some(required) = self.input.len().checked_add(chunk.len()) else {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::LengthOverflow));
        };
        if required > self.limits.max_input_bytes() {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::InputLimitExceeded));
        }
        if self.input.try_reserve(chunk.len()).is_err() {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::AllocationFailed));
        }
        self.input.extend_from_slice(chunk);
        Ok(())
    }

    /// Parses the complete accumulated document.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for malformed armor or finite-limit failure.
    pub fn finish(mut self) -> Result<ArmorDocument, OpenPgpError> {
        self.terminal = true;
        parse_armor_document(&self.input, self.limits, self.checksum)
    }
}

/// Parses all ordinary RFC 9580 armor blocks from one bounded document.
///
/// Only ASCII whitespace is accepted outside blocks. Under
/// [`ChecksumPolicy::Rfc9580`], checksum defects are reported through each
/// block's [`ChecksumStatus`] as required by RFC 9580 rather than rejected.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for malformed framing, headers, Base64, strict
/// checksum-policy failure, or a finite-limit failure. No partial document is
/// returned.
pub fn parse_armor_document(
    input: &[u8],
    limits: OpenPgpLimits,
    checksum_policy: ChecksumPolicy,
) -> Result<ArmorDocument, OpenPgpError> {
    let raw = parse_raw_document(input, limits)?;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(raw.blocks.len())
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    let mut decoded_total = 0usize;
    for raw_block in raw.blocks {
        let required = base64_ng::STRICT_STANDARD_PADDED
            .decoded_len(&raw_block.body)
            .map_err(crate::error::map_base64)?;
        decoded_total = decoded_total
            .checked_add(required)
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
        if decoded_total > limits.max_decoded_output_bytes() {
            return Err(OpenPgpError::new(
                OpenPgpErrorKind::DecodedOutputLimitExceeded,
            ));
        }
        let contents = base64_ng::STRICT_STANDARD_PADDED
            .decode_to_vec_with_limit(&raw_block.body, required)
            .map_err(crate::error::map_base64)?;
        let status = checksum_status(raw_block.checksum, &contents);
        enforce_checksum(checksum_policy, status)?;
        blocks.push(ArmorBlock::new(
            raw_block.kind,
            raw_block.headers,
            contents,
            status,
        ));
    }
    Ok(ArmorDocument::new(blocks, raw.adjacent))
}

pub(crate) fn parse_raw_document(
    input: &[u8],
    limits: OpenPgpLimits,
) -> Result<RawArmorDocument, OpenPgpError> {
    if input.len() > limits.max_input_bytes() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::InputLimitExceeded));
    }
    if input.len() > limits.max_work_before_output() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::WorkLimitExceeded));
    }
    let mut lines = Lines::new(input, limits.max_physical_line_bytes());
    let mut blocks = Vec::new();
    let mut adjacent = 0usize;
    let mut total_header_bytes = 0usize;
    while let Some(line) = lines.next_line()? {
        if let Some(kind) = parse_boundary(line.bytes, true, limits, line.start)? {
            if blocks.len() >= limits.max_blocks() {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::BlockLimitExceeded,
                    line.start,
                ));
            }
            let block = parse_block(
                &mut lines,
                kind,
                limits,
                &mut total_header_bytes,
                line.start,
            )?;
            blocks
                .try_reserve(1)
                .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
            blocks.push(block);
        } else {
            if !line.bytes.iter().all(u8::is_ascii_whitespace) {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::TrailingAmbiguity,
                    line.start,
                ));
            }
            adjacent = adjacent
                .checked_add(line.span_len)
                .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
            if adjacent > limits.max_adjacent_document_bytes() {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::AdjacentDocumentLimitExceeded,
                    line.start,
                ));
            }
        }
    }
    if blocks.is_empty() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::BeginBoundaryMissing));
    }
    Ok(RawArmorDocument { blocks, adjacent })
}

fn parse_block(
    lines: &mut Lines<'_>,
    kind: ArmorType,
    limits: OpenPgpLimits,
    total_header_bytes: &mut usize,
    begin_position: usize,
) -> Result<RawArmorBlock, OpenPgpError> {
    let mut headers = Vec::new();
    loop {
        let line = lines.next_line()?.ok_or_else(|| {
            OpenPgpError::at(OpenPgpErrorKind::MissingHeaderSeparator, begin_position)
        })?;
        if line.bytes.iter().all(|byte| matches!(byte, b' ' | b'\t')) {
            break;
        }
        if parse_boundary(line.bytes, false, limits, line.start)?.is_some() {
            return Err(OpenPgpError::at(
                OpenPgpErrorKind::MissingHeaderSeparator,
                line.start,
            ));
        }
        if headers.len() >= limits.max_header_count() {
            return Err(OpenPgpError::at(
                OpenPgpErrorKind::HeaderCountLimitExceeded,
                line.start,
            ));
        }
        let header = parse_header(line)?;
        *total_header_bytes = total_header_bytes
            .checked_add(line.bytes.len())
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
        if *total_header_bytes > limits.max_total_header_bytes() {
            return Err(OpenPgpError::at(
                OpenPgpErrorKind::HeaderBytesLimitExceeded,
                line.start,
            ));
        }
        headers
            .try_reserve(1)
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
        headers.push(header);
    }

    let mut body = WipingBytes::default();
    let mut checksum = RawChecksum::Absent;
    let mut saw_checksum = false;
    while let Some(line) = lines.next_line()? {
        if let Some(end_kind) = parse_boundary(line.bytes, false, limits, line.start)? {
            if end_kind != kind {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::MismatchedEndBoundary,
                    line.start,
                ));
            }
            base64_ng::STRICT_STANDARD_PADDED
                .validate(&body)
                .map_err(|_| OpenPgpError::at(OpenPgpErrorKind::InvalidBody, line.start))?;
            return Ok(RawArmorBlock {
                kind,
                headers,
                body,
                checksum,
            });
        }
        let trimmed = trim_trailing_blanks(line.bytes);
        if trimmed.starts_with(b"=") {
            if saw_checksum {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::TrailingAmbiguity,
                    line.start,
                ));
            }
            checksum = parse_checksum(trimmed);
            saw_checksum = true;
            continue;
        }
        if saw_checksum {
            if trimmed.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            return Err(OpenPgpError::at(
                OpenPgpErrorKind::TrailingAmbiguity,
                line.start,
            ));
        }
        compact_body_line(line, &mut body)?;
    }
    Err(OpenPgpError::at(
        OpenPgpErrorKind::MissingEndBoundary,
        begin_position,
    ))
}

fn parse_header(line: Line<'_>) -> Result<ArmorHeader, OpenPgpError> {
    let Some(separator) = line.bytes.windows(2).position(|window| window == b": ") else {
        return Err(OpenPgpError::at(
            OpenPgpErrorKind::InvalidHeader,
            line.start,
        ));
    };
    let key = core::str::from_utf8(&line.bytes[..separator])
        .map_err(|_| OpenPgpError::at(OpenPgpErrorKind::InvalidHeader, line.start))?;
    let value = core::str::from_utf8(&line.bytes[separator + 2..])
        .map_err(|_| OpenPgpError::at(OpenPgpErrorKind::InvalidHeader, line.start))?;
    ArmorHeader::new(key, value)
        .map_err(|_| OpenPgpError::at(OpenPgpErrorKind::InvalidHeader, line.start))
}

fn compact_body_line(line: Line<'_>, body: &mut WipingBytes) -> Result<(), OpenPgpError> {
    let symbols = line
        .bytes
        .iter()
        .filter(|byte| !byte.is_ascii_whitespace())
        .count();
    if symbols > 76 {
        return Err(OpenPgpError::at(
            OpenPgpErrorKind::BodyLineTooLong,
            line.start,
        ));
    }
    body.try_reserve(symbols)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    for byte in line.bytes.iter().copied() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')) {
            return Err(OpenPgpError::at(OpenPgpErrorKind::InvalidBody, line.start));
        }
        body.push(byte);
    }
    Ok(())
}

fn parse_boundary(
    bytes: &[u8],
    begin: bool,
    limits: OpenPgpLimits,
    position: usize,
) -> Result<Option<ArmorType>, OpenPgpError> {
    let bytes = trim_trailing_blanks(bytes);
    let prefix = if begin { BEGIN_PREFIX } else { END_PREFIX };
    if !bytes.starts_with(prefix) {
        return Ok(None);
    }
    let label = bytes
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(BOUNDARY_SUFFIX))
        .ok_or_else(|| OpenPgpError::at(OpenPgpErrorKind::InvalidBoundary, position))?;
    if label.len() > limits.max_label_bytes() {
        return Err(OpenPgpError::at(
            OpenPgpErrorKind::LabelLimitExceeded,
            position,
        ));
    }
    ArmorType::from_label(label)
        .map(Some)
        .ok_or_else(|| OpenPgpError::at(OpenPgpErrorKind::InvalidBoundary, position))
}

fn parse_checksum(line: &[u8]) -> RawChecksum {
    if line.len() != 5 {
        return RawChecksum::Malformed;
    }
    let mut decoded = [0u8; 3];
    match base64_ng::STRICT_STANDARD_PADDED.decode_into(&line[1..], &mut decoded) {
        Ok(3) => RawChecksum::Value(
            (u32::from(decoded[0]) << 16) | (u32::from(decoded[1]) << 8) | u32::from(decoded[2]),
        ),
        _ => RawChecksum::Malformed,
    }
}

pub(crate) fn checksum_status(checksum: RawChecksum, contents: &[u8]) -> ChecksumStatus {
    match checksum {
        RawChecksum::Absent => ChecksumStatus::Absent,
        RawChecksum::Malformed => ChecksumStatus::Malformed,
        RawChecksum::Value(value) if value == crc24::crc24(contents) => ChecksumStatus::Valid,
        RawChecksum::Value(_) => ChecksumStatus::Mismatch,
    }
}

pub(crate) fn enforce_checksum(
    policy: ChecksumPolicy,
    status: ChecksumStatus,
) -> Result<(), OpenPgpError> {
    if policy == ChecksumPolicy::Rfc9580 || status == ChecksumStatus::Valid {
        return Ok(());
    }
    let kind = match status {
        ChecksumStatus::Absent => OpenPgpErrorKind::ChecksumMissing,
        ChecksumStatus::Malformed => OpenPgpErrorKind::ChecksumMalformed,
        ChecksumStatus::Mismatch => OpenPgpErrorKind::ChecksumMismatch,
        ChecksumStatus::Valid => return Ok(()),
    };
    Err(OpenPgpError::new(kind))
}

fn trim_trailing_blanks(mut bytes: &[u8]) -> &[u8] {
    while bytes
        .last()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}
