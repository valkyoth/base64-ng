use alloc::{string::String, vec::Vec};

use crate::{
    ArmorHeader, ArmorType, ChecksumGeneration, GenerationOptions, OpenPgpError, OpenPgpErrorKind,
    OpenPgpLimits, crc24,
};

const BEGIN_PREFIX: &[u8] = b"-----BEGIN ";
const END_PREFIX: &[u8] = b"-----END ";
const BOUNDARY_SUFFIX: &[u8] = b"-----";

/// Incremental bounded payload collector for one armor block.
///
/// Payload chunks are ordinary non-wiping bytes. Secret-bearing callers must
/// retain payloads in their approved secret owner and expose them only at the
/// final generation boundary.
pub struct ArmorEncoder {
    kind: ArmorType,
    headers: Vec<ArmorHeader>,
    limits: OpenPgpLimits,
    options: GenerationOptions,
    payload: Vec<u8>,
    terminal: bool,
}

impl ArmorEncoder {
    /// Creates an empty encoder after validating static limits and headers.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for invalid limits, headers, or allocation.
    pub fn new(
        kind: ArmorType,
        headers: &[ArmorHeader],
        limits: OpenPgpLimits,
        options: GenerationOptions,
    ) -> Result<Self, OpenPgpError> {
        preflight(kind, headers, 0, limits, options, usize::MAX)?;
        let mut owned_headers = Vec::new();
        owned_headers
            .try_reserve_exact(headers.len())
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
        owned_headers.extend_from_slice(headers);
        Ok(Self {
            kind,
            headers: owned_headers,
            limits,
            options,
            payload: Vec::new(),
            terminal: false,
        })
    }

    /// Appends one payload chunk under the final limits.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for terminal state, a length or limit failure,
    /// or allocation failure. Any error latches terminal state.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), OpenPgpError> {
        if self.terminal {
            return Err(OpenPgpError::new(OpenPgpErrorKind::TerminalState));
        }
        let Some(required) = self.payload.len().checked_add(chunk.len()) else {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::LengthOverflow));
        };
        if required > self.limits.max_input_bytes() {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::InputLimitExceeded));
        }
        if required > self.limits.max_decoded_output_bytes() {
            self.terminal = true;
            return Err(OpenPgpError::new(
                OpenPgpErrorKind::DecodedOutputLimitExceeded,
            ));
        }
        if required > self.limits.max_work_before_output() {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::WorkLimitExceeded));
        }
        if self.payload.try_reserve(chunk.len()).is_err() {
            self.terminal = true;
            return Err(OpenPgpError::new(OpenPgpErrorKind::AllocationFailed));
        }
        self.payload.extend_from_slice(chunk);
        Ok(())
    }

    /// Finishes transactionally into caller-owned output.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for final limits, allocation, or capacity.
    pub fn finish_into(mut self, output: &mut [u8]) -> Result<usize, OpenPgpError> {
        self.terminal = true;
        encode_armor_into(
            self.kind,
            &self.headers,
            &self.payload,
            output,
            self.limits,
            self.options,
        )
    }

    /// Finishes into an owned armor string.
    ///
    /// # Errors
    ///
    /// Returns [`OpenPgpError`] for final limits or allocation failure.
    pub fn finish_to_string(mut self) -> Result<String, OpenPgpError> {
        self.terminal = true;
        encode_armor_to_string(
            self.kind,
            &self.headers,
            &self.payload,
            self.limits,
            self.options,
        )
    }
}

/// Returns the exact generated length for one armor block.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for arithmetic overflow or invalid header size.
pub fn armor_encoded_len(
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload_len: usize,
    options: GenerationOptions,
) -> Result<usize, OpenPgpError> {
    let ending = options.line_ending().bytes().len();
    let label = kind.label().len();
    let boundary = BEGIN_PREFIX
        .len()
        .checked_add(label)
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    let body = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload_len)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    let body_lines = if body == 0 { 0 } else { body.div_ceil(76) };
    let mut total = boundary
        .checked_add(ending)
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    for header in headers {
        total = total
            .checked_add(
                header
                    .wire_len()
                    .and_then(|value| value.checked_add(ending))
                    .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?,
            )
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    }
    total = total
        .checked_add(ending)
        .and_then(|value| value.checked_add(body))
        .and_then(|value| value.checked_add(body_lines.checked_mul(ending)?))
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    if options.checksum() == ChecksumGeneration::LegacyCrc24 {
        total = total
            .checked_add(5)
            .and_then(|value| value.checked_add(ending))
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    }
    total = total
        .checked_add(END_PREFIX.len())
        .and_then(|value| value.checked_add(label))
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    if options.terminal_line_ending() {
        total = total
            .checked_add(ending)
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    }
    Ok(total)
}

/// Generates one canonical ordinary RFC 9580 armor block transactionally.
///
/// Every error leaves `output` unchanged. Base64 body lines contain at most
/// 76 characters. CRC-24 is emitted only when explicitly selected.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for finite-limit failure, allocation failure, or
/// insufficient caller-owned output.
pub fn encode_armor_into(
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload: &[u8],
    output: &mut [u8],
    limits: OpenPgpLimits,
    options: GenerationOptions,
) -> Result<usize, OpenPgpError> {
    preflight(kind, headers, payload.len(), limits, options, output.len())?;
    let required = armor_encoded_len(kind, headers, payload.len(), options)?;
    let body_len = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload.len())
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    let mut body = Vec::new();
    body.try_reserve_exact(body_len)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    body.resize(body_len, 0);
    base64_ng::STRICT_STANDARD_PADDED
        .encode_into(payload, &mut body)
        .map_err(crate::error::map_base64)?;
    write_validated(
        kind,
        headers,
        payload,
        &body,
        &mut output[..required],
        options,
    );
    Ok(required)
}

