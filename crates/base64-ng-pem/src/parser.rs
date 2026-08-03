use alloc::vec::Vec;

use crate::{
    PemBlock, PemDocument, PemError, PemErrorKind, PemLabel, PemLimits, PemParsePolicy,
    PemParseReport,
};

mod lines;

use lines::{Line, LineEnding, Lines};

const BEGIN_PREFIX: &[u8] = b"-----BEGIN ";
const END_PREFIX: &[u8] = b"-----END ";
pub(crate) struct RawPemBlock {
    pub label: PemLabel,
    body: WipingBytes,
}

impl RawPemBlock {
    pub fn take_body(&mut self) -> WipingBytes {
        core::mem::take(&mut self.body)
    }
}

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

pub(crate) struct RawPemDocument {
    pub blocks: Vec<RawPemBlock>,
    pub report: PemParseReport,
}

/// Incremental bounded document collector.
///
/// Input chunks are retained only up to [`PemLimits::max_input_bytes`].
/// Parsing and decoded output release happen transactionally at [`Self::finish`].
pub struct PemDocumentParser {
    limits: PemLimits,
    policy: PemParsePolicy,
    input: Vec<u8>,
    terminal: bool,
}

impl PemDocumentParser {
    /// Creates an empty bounded parser.
    #[must_use]
    pub const fn new(limits: PemLimits, policy: PemParsePolicy) -> Self {
        Self {
            limits,
            policy,
            input: Vec::new(),
            terminal: false,
        }
    }

    /// Appends one transport chunk without parsing or releasing plaintext.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] on terminal state, limit failure, or allocation
    /// failure. Any failure latches the parser terminal.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), PemError> {
        if self.terminal {
            return Err(PemError::new(PemErrorKind::TerminalState));
        }
        let Some(required) = self.input.len().checked_add(chunk.len()) else {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::LengthOverflow));
        };
        if required > self.limits.max_input_bytes() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::InputLimitExceeded));
        }
        if self.input.try_reserve(chunk.len()).is_err() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::AllocationFailed));
        }
        self.input.extend_from_slice(chunk);
        Ok(())
    }

    /// Parses the complete accumulated document.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] for malformed grammar, Base64, or finite-limit
    /// failure. No decoded block is returned on error.
    pub fn finish(mut self) -> Result<PemDocument, PemError> {
        self.terminal = true;
        parse_pem_document(&self.input, self.limits, self.policy)
    }
}

/// Parses all RFC 7468 textual encoding instances from one bounded document.
///
/// Surrounding text is accepted under its explicit finite limit and counted
/// in the report. Strict policy applies Figure 3 to each block; compatible
/// policy applies the RFC parser latitude and reports every deviation.
///
/// # Errors
///
/// Returns [`PemError`] for malformed grammar, Base64, or finite-limit
/// failure. No partial document is returned.
pub fn parse_pem_document(
    input: &[u8],
    limits: PemLimits,
    policy: PemParsePolicy,
) -> Result<PemDocument, PemError> {
    let raw = parse_raw_document(input, limits, policy, true)?;
    let RawPemDocument {
        blocks: raw_blocks,
        report,
    } = raw;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(raw_blocks.len())
        .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
    let mut decoded_total = 0usize;
    for mut block in raw_blocks {
        let body = block.take_body();
        let required = base64_ng::STRICT_STANDARD_PADDED
            .decoded_len(&body)
            .map_err(crate::error::map_base64)?;
        decoded_total = decoded_total
            .checked_add(required)
            .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
        if decoded_total > limits.max_decoded_output_bytes() {
            return Err(PemError::new(PemErrorKind::DecodedOutputLimitExceeded));
        }
        let contents = base64_ng::STRICT_STANDARD_PADDED
            .decode_to_vec_with_limit(&body, required)
            .map_err(crate::error::map_base64)?;
        blocks.push(PemBlock::new(block.label, contents));
    }
    Ok(PemDocument::new(blocks, report))
}

pub(crate) fn parse_raw_document(
    input: &[u8],
    limits: PemLimits,
    policy: PemParsePolicy,
    validate_base64: bool,
) -> Result<RawPemDocument, PemError> {
    preflight_input(input, limits)?;
    let mut lines = Lines::new(input, limits.max_physical_line_bytes());
    let mut blocks = Vec::new();
    let mut report = PemParseReport::default();
    while let Some(line) = lines.next_line()? {
        let Some(boundary) = boundary_label(line.bytes, true, policy)? else {
            add_adjacent(&mut report, line.span_len, limits)?;
            if !matches!(line.ending, LineEnding::CrLf | LineEnding::None) {
                report.non_crlf_line_endings += 1;
            }
            continue;
        };
        if blocks.len() >= limits.max_blocks() {
            return Err(PemError::at(PemErrorKind::BlockLimitExceeded, line.start));
        }
        let block = parse_block(
            &mut lines,
            line,
            boundary,
            limits,
            policy,
            validate_base64,
            &mut report,
        )?;
        blocks
            .try_reserve(1)
            .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
        blocks.push(block);
    }
    if blocks.is_empty() {
        return Err(PemError::new(PemErrorKind::BeginBoundaryMissing));
    }
    Ok(RawPemDocument { blocks, report })
}

