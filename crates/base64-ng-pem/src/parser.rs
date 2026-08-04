use alloc::vec::Vec;

use crate::{
    PemBlock, PemDocument, PemError, PemErrorKind, PemLabel, PemLimits, PemParsePolicy,
    PemParseReport, limits::WorkBudget,
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
    pub work: WorkBudget,
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
        mut work,
    } = raw;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(raw_blocks.len())
        .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
    let mut decoded_total = 0usize;
    for mut block in raw_blocks {
        let body = block.take_body();
        work.charge(body.len())?;
        let required = base64_ng::STRICT_STANDARD_PADDED
            .decoded_len(&body)
            .map_err(crate::error::map_base64_decode)?;
        decoded_total = decoded_total
            .checked_add(required)
            .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
        if decoded_total > limits.max_decoded_output_bytes() {
            return Err(PemError::new(PemErrorKind::DecodedOutputLimitExceeded));
        }
        // decode_to_vec_with_limit measures once, decode_into measures again,
        // then the validated decoder consumes the body.
        work.charge_repeated(body.len(), 3)?;
        let contents = base64_ng::STRICT_STANDARD_PADDED
            .decode_to_vec_with_limit(&body, required)
            .map_err(crate::error::map_base64_decode)?;
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
    if input.len() > limits.max_input_bytes() {
        return Err(PemError::new(PemErrorKind::InputLimitExceeded));
    }
    let mut work = WorkBudget::new(limits);
    // The physical-line cursor inspects every source byte once.
    work.charge(input.len())?;
    let mut lines = Lines::new(input, limits.max_physical_line_bytes());
    let mut blocks = Vec::new();
    let mut context = ParseContext {
        limits,
        policy,
        validate_base64,
        report: PemParseReport::default(),
        work,
    };
    while let Some(line) = lines.next_line()? {
        context.work.charge(line.bytes.len())?;
        let Some(boundary) = boundary_label(line.bytes, true, policy)? else {
            add_adjacent(&mut context.report, line.span_len, limits)?;
            if !matches!(line.ending, LineEnding::CrLf | LineEnding::None) {
                context.report.non_crlf_line_endings += 1;
            }
            continue;
        };
        if blocks.len() >= limits.max_blocks() {
            return Err(PemError::at(PemErrorKind::BlockLimitExceeded, line.start));
        }
        let block = parse_block(&mut lines, line, boundary, &mut context)?;
        blocks
            .try_reserve(1)
            .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
        blocks.push(block);
    }
    if blocks.is_empty() {
        return Err(PemError::new(PemErrorKind::BeginBoundaryMissing));
    }
    Ok(RawPemDocument {
        blocks,
        report: context.report,
        work: context.work,
    })
}

#[derive(Clone, Copy)]
struct ParsedBoundary<'a> {
    label: &'a [u8],
    had_blanks: bool,
}

struct ParseContext {
    limits: PemLimits,
    policy: PemParsePolicy,
    validate_base64: bool,
    report: PemParseReport,
    work: WorkBudget,
}

fn parse_block(
    lines: &mut Lines<'_>,
    begin_line: Line<'_>,
    begin_boundary: ParsedBoundary<'_>,
    context: &mut ParseContext,
) -> Result<RawPemBlock, PemError> {
    let label = own_boundary_label(begin_boundary, begin_line.start, context)?;
    account_line_ending(
        begin_line.ending,
        context.policy,
        &mut context.report,
        begin_line.start,
        true,
    )?;

    let mut body = WipingBytes::default();
    let mut body_lines = 0usize;
    let mut completed_body_lines_are_64 = true;
    let mut final_body_line_len = 0usize;
    while let Some(line) = lines.next_line()? {
        context.work.charge(line.bytes.len())?;
        if let Some(end_boundary) = boundary_label(line.bytes, false, context.policy)? {
            let end_label = own_boundary_label(end_boundary, line.start, context)?;
            if end_label != label {
                if context.policy == PemParsePolicy::Strict {
                    return Err(PemError::at(PemErrorKind::MismatchedEndLabel, line.start));
                }
                context.report.mismatched_end_labels += 1;
            }
            account_line_ending(
                line.ending,
                context.policy,
                &mut context.report,
                line.start,
                false,
            )?;
            if context.validate_base64 {
                context.work.charge(body.len())?;
                base64_ng::STRICT_STANDARD_PADDED
                    .validate(&body)
                    .map_err(|error| crate::error::map_base64_decode_at(error, line.start))?;
            }
            validate_body_layout(
                body_lines,
                completed_body_lines_are_64,
                final_body_line_len,
                context.policy,
                &mut context.report,
                line.start,
            )?;
            return Ok(RawPemBlock { label, body });
        }
        context.work.charge_repeated(line.bytes.len(), 2)?;
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
        compact_body_line(line, context, &mut body)?;
        final_body_line_len = body.len() - before;
        body_lines = body_lines
            .checked_add(1)
            .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
        account_line_ending(
            line.ending,
            context.policy,
            &mut context.report,
            line.start,
            true,
        )?;
    }
    Err(PemError::at(
        PemErrorKind::MissingEndBoundary,
        begin_line.start,
    ))
}

fn own_boundary_label(
    boundary: ParsedBoundary<'_>,
    position: usize,
    context: &mut ParseContext,
) -> Result<PemLabel, PemError> {
    let bytes = boundary.label;
    if bytes.len() > context.limits.max_label_bytes() {
        return Err(PemError::at(PemErrorKind::LabelLimitExceeded, position));
    }
    context.work.charge_repeated(bytes.len(), 2)?;
    let text = core::str::from_utf8(bytes)
        .map_err(|_| PemError::at(PemErrorKind::InvalidLabel, position))?;
    let label = PemLabel::new(text).map_err(|error| match error {
        crate::PemLabelError::InvalidSyntax => PemError::at(PemErrorKind::InvalidLabel, position),
        crate::PemLabelError::AllocationFailed => PemError::new(PemErrorKind::AllocationFailed),
    })?;
    account_boundary(boundary, &mut context.report)?;
    account_label(&label, context.policy, &mut context.report, position)?;
    Ok(label)
}

fn compact_body_line(
    line: Line<'_>,
    context: &mut ParseContext,
    body: &mut WipingBytes,
) -> Result<(), PemError> {
    context.work.charge_repeated(line.bytes.len(), 2)?;
    if line.bytes.contains(&b':') && body.is_empty() {
        return Err(PemError::at(
            PemErrorKind::LegacyHeadersNotSupported,
            line.start,
        ));
    }
    for byte in line.bytes.iter().copied() {
        if context.policy == PemParsePolicy::Strict
            || byte.is_ascii_alphanumeric()
            || matches!(byte, b'+' | b'/' | b'=')
        {
            body.try_reserve(1)
                .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
            body.push(byte);
        } else {
            context.report.skipped_body_bytes = context
                .report
                .skipped_body_bytes
                .checked_add(1)
                .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
            if context.report.skipped_body_bytes > context.limits.max_adjacent_text_bytes() {
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
