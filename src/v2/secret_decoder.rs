//! Fixed-work incremental core for bounded secret decoding frames.

use super::{
    contracts::Progress,
    specifications::{CodecSettings, DecodePadding},
};

/// Maximum decoded capacity accepted by a stack-backed secret frame.
pub const MAX_SECRET_STACK_DECODED: usize = 1_024;

/// Opaque error returned by a bounded secret decode frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecretDecodeError {
    /// The public cumulative encoded length exceeds the frame limit.
    InputTooLarge {
        /// Public encoded length presented so far.
        input_len: usize,
        /// Maximum encoded length derived from decoded capacity and padding.
        maximum_encoded_len: usize,
    },
    /// Caller-provided storage cannot hold the declared decoded capacity.
    OutputFull {
        /// Public declared decoded capacity.
        required: usize,
        /// Public available bytes.
        available: usize,
    },
    /// Private staging overlaps encoded input or final output.
    OverlappingBuffers,
    /// A byte-range end address cannot be represented by `usize`.
    AddressRangeOverflow,
    /// The codec permits a compatibility form excluded from secret decoding.
    UnsupportedPolicy,
    /// Fixed-work validation rejected the input without localized details.
    InvalidInput,
    /// Capacity or source-position arithmetic overflowed.
    LengthOverflow,
    /// A preallocated heap frame could not reserve its complete storage.
    #[cfg(feature = "alloc")]
    AllocationFailed,
    /// The state previously failed and is absorbing.
    Failed,
    /// The state has already completed successfully.
    Complete,
}

impl core::fmt::Display for SecretDecodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputTooLarge {
                input_len,
                maximum_encoded_len,
            } => write!(
                formatter,
                "secret input length {input_len} exceeds public frame limit {maximum_encoded_len}"
            ),
            Self::OutputFull {
                required,
                available,
            } => write!(
                formatter,
                "secret frame requires {required} decoded bytes; storage has {available}"
            ),
            Self::OverlappingBuffers => {
                formatter.write_str("secret input, staging, and final output must be disjoint")
            }
            Self::AddressRangeOverflow => {
                formatter.write_str("secret frame byte-range address overflows usize")
            }
            Self::UnsupportedPolicy => {
                formatter.write_str("codec policy is not eligible for secret decoding")
            }
            Self::InvalidInput => formatter.write_str("invalid secret base64 input"),
            Self::LengthOverflow => formatter.write_str("secret frame length overflows usize"),
            #[cfg(feature = "alloc")]
            Self::AllocationFailed => {
                formatter.write_str("failed to reserve bounded secret frame storage")
            }
            Self::Failed => formatter.write_str("secret decoder is in an absorbing failed state"),
            Self::Complete => formatter.write_str("secret decoder is already complete"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SecretDecodeError {}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Active,
    Failed,
    Complete,
}

/// Public metadata and fixed-work core shared by secret decode frames.
///
/// The state is deliberately not directly constructible. Use
/// [`super::secret::SecretArrayFrame`], [`super::secret::SecretFrame`], or
/// `SecretVecFrame`; those owners bind the state to disjoint private staging
/// and final output for the complete operation.
pub struct SecretDecoderState {
    settings: CodecSettings,
    maximum_decoded_len: usize,
    maximum_encoded_len: usize,
    input_len: usize,
    staged_len: usize,
    pending_bytes: [u8; 4],
    pending_values: [u8; 4],
    pending_valid: [u8; 4],
    pending_len: usize,
    invalid: u8,
    phase: Phase,
    #[cfg(test)]
    symbol_scans: usize,
}

impl SecretDecoderState {
    pub(super) fn new(
        settings: CodecSettings,
        maximum_decoded_len: usize,
    ) -> Result<Self, SecretDecodeError> {
        if !settings.permits_secret_processing() {
            return Err(SecretDecodeError::UnsupportedPolicy);
        }
        let padded = settings.decode_padding() == DecodePadding::RequireCanonical;
        let maximum_encoded_len = crate::checked_encoded_len(maximum_decoded_len, padded)
            .ok_or(SecretDecodeError::LengthOverflow)?;
        Ok(Self {
            settings,
            maximum_decoded_len,
            maximum_encoded_len,
            input_len: 0,
            staged_len: 0,
            pending_bytes: [0; 4],
            pending_values: [0; 4],
            pending_valid: [0; 4],
            pending_len: 0,
            invalid: 0,
            phase: Phase::Active,
            #[cfg(test)]
            symbol_scans: 0,
        })
    }

    /// Returns the public maximum decoded bytes.
    #[must_use]
    pub const fn maximum_decoded_len(&self) -> usize {
        self.maximum_decoded_len
    }

    /// Returns the public maximum encoded bytes for this padding policy.
    #[must_use]
    pub const fn maximum_encoded_len(&self) -> usize {
        self.maximum_encoded_len
    }

    /// Returns the public encoded bytes accepted so far.
    #[must_use]
    pub const fn input_len(&self) -> usize {
        self.input_len
    }