#[derive(Clone, Copy)]
struct ParsedBoundary<'a> {
    label: &'a [u8],
    had_blanks: bool,
}

fn parse_block(
    lines: &mut Lines<'_>,
    begin_line: Line<'_>,
    begin_boundary: ParsedBoundary<'_>,
    limits: PemLimits,
    policy: PemParsePolicy,
    validate_base64: bool,
    report: &mut PemParseReport,
) -> Result<RawPemBlock, PemError> {
    let label_bytes = begin_boundary.label;
    if label_bytes.len() > limits.max_label_bytes() {
        return Err(PemError::at(
            PemErrorKind::LabelLimitExceeded,
            begin_line.start,
        ));
    }
    let label_text = core::str::from_utf8(label_bytes)
        .map_err(|_| PemError::at(PemErrorKind::InvalidLabel, begin_line.start))?;
    let label = PemLabel::new(label_text).map_err(|error| match error {
        crate::PemLabelError::InvalidSyntax => {
            PemError::at(PemErrorKind::InvalidLabel, begin_line.start)
        }
        crate::PemLabelError::AllocationFailed => PemError::new(PemErrorKind::AllocationFailed),
    })?;
    account_boundary(begin_boundary, report)?;
    account_label(&label, policy, report, begin_line.start)?;
    account_line_ending(begin_line.ending, policy, report, begin_line.start, true)?;

    let mut body = WipingBytes::default();
    let mut body_lines = 0usize;
    let mut completed_body_lines_are_64 = true;
    let mut final_body_line_len = 0usize;
    while let Some(line) = lines.next_line()? {
        if let Some(end_boundary) = boundary_label(line.bytes, false, policy)? {
            let end_label_bytes = end_boundary.label;
            let end_text = core::str::from_utf8(end_label_bytes)
                .map_err(|_| PemError::at(PemErrorKind::InvalidLabel, line.start))?;
            let end_label = PemLabel::new(end_text).map_err(|error| match error {
                crate::PemLabelError::InvalidSyntax => {
                    PemError::at(PemErrorKind::InvalidLabel, line.start)
                }
                crate::PemLabelError::AllocationFailed => {
                    PemError::new(PemErrorKind::AllocationFailed)
                }
            })?;
            account_boundary(end_boundary, report)?;
            account_label(&end_label, policy, report, line.start)?;
            if end_label != label {
                if policy == PemParsePolicy::Strict {
                    return Err(PemError::at(PemErrorKind::MismatchedEndLabel, line.start));
                }
                report.mismatched_end_labels += 1;
            }
            account_line_ending(line.ending, policy, report, line.start, false)?;
            if validate_base64 {
                base64_ng::STRICT_STANDARD_PADDED
                    .validate(&body)
                    .map_err(|_| PemError::at(PemErrorKind::InvalidBody, line.start))?;
            }
            validate_body_layout(
                body_lines,
                completed_body_lines_are_64,
                final_body_line_len,
                policy,
                report,
                line.start,
            )?;
            return Ok(RawPemBlock { label, body });
        }
        if line.bytes.windows(5).any(|window| window == b"-----") && line.bytes.contains(&b':') {
            return Err(PemError::at(
                PemErrorKind::LegacyHeadersNotSupported,
                line.start,
            ));
        }
        if body_lines != 0 && final_body_line_len != 64 {
            completed_body_lines_are_64 = false;
        }
        let before = body.len();
        compact_body_line(line, policy, limits, report, &mut body)?;
        final_body_line_len = body.len() - before;
        body_lines = body_lines
            .checked_add(1)
            .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
        account_line_ending(line.ending, policy, report, line.start, true)?;
    }
    Err(PemError::at(
        PemErrorKind::MissingEndBoundary,
        begin_line.start,
    ))
}

