use base64_ng::secret::{SecretInput, SecretVec, SecretVecFrame};

use crate::{
    ArmorHeader, ArmorType, ChecksumPolicy, ChecksumStatus, OpenPgpError, OpenPgpErrorKind,
    OpenPgpLimits,
    parser::{BodyValidation, checksum_status, enforce_checksum, parse_raw_document},
};

/// One expected-type armor payload in clear-on-drop secret storage.
pub struct SecretArmorBlock {
    kind: ArmorType,
    headers: alloc::vec::Vec<ArmorHeader>,
    contents: SecretVec,
    checksum: ChecksumStatus,
}

impl SecretArmorBlock {
    /// Returns the public armor type.
    #[must_use]
    pub const fn kind(&self) -> ArmorType {
        self.kind
    }
    /// Returns public armor headers in source order.
    #[must_use]
    pub fn headers(&self) -> &[ArmorHeader] {
        &self.headers
    }
    /// Returns clear-on-drop storage for explicit caller exposure.
    #[must_use]
    pub const fn contents(&self) -> &SecretVec {
        &self.contents
    }
    /// Consumes the block and returns its clear-on-drop payload storage.
    #[must_use]
    pub fn into_contents(self) -> SecretVec {
        self.contents
    }
    /// Returns the observed checksum state.
    #[must_use]
    pub const fn checksum_status(&self) -> ChecksumStatus {
        self.checksum
    }
}

impl core::fmt::Debug for SecretArmorBlock {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretArmorBlock")
            .field("kind", &self.kind)
            .field("header_count", &self.headers.len())
            .field("contents", &"<redacted>")
            .field("decoded_len", &self.contents.len())
            .field("checksum", &self.checksum)
            .finish()
    }
}

/// Parses exactly one expected armor type into clear-on-drop secret storage.
///
/// The compacted Base64 body is wiped on drop. Body-symbol, padding, and
/// canonical-tail validation are deferred to the core fixed-work secret
/// decoder, which stages plaintext and releases it only after the result gate.
/// Public armor framing, headers, line limits, block selection, and checksum
/// metadata remain ordinary parsing work before or after that boundary.
///
/// # Errors
///
/// Returns [`OpenPgpError`] for grammar, limits, selection, allocation,
/// checksum-policy, or opaque secret-decoder failure.
pub fn parse_secret_armor_block(
    input: &[u8],
    expected_kind: ArmorType,
    limits: OpenPgpLimits,
    checksum_policy: ChecksumPolicy,
) -> Result<SecretArmorBlock, OpenPgpError> {
    let mut document = parse_raw_document(input, limits, BodyValidation::DeferredSecret)?;
    if document.blocks.len() != 1 || document.blocks[0].kind != expected_kind {
        return Err(OpenPgpError::new(OpenPgpErrorKind::SecretBlockSelection));
    }
    let Some(mut raw) = document.blocks.pop() else {
        return Err(OpenPgpError::new(OpenPgpErrorKind::SecretBlockSelection));
    };
    let encoded = SecretVec::from_vec(raw.take_body().into_vec());
    let maximum = encoded
        .len()
        .checked_add(3)
        .map(|length| length / 4)
        .and_then(|quanta| quanta.checked_mul(3))
        .ok_or_else(|| OpenPgpError::new(OpenPgpErrorKind::LengthOverflow))?;
    if maximum > limits.max_decoded_output_bytes() {
        return Err(OpenPgpError::new(
            OpenPgpErrorKind::DecodedOutputLimitExceeded,
        ));
    }
    let mut frame = SecretVecFrame::new(&base64_ng::STRICT_STANDARD_PADDED, maximum)
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::AllocationFailed))?;
    let exposed_encoded = encoded.expose_secret();
    frame
        .update(&SecretInput::new(exposed_encoded.as_ref()))
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::InvalidBody))?;
    let contents = frame
        .finish()
        .map_err(|_| OpenPgpError::new(OpenPgpErrorKind::InvalidBody))?;
    let checksum = {
        let exposed_contents = contents.expose_secret();
        checksum_status(raw.checksum, exposed_contents.as_ref())
    };
    enforce_checksum(checksum_policy, checksum)?;
    Ok(SecretArmorBlock {
        kind: raw.kind,
        headers: raw.headers,
        contents,
        checksum,
    })
}