    /// Returns whether this state has entered an absorbing failure.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self.phase, Phase::Failed)
    }

    pub(super) fn update(
        &mut self,
        input: &[u8],
        staging: &mut [u8],
    ) -> Result<Progress, SecretDecodeError> {
        self.require_active()?;
        let attempted = self
            .input_len
            .checked_add(input.len())
            .ok_or_else(|| self.fail(SecretDecodeError::LengthOverflow))?;
        if attempted > self.maximum_encoded_len {
            return Err(self.fail(SecretDecodeError::InputTooLarge {
                input_len: attempted,
                maximum_encoded_len: self.maximum_encoded_len,
            }));
        }

        for &byte in input {
            if self.pending_len == 4 {
                self.commit_nonfinal(staging)?;
            }
            let slot = self.pending_len;
            let (value, valid) = decode_symbol(self.settings, byte);
            #[cfg(test)]
            {
                self.symbol_scans += 64;
            }
            self.pending_bytes[slot] = byte;
            self.pending_values[slot] = value;
            self.pending_valid[slot] = valid;
            self.pending_len += 1;
        }
        self.input_len = attempted;
        Ok(Progress::new(input.len(), 0))
    }

    pub(super) fn finish(&mut self) -> Result<FinalCandidate, SecretDecodeError> {
        self.require_active()?;
        let candidate = match self.settings.decode_padding() {
            DecodePadding::RequireCanonical => self.finish_padded(),
            DecodePadding::Forbid => self.finish_unpadded(),
            DecodePadding::Indifferent => {
                return Err(self.fail(SecretDecodeError::UnsupportedPolicy));
            }
        };
        let remaining = self.maximum_decoded_len - self.staged_len;
        let public_remaining = u8::try_from(remaining.min(3)).unwrap_or(3);
        let candidate_len = u8::try_from(candidate.len).unwrap_or(3);
        self.invalid = accumulate(
            self.invalid,
            crate::ct_mask_lt_u8(public_remaining, candidate_len),
        );
        crate::ct_error_gate_barrier(self.invalid, 0);
        if core::hint::black_box(self.invalid) != 0 {
            return Err(self.fail(SecretDecodeError::InvalidInput));
        }
        self.phase = Phase::Complete;
        Ok(candidate)
    }

    fn commit_nonfinal(&mut self, staging: &mut [u8]) -> Result<(), SecretDecodeError> {
        let Some(end) = self.staged_len.checked_add(3) else {
            return Err(self.fail(SecretDecodeError::LengthOverflow));
        };
        if end > staging.len() || end > self.maximum_decoded_len {
            return Err(self.fail(SecretDecodeError::OutputFull {
                required: end,
                available: staging.len().min(self.maximum_decoded_len),
            }));
        }
        let candidate = candidate_bytes(self.pending_values);
        staging[self.staged_len..end].copy_from_slice(&candidate);
        self.staged_len = end;
        for valid in self.pending_valid {
            self.invalid = accumulate(self.invalid, !valid);
        }
        self.clear_pending();
        Ok(())
    }

    fn finish_padded(&mut self) -> FinalCandidate {
        if self.input_len == 0 {
            return FinalCandidate::empty(self.staged_len);
        }
        self.invalid = accumulate(
            self.invalid,
            crate::ct_mask_nonzero_u8(u8::from(self.pending_len != 4)),
        );
        let equals_third = crate::ct_mask_eq_u8(self.pending_bytes[2], b'=');
        let equals_fourth = crate::ct_mask_eq_u8(self.pending_bytes[3], b'=');
        let no_padding = !equals_third & !equals_fourth;
        let one_padding = !equals_third & equals_fourth;
        let two_padding = equals_third & equals_fourth;
        let malformed_padding = equals_third & !equals_fourth;
        let require_third = no_padding | one_padding;
        self.invalid = accumulate(self.invalid, !self.pending_valid[0]);
        self.invalid = accumulate(self.invalid, !self.pending_valid[1]);
        self.invalid = accumulate(self.invalid, !self.pending_valid[2] & require_third);
        self.invalid = accumulate(self.invalid, !self.pending_valid[3] & no_padding);
        self.invalid = accumulate(self.invalid, malformed_padding);
        self.invalid = accumulate(
            self.invalid,
            crate::ct_mask_nonzero_u8(self.pending_values[1] & 0x0f) & two_padding,
        );
        self.invalid = accumulate(
            self.invalid,
            crate::ct_mask_nonzero_u8(self.pending_values[2] & 0x03) & one_padding,
        );
        let padding = usize::from((equals_third & 1) + (equals_fourth & 1));
        FinalCandidate::new(
            self.staged_len,
            candidate_bytes(self.pending_values),
            3 - padding,
        )
    }

    fn finish_unpadded(&mut self) -> FinalCandidate {
        let candidate = candidate_bytes(self.pending_values);
        let final_len = match self.pending_len {
            0 => 0,
            2 => {
                self.invalid = accumulate(self.invalid, !self.pending_valid[0]);
                self.invalid = accumulate(self.invalid, !self.pending_valid[1]);
                self.invalid = accumulate(
                    self.invalid,
                    crate::ct_mask_nonzero_u8(self.pending_values[1] & 0x0f),
                );
                1
            }
            3 => {
                self.invalid = accumulate(self.invalid, !self.pending_valid[0]);
                self.invalid = accumulate(self.invalid, !self.pending_valid[1]);
                self.invalid = accumulate(self.invalid, !self.pending_valid[2]);
                self.invalid = accumulate(
                    self.invalid,
                    crate::ct_mask_nonzero_u8(self.pending_values[2] & 0x03),
                );
                2
            }
            4 => {
                for valid in self.pending_valid {
                    self.invalid = accumulate(self.invalid, !valid);
                }
                3
            }
            _ => {
                self.invalid = accumulate(self.invalid, 0xff);
                0
            }
        };
        FinalCandidate::new(self.staged_len, candidate, final_len)
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending_bytes);
        crate::wipe_bytes(&mut self.pending_values);
        crate::wipe_bytes(&mut self.pending_valid);
        self.pending_len = 0;
    }

    fn require_active(&self) -> Result<(), SecretDecodeError> {
        match self.phase {
            Phase::Active => Ok(()),
            Phase::Failed => Err(SecretDecodeError::Failed),
            Phase::Complete => Err(SecretDecodeError::Complete),
        }
    }

    fn fail(&mut self, error: SecretDecodeError) -> SecretDecodeError {
        self.phase = Phase::Failed;
        error
    }

    pub(super) fn latch_external_failure(&mut self) {
        self.phase = Phase::Failed;
        self.clear_pending();
    }
}