fn compact_body_line(
    line: Line<'_>,
    policy: PemParsePolicy,
    limits: PemLimits,
    report: &mut PemParseReport,
    body: &mut WipingBytes,
) -> Result<(), PemError> {
    if line.bytes.contains(&b':') && body.is_empty() {
        return Err(PemError::at(
            PemErrorKind::LegacyHeadersNotSupported,
            line.start,
        ));
    }
    for byte in line.bytes.iter().copied() {
        if policy == PemParsePolicy::Strict
            || byte.is_ascii_alphanumeric()
            || matches!(byte, b'+' | b'/' | b'=')
        {
            body.try_reserve(1)
                .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
            body.push(byte);
        } else {
            report.skipped_body_bytes = report
                .skipped_body_bytes
                .checked_add(1)
                .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
            if report.skipped_body_bytes > limits.max_adjacent_text_bytes() {
                return Err(PemError::at(
                    PemErrorKind::AdjacentTextLimitExceeded,
                    line.start,
                ));
            }
        }
    }
    Ok(())
}

fn validate_body_layout(
    body_lines: usize,
    completed_body_lines_are_64: bool,
    final_body_line_len: usize,
    policy: PemParsePolicy,
    report: &mut PemParseReport,
    position: usize,
) -> Result<(), PemError> {
    let strict =
        body_lines != 0 && completed_body_lines_are_64 && (4..=64).contains(&final_body_line_len);
    if strict {
        return Ok(());
    }
    if policy == PemParsePolicy::Strict {
        return Err(PemError::at(PemErrorKind::NonCanonicalLayout, position));
    }
    report.noncanonical_body_lines += 1;
    Ok(())
}

fn boundary_label(
    line: &[u8],
    begin: bool,
    policy: PemParsePolicy,
) -> Result<Option<ParsedBoundary<'_>>, PemError> {
    let prefix = if begin { BEGIN_PREFIX } else { END_PREFIX };
    let trimmed = if policy == PemParsePolicy::Rfc7468Compatible {
        trim_blanks(line)
    } else {
        line
    };
    if !trimmed.starts_with(prefix) {
        return Ok(None);
    }
    let Some(rest) = trimmed.strip_prefix(prefix) else {
        return Ok(None);
    };
    let Some(label) = rest.strip_suffix(b"-----") else {
        return Err(PemError::new(PemErrorKind::InvalidBoundary));
    };
    Ok(Some(ParsedBoundary {
        label,
        had_blanks: trimmed.len() != line.len(),
    }))
}

fn account_boundary(
    boundary: ParsedBoundary<'_>,
    report: &mut PemParseReport,
) -> Result<(), PemError> {
    if boundary.had_blanks {
        report.noncanonical_boundary_lines = report
            .noncanonical_boundary_lines
            .checked_add(1)
            .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
    }
    Ok(())
}

fn trim_blanks(mut bytes: &[u8]) -> &[u8] {
    while bytes
        .first()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        bytes = &bytes[1..];
    }
    while bytes
        .last()
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn account_label(
    label: &PemLabel,
    policy: PemParsePolicy,
    report: &mut PemParseReport,
    position: usize,
) -> Result<(), PemError> {
    if label.is_canonical_uppercase() {
        return Ok(());
    }
    if policy == PemParsePolicy::Strict {
        return Err(PemError::at(PemErrorKind::NonCanonicalLabel, position));
    }
    report.noncanonical_labels += 1;
    Ok(())
}

fn account_line_ending(
    ending: LineEnding,
    policy: PemParsePolicy,
    report: &mut PemParseReport,
    position: usize,
    required: bool,
) -> Result<(), PemError> {
    if ending == LineEnding::CrLf {
        return Ok(());
    }
    if !required && ending == LineEnding::None && policy == PemParsePolicy::Rfc7468Compatible {
        return Ok(());
    }
    if policy == PemParsePolicy::Strict || (required && ending == LineEnding::None) {
        return Err(PemError::at(PemErrorKind::NonCanonicalLayout, position));
    }
    report.non_crlf_line_endings += 1;
    Ok(())
}

fn add_adjacent(
    report: &mut PemParseReport,
    count: usize,
    limits: PemLimits,
) -> Result<(), PemError> {
    report.adjacent_text_bytes = report
        .adjacent_text_bytes
        .checked_add(count)
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
    if report.adjacent_text_bytes > limits.max_adjacent_text_bytes() {
        return Err(PemError::new(PemErrorKind::AdjacentTextLimitExceeded));
    }
    Ok(())
}

fn preflight_input(input: &[u8], limits: PemLimits) -> Result<(), PemError> {
    if input.len() > limits.max_input_bytes() {
        return Err(PemError::new(PemErrorKind::InputLimitExceeded));
    }
    if input.len() > limits.max_work_before_output() {
        return Err(PemError::new(PemErrorKind::WorkLimitExceeded));
    }
    Ok(())
}
