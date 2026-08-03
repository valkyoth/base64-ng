use alloc::vec::Vec;

use crate::{
    PemError, PemErrorKind, PemGenerationOptions, PemLabel, PemLimits, types::checked_string,
};

const BEGIN_PREFIX: &[u8] = b"-----BEGIN ";
const END_PREFIX: &[u8] = b"-----END ";
const BOUNDARY_SUFFIX: &[u8] = b"-----";

/// Incremental bounded payload collector for one PEM block.
///
/// Payload chunks remain ordinary non-wiping bytes. Callers handling secret
/// payloads should use their approved secret owner and explicitly expose only
/// at the final generation boundary.
pub struct PemBlockEncoder {
    label: PemLabel,
    limits: PemLimits,
    options: PemGenerationOptions,
    payload: Vec<u8>,
    terminal: bool,
}

impl PemBlockEncoder {
    /// Creates an empty encoder after validating label and static limits.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] when the label or zero-length output cannot satisfy
    /// the selected finite policy.
    pub fn new(
        label: PemLabel,
        limits: PemLimits,
        options: PemGenerationOptions,
    ) -> Result<Self, PemError> {
        let required = candidate_pem_block_encoded_len(&label, 0, options)?;
        preflight(&label, 0, limits, options, required)?;
        Ok(Self {
            label,
            limits,
            options,
            payload: Vec::new(),
            terminal: false,
        })
    }

    /// Appends one payload chunk under the final document limits.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] for terminal state, length/limit failure, or
    /// allocation failure. Any failure latches the encoder terminal.
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), PemError> {
        if self.terminal {
            return Err(PemError::new(PemErrorKind::TerminalState));
        }
        let Some(required) = self.payload.len().checked_add(chunk.len()) else {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::LengthOverflow));
        };
        if required > self.limits.max_input_bytes() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::InputLimitExceeded));
        }
        if required > self.limits.max_work_before_output() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::WorkLimitExceeded));
        }
        if required > self.limits.max_decoded_output_bytes() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::DecodedOutputLimitExceeded));
        }
        if self.payload.try_reserve(chunk.len()).is_err() {
            self.terminal = true;
            return Err(PemError::new(PemErrorKind::AllocationFailed));
        }
        self.payload.extend_from_slice(chunk);
        Ok(())
    }

    /// Finishes into caller-owned output transactionally.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] for final limits, allocation, or output capacity.
    pub fn finish_into(mut self, output: &mut [u8]) -> Result<usize, PemError> {
        self.terminal = true;
        encode_pem_block_into(
            &self.label,
            &self.payload,
            output,
            self.limits,
            self.options,
        )
    }

    /// Finishes into an owned textual encoding.
    ///
    /// # Errors
    ///
    /// Returns [`PemError`] for final limits or allocation failure.
    pub fn finish_to_string(mut self) -> Result<alloc::string::String, PemError> {
        self.terminal = true;
        encode_pem_block_to_string(&self.label, &self.payload, self.limits, self.options)
    }
}

/// Returns the exact generated length for one RFC 7468 block.
///
/// # Errors
///
/// Returns [`PemError`] for an empty payload, noncanonical label, or
/// arithmetic overflow.
pub fn pem_block_encoded_len(
    label: &PemLabel,
    payload_len: usize,
    options: PemGenerationOptions,
) -> Result<usize, PemError> {
    require_nonempty_payload(payload_len)?;
    candidate_pem_block_encoded_len(label, payload_len, options)
}

fn candidate_pem_block_encoded_len(
    label: &PemLabel,
    payload_len: usize,
    options: PemGenerationOptions,
) -> Result<usize, PemError> {
    require_canonical_label(label)?;
    let body_len = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload_len)
        .map_err(|_| PemError::new(PemErrorKind::LengthOverflow))?;
    let ending_len = options.line_ending().bytes().len();
    let body_lines = if body_len == 0 {
        1
    } else {
        body_len.div_ceil(64)
    };
    let boundary_bytes = BEGIN_PREFIX
        .len()
        .checked_add(label.as_str().len())
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .and_then(|value| value.checked_add(ending_len))
        .and_then(|value| value.checked_add(END_PREFIX.len()))
        .and_then(|value| value.checked_add(label.as_str().len()))
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
    let body_endings = body_lines
        .checked_mul(ending_len)
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
    boundary_bytes
        .checked_add(body_len)
        .and_then(|value| value.checked_add(body_endings))
        .and_then(|value| {
            if options.terminal_line_ending() {
                value.checked_add(ending_len)
            } else {
                Some(value)
            }
        })
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))
}

