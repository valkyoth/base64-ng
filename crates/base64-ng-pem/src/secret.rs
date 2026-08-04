use base64_ng::secret::{SecretDecodeError, SecretInput, SecretVec, SecretVecFrame};

use crate::{
    PemError, PemErrorKind, PemLabel, PemLimits, PemParsePolicy, PemParseReport,
    parser::parse_raw_document,
};

/// One expected-label PEM payload in clear-on-drop secret storage.
pub struct SecretPemBlock {
    label: PemLabel,
    contents: SecretVec,
    report: PemParseReport,
}

impl SecretPemBlock {
    /// Returns the public boundary label.
    #[must_use]
    pub const fn label(&self) -> &PemLabel {
        &self.label
    }

    /// Returns decoded secret storage for explicit exposure by the caller.
    #[must_use]
    pub const fn contents(&self) -> &SecretVec {
        &self.contents
    }

    /// Consumes the block and returns its clear-on-drop secret storage.
    #[must_use]
    pub fn into_contents(self) -> SecretVec {
        self.contents
    }

    /// Returns bounded parser deviations.
    #[must_use]
    pub const fn report(&self) -> PemParseReport {
        self.report
    }
}

impl core::fmt::Debug for SecretPemBlock {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretPemBlock")
            .field("label", &self.label)
            .field("contents", &"<redacted>")
            .field("decoded_len", &self.contents.len())
            .field("report", &self.report)
            .finish()
    }
}

/// Parses exactly one expected-label block into clear-on-drop secret storage.
///
/// The compacted encoded body is classified as secret and wiped on drop. The
/// core fixed-work secret decoder stages plaintext and releases it only after
/// complete validation. The expected label prevents silent type confusion.
///
/// # Errors
///
/// Returns [`PemError`] for grammar, limits, label selection, allocation, or
/// opaque secret-decoder failure. No secret block is returned on error.
pub fn parse_pem_secret_block(
    input: &[u8],
    expected_label: &PemLabel,
    limits: PemLimits,
    policy: PemParsePolicy,
) -> Result<SecretPemBlock, PemError> {
    if policy != PemParsePolicy::Strict {
        return Err(PemError::new(PemErrorKind::SecretBlockSelection));
    }
    let mut document = parse_raw_document(input, limits, policy, false)?;
    if document.blocks.len() != 1 || document.blocks[0].label != *expected_label {
        return Err(PemError::new(PemErrorKind::SecretBlockSelection));
    }
    let Some(mut raw) = document.blocks.pop() else {
        return Err(PemError::new(PemErrorKind::SecretBlockSelection));
    };
    let encoded = SecretVec::from_vec(raw.take_body().into_vec());
    document.work.charge(encoded.len())?;
    let exposed = encoded.expose_secret();
    let decoded_len = candidate_decoded_len(exposed.as_ref())?;
    if decoded_len > limits.max_decoded_output_bytes() {
        return Err(PemError::new(PemErrorKind::DecodedOutputLimitExceeded));
    }
    let mut frame = SecretVecFrame::new(&base64_ng::STRICT_STANDARD_PADDED, decoded_len)
        .map_err(map_secret_frame_creation)?;
    document.work.charge(encoded.len())?;
    frame
        .update(&SecretInput::new(exposed.as_ref()))
        .map_err(|_| PemError::new(PemErrorKind::InvalidBody))?;
    let contents = frame
        .finish()
        .map_err(|_| PemError::new(PemErrorKind::InvalidBody))?;
    Ok(SecretPemBlock {
        label: raw.label,
        contents,
        report: document.report,
    })
}

fn candidate_decoded_len(encoded: &[u8]) -> Result<usize, PemError> {
    if encoded.len() < 4 || !encoded.len().is_multiple_of(4) {
        return Err(PemError::new(PemErrorKind::InvalidBody));
    }
    let padding = if encoded.ends_with(b"==") {
        2
    } else {
        usize::from(encoded.ends_with(b"="))
    };
    (encoded.len() / 4)
        .checked_mul(3)
        .and_then(|length| length.checked_sub(padding))
        .ok_or_else(|| PemError::new(PemErrorKind::LengthOverflow))
}

fn map_secret_frame_creation(error: SecretDecodeError) -> PemError {
    match error {
        SecretDecodeError::AllocationFailed => PemError::new(PemErrorKind::AllocationFailed),
        SecretDecodeError::LengthOverflow => PemError::new(PemErrorKind::LengthOverflow),
        _ => PemError::new(PemErrorKind::InternalInvariantViolation),
    }
}