/// Generates one canonical ordinary RFC 9580 armor block as a string.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for finite-limit or allocation failure.
pub fn encode_armor_to_string(
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload: &[u8],
    limits: OpenPgpLimits,
    options: GenerationOptions,
) -> Result<String, OpenPgpError> {
    let required = armor_encoded_len(kind, headers, payload.len(), options)?;
    preflight(kind, headers, payload.len(), limits, options, required)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    encode_armor_into(kind, headers, payload, &mut output, limits, options)?;
    String::from_utf8(output)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::InternalInvariantViolation))
}

fn preflight(
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload_len: usize,
    limits: OpenPgpLimits,
    options: GenerationOptions,
    available: usize,
) -> Result<(), OpenPgpError> {
    if payload_len > limits.max_input_bytes() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::InputLimitExceeded));
    }
    if payload_len > limits.max_decoded_output_bytes() {
        return Err(OpenPgpError::new(
            OpenPgpErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    if payload_len > limits.max_work_before_output() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::WorkLimitExceeded));
    }
    if limits.max_blocks() == 0 {
        return Err(OpenPgpError::new(OpenPgpErrorKind::BlockLimitExceeded));
    }
    if kind.label().len() > limits.max_label_bytes() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::LabelLimitExceeded));
    }
    if headers.len() > limits.max_header_count() {
        return Err(OpenPgpError::new(
            OpenPgpErrorKind::HeaderCountLimitExceeded,
        ));
    }
    let mut header_bytes = 0usize;
    for header in headers {
        let wire = header
            .wire_len()
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
        header_bytes = header_bytes
            .checked_add(wire)
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
        if wire > limits.max_physical_line_bytes() {
            return Err(OpenPgpError::new(OpenPgpErrorKind::PhysicalLineTooLong));
        }
    }
    if header_bytes > limits.max_total_header_bytes() {
        return Err(OpenPgpError::new(
            OpenPgpErrorKind::HeaderBytesLimitExceeded,
        ));
    }
    let boundary_len = BEGIN_PREFIX
        .len()
        .checked_add(kind.label().len())
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    let body_len = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload_len)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    let longest_body_line = body_len.min(76);
    let checksum_line = if options.checksum() == ChecksumGeneration::LegacyCrc24 {
        5
    } else {
        0
    };
    if boundary_len.max(longest_body_line).max(checksum_line) > limits.max_physical_line_bytes() {
        return Err(OpenPgpError::new(OpenPgpErrorKind::PhysicalLineTooLong));
    }
    let required = armor_encoded_len(kind, headers, payload_len, options)?;
    if required > limits.max_encoded_output_bytes() {
        return Err(OpenPgpError::new(
            OpenPgpErrorKind::EncodedOutputLimitExceeded,
        ));
    }
    if required > available {
        return Err(OpenPgpError::capacity(required, available));
    }
    Ok(())
}

fn write_validated(
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload: &[u8],
    body: &[u8],
    output: &mut [u8],
    options: GenerationOptions,
) {
    let ending = options.line_ending().bytes();
    let mut cursor = 0usize;
    put(output, &mut cursor, BEGIN_PREFIX);
    put(output, &mut cursor, kind.label().as_bytes());
    put(output, &mut cursor, BOUNDARY_SUFFIX);
    put(output, &mut cursor, ending);
    for header in headers {
        put(output, &mut cursor, header.key().as_bytes());
        put(output, &mut cursor, b": ");
        put(output, &mut cursor, header.value().as_bytes());
        put(output, &mut cursor, ending);
    }
    put(output, &mut cursor, ending);
    for line in body.chunks(76) {
        put(output, &mut cursor, line);
        put(output, &mut cursor, ending);
    }
    if options.checksum() == ChecksumGeneration::LegacyCrc24 {
        let crc = crc24::bytes(crc24::crc24(payload));
        let encoded = encode_three(crc);
        put(output, &mut cursor, b"=");
        put(output, &mut cursor, &encoded);
        put(output, &mut cursor, ending);
    }
    put(output, &mut cursor, END_PREFIX);
    put(output, &mut cursor, kind.label().as_bytes());
    put(output, &mut cursor, BOUNDARY_SUFFIX);
    if options.terminal_line_ending() {
        put(output, &mut cursor, ending);
    }
    debug_assert_eq!(cursor, output.len());
}

fn encode_three(input: [u8; 3]) -> [u8; 4] {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    [
        ALPHABET[usize::from(input[0] >> 2)],
        ALPHABET[usize::from(((input[0] & 0x03) << 4) | (input[1] >> 4))],
        ALPHABET[usize::from(((input[1] & 0x0f) << 2) | (input[2] >> 6))],
        ALPHABET[usize::from(input[2] & 0x3f)],
    ]
}

fn put(output: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    output[*cursor..*cursor + bytes.len()].copy_from_slice(bytes);
    *cursor += bytes.len();
}