/// Generates one canonical RFC 7468 block transactionally.
///
/// Every error leaves `output` unchanged. The payload is encoded as strict
/// RFC 4648 Standard Base64 in exact 64-character lines.
///
/// # Errors
///
/// Returns [`PemError`] for invalid policy, finite-limit failure, allocation
/// failure during preflight, or insufficient caller output.
pub fn encode_pem_block_into(
    label: &PemLabel,
    payload: &[u8],
    output: &mut [u8],
    limits: PemLimits,
    options: PemGenerationOptions,
) -> Result<usize, PemError> {
    require_nonempty_payload(payload.len())?;
    preflight(label, payload.len(), limits, options, output.len())?;
    let required = candidate_pem_block_encoded_len(label, payload.len(), options)?;
    let mut body = Vec::new();
    let body_len = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload.len())
        .map_err(|_| PemError::new(PemErrorKind::LengthOverflow))?;
    body.try_reserve_exact(body_len)
        .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
    body.resize(body_len, 0);
    base64_ng::STRICT_STANDARD_PADDED
        .encode_into(payload, &mut body)
        .map_err(crate::error::map_base64)?;
    write_validated(label, &body, &mut output[..required], options);
    Ok(required)
}

/// Generates one canonical RFC 7468 block as an owned string.
///
/// # Errors
///
/// Returns [`PemError`] for invalid policy, finite-limit failure, length
/// overflow, or allocation failure.
pub fn encode_pem_block_to_string(
    label: &PemLabel,
    payload: &[u8],
    limits: PemLimits,
    options: PemGenerationOptions,
) -> Result<alloc::string::String, PemError> {
    require_nonempty_payload(payload.len())?;
    let required = pem_block_encoded_len(label, payload.len(), options)?;
    preflight(label, payload.len(), limits, options, required)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(required)
        .map_err(|_| PemError::new(PemErrorKind::AllocationFailed))?;
    output.resize(required, 0);
    encode_pem_block_into(label, payload, &mut output, limits, options)?;
    checked_string(output)
}

fn preflight(
    label: &PemLabel,
    payload_len: usize,
    limits: PemLimits,
    options: PemGenerationOptions,
    available: usize,
) -> Result<(), PemError> {
    require_canonical_label(label)?;
    if payload_len > limits.max_input_bytes() {
        return Err(PemError::new(PemErrorKind::InputLimitExceeded));
    }
    if payload_len > limits.max_work_before_output() {
        return Err(PemError::new(PemErrorKind::WorkLimitExceeded));
    }
    if limits.max_blocks() == 0 {
        return Err(PemError::new(PemErrorKind::BlockLimitExceeded));
    }
    if label.as_str().len() > limits.max_label_bytes() {
        return Err(PemError::new(PemErrorKind::LabelLimitExceeded));
    }
    let boundary_line = BEGIN_PREFIX
        .len()
        .checked_add(label.as_str().len())
        .and_then(|value| value.checked_add(BOUNDARY_SUFFIX.len()))
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))?;
    let body_len = base64_ng::STRICT_STANDARD_PADDED
        .encoded_len(payload_len)
        .map_err(|_| PemError::new(PemErrorKind::LengthOverflow))?;
    let longest_body_line = body_len.min(64);
    if boundary_line.max(longest_body_line) > limits.max_physical_line_bytes() {
        return Err(PemError::new(PemErrorKind::PhysicalLineTooLong));
    }
    if payload_len > limits.max_decoded_output_bytes() {
        return Err(PemError::new(PemErrorKind::DecodedOutputLimitExceeded));
    }
    let required = candidate_pem_block_encoded_len(label, payload_len, options)?;
    if required > limits.max_encoded_output_bytes() {
        return Err(PemError::new(PemErrorKind::EncodedOutputLimitExceeded));
    }
    if required > available {
        return Err(PemError::capacity(required, available));
    }
    Ok(())
}

fn require_canonical_label(label: &PemLabel) -> Result<(), PemError> {
    if label.is_canonical_uppercase() {
        Ok(())
    } else {
        Err(PemError::new(PemErrorKind::NonCanonicalLabel))
    }
}

fn require_nonempty_payload(payload_len: usize) -> Result<(), PemError> {
    if payload_len == 0 {
        Err(PemError::new(PemErrorKind::InvalidBody))
    } else {
        Ok(())
    }
}

fn write_validated(
    label: &PemLabel,
    body: &[u8],
    output: &mut [u8],
    options: PemGenerationOptions,
) {
    let ending = options.line_ending().bytes();
    let mut cursor = 0;
    put(output, &mut cursor, BEGIN_PREFIX);
    put(output, &mut cursor, label.as_str().as_bytes());
    put(output, &mut cursor, BOUNDARY_SUFFIX);
    put(output, &mut cursor, ending);
    for line in body.chunks(64) {
        put(output, &mut cursor, line);
        put(output, &mut cursor, ending);
    }
    put(output, &mut cursor, END_PREFIX);
    put(output, &mut cursor, label.as_str().as_bytes());
    put(output, &mut cursor, BOUNDARY_SUFFIX);
    if options.terminal_line_ending() {
        put(output, &mut cursor, ending);
    }
    debug_assert_eq!(cursor, output.len());
}

fn put(output: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    output[*cursor..*cursor + bytes.len()].copy_from_slice(bytes);
    *cursor += bytes.len();
}
