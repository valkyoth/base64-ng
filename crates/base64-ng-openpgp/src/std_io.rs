use alloc::vec::Vec;
use std::io::{Read, Write};

use crate::{
    ArmorDocument, ArmorHeader, ArmorType, ChecksumPolicy, GenerationOptions, OpenPgpError,
    OpenPgpErrorKind, OpenPgpLimits, encode_armor_to_string, parse_armor_document,
};

/// Reads and parses one bounded `OpenPGP` armor document.
///
/// The reader is consumed in fixed-size chunks. No decoded block is released
/// before EOF and complete validation.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for I/O, allocation, grammar, checksum-policy, or
/// finite-limit failure.
pub fn read_armor_document<R: Read>(
    mut reader: R,
    limits: OpenPgpLimits,
    checksum: ChecksumPolicy,
) -> Result<ArmorDocument, OpenPgpError> {
    let mut input = Vec::new();
    let reserve = limits.max_input_bytes().min(8192);
    input
        .try_reserve(reserve)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    let mut chunk = [0u8; 4096];
    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::Io))?;
        if read == 0 {
            break;
        }
        let required = input
            .len()
            .checked_add(read)
            .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
        if required > limits.max_input_bytes() {
            return Err(OpenPgpError::new(OpenPgpErrorKind::InputLimitExceeded));
        }
        input
            .try_reserve(read)
            .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
        input.extend_from_slice(&chunk[..read]);
    }
    parse_armor_document(&input, limits, checksum)
}

/// Generates and writes one bounded armor block.
///
/// Generation is fully validated before the first writer call. An underlying
/// writer error can still leave an externally visible committed prefix, as is
/// inherent to [`Write::write_all`].
///
/// # Errors
///
/// Returns [`OpenPgpError`] for generation or writer failure.
pub fn write_armor_block<W: Write>(
    mut writer: W,
    kind: ArmorType,
    headers: &[ArmorHeader],
    payload: &[u8],
    limits: OpenPgpLimits,
    options: GenerationOptions,
) -> Result<usize, OpenPgpError> {
    let text = encode_armor_to_string(kind, headers, payload, limits, options)?;
    writer
        .write_all(text.as_bytes())
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::Io))?;
    Ok(text.len())
}