impl Drop for SecretDecoderState {
    fn drop(&mut self) {
        self.clear_pending();
        self.invalid = 0;
        self.input_len = 0;
        self.staged_len = 0;
    }
}

impl core::fmt::Debug for SecretDecoderState {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretDecoderState")
            .field("pending", &"<redacted>")
            .field("input_len", &self.input_len)
            .field("maximum_encoded_len", &self.maximum_encoded_len)
            .field("maximum_decoded_len", &self.maximum_decoded_len)
            .field("failed", &self.is_failed())
            .finish_non_exhaustive()
    }
}

pub(super) struct FinalCandidate {
    pub(super) staged_len: usize,
    pub(super) bytes: [u8; 3],
    pub(super) len: usize,
}

impl FinalCandidate {
    const fn new(staged_len: usize, bytes: [u8; 3], len: usize) -> Self {
        Self {
            staged_len,
            bytes,
            len,
        }
    }

    const fn empty(staged_len: usize) -> Self {
        Self::new(staged_len, [0; 3], 0)
    }

    pub(super) const fn written(&self) -> usize {
        self.staged_len + self.len
    }
}

impl Drop for FinalCandidate {
    fn drop(&mut self) {
        crate::wipe_bytes(&mut self.bytes);
        self.len = 0;
        self.staged_len = 0;
    }
}

#[inline(never)]
fn decode_symbol(settings: CodecSettings, byte: u8) -> (u8, u8) {
    let mut decoded = 0u8;
    let mut valid = 0u8;
    let mut candidate = 0u8;
    while candidate < 64 {
        let matches = core::hint::black_box(crate::ct_mask_eq_u8(
            core::hint::black_box(byte),
            core::hint::black_box(settings.alphabet().as_array()[usize::from(candidate)]),
        ));
        decoded = accumulate(decoded, candidate & matches);
        valid = accumulate(valid, matches);
        candidate += 1;
    }
    (decoded, valid)
}

fn candidate_bytes(values: [u8; 4]) -> [u8; 3] {
    [
        (values[0] << 2) | (values[1] >> 4),
        (values[1] << 4) | (values[2] >> 2),
        (values[2] << 6) | values[3],
    ]
}

fn accumulate(accumulator: u8, value: u8) -> u8 {
    crate::ct_accumulate_u8(accumulator, value)
}

pub(super) fn require_disjoint(left: &[u8], right: &[u8]) -> Result<(), SecretDecodeError> {
    require_disjoint_ranges(
        left.as_ptr() as usize,
        left.len(),
        right.as_ptr() as usize,
        right.len(),
    )
}

fn require_disjoint_ranges(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), SecretDecodeError> {
    let left_end = left_start
        .checked_add(left_len)
        .ok_or(SecretDecodeError::AddressRangeOverflow)?;
    let right_end = right_start
        .checked_add(right_len)
        .ok_or(SecretDecodeError::AddressRangeOverflow)?;
    if left_len != 0 && right_len != 0 && left_start < right_end && right_start < left_end {
        Err(SecretDecodeError::OverlappingBuffers)
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn require_disjoint_ranges_for_test(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), SecretDecodeError> {
    require_disjoint_ranges(left_start, left_len, right_start, right_len)
}

#[cfg(test)]
impl SecretDecoderState {
    pub(super) const fn symbol_scans_for_test(&self) -> usize {
        self.symbol_scans
    }
}
