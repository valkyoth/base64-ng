//! Fixed-work scalar core for secret-bearing Base64 encoding.

use super::{
    alphabet::{STANDARD_ALPHABET, URL_SAFE_ALPHABET, ValidatedAlphabet},
    contracts::Progress,
    specifications::{CodecSettings, EncodePadding},
};

/// Error returned by a secret encoding frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecretEncodeError {
    /// The public cumulative input length exceeds the declared frame limit.
    InputTooLarge {
        /// Public input bytes presented so far.
        input_len: usize,
        /// Public maximum input bytes declared at construction.
        maximum_input_len: usize,
    },
    /// Secret output storage cannot hold the declared maximum encoded result.
    OutputFull {
        /// Required encoded bytes for the declared input bound.
        required: usize,
        /// Available secret output bytes.
        available: usize,
    },
    /// Secret input and output storage overlap.
    OverlappingBuffers,
    /// A byte-range end address cannot be represented by `usize`.
    AddressRangeOverflow,
    /// Encoded-length or source-position arithmetic overflowed.
    LengthOverflow,
    /// A preallocated heap encoder could not reserve its complete storage.
    #[cfg(feature = "alloc")]
    AllocationFailed,
    /// The encoder previously failed and is absorbing.
    Failed,
    /// The encoder has already completed successfully.
    Complete,
}

impl core::fmt::Display for SecretEncodeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InputTooLarge {
                input_len,
                maximum_input_len,
            } => write!(
                formatter,
                "secret input length {input_len} exceeds public frame limit {maximum_input_len}"
            ),
            Self::OutputFull {
                required,
                available,
            } => write!(
                formatter,
                "secret encoder requires {required} output bytes; storage has {available}"
            ),
            Self::OverlappingBuffers => {
                formatter.write_str("secret encoder input and output must be disjoint")
            }
            Self::AddressRangeOverflow => {
                formatter.write_str("secret encoder byte-range address overflows usize")
            }
            Self::LengthOverflow => formatter.write_str("secret encode length overflows usize"),
            #[cfg(feature = "alloc")]
            Self::AllocationFailed => {
                formatter.write_str("failed to reserve bounded secret encoder storage")
            }
            Self::Failed => formatter.write_str("secret encoder is in an absorbing failed state"),
            Self::Complete => formatter.write_str("secret encoder is already complete"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SecretEncodeError {}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Active,
    Failed,
    Complete,
}

#[derive(Clone, Copy)]
enum SecretMapper {
    Standard,
    UrlSafe,
    Scanned(ValidatedAlphabet),
}

impl SecretMapper {
    fn new(alphabet: ValidatedAlphabet) -> Self {
        if alphabet == STANDARD_ALPHABET {
            Self::Standard
        } else if alphabet == URL_SAFE_ALPHABET {
            Self::UrlSafe
        } else {
            Self::Scanned(alphabet)
        }
    }

    #[inline(never)]
    fn map(self, value: u8) -> u8 {
        match self {
            Self::Standard => secret_encode_ascii(value, b'+', b'/'),
            Self::UrlSafe => secret_encode_ascii(value, b'-', b'_'),
            Self::Scanned(alphabet) => secret_encode_scan(value, &alphabet),
        }
    }
}

/// Public metadata and fixed-work core shared by secret encoding frames.
///
/// Construction remains frame-owned so this state cannot be detached from
/// wiping secret output storage.
pub struct SecretEncoderState {
    mapper: SecretMapper,
    padding: EncodePadding,
    maximum_input_len: usize,
    maximum_encoded_len: usize,
    input_len: usize,
    output_len: usize,
    tail: [u8; 3],
    tail_len: usize,
    phase: Phase,
    #[cfg(test)]
    mapping_work: usize,
}

impl SecretEncoderState {
    pub(super) fn new(
        settings: CodecSettings,
        maximum_input_len: usize,
        output_capacity: usize,
    ) -> Result<Self, SecretEncodeError> {
        let padded = settings.encode_padding() == EncodePadding::Padded;
        let maximum_encoded_len = crate::checked_encoded_len(maximum_input_len, padded)
            .ok_or(SecretEncodeError::LengthOverflow)?;
        if maximum_encoded_len > output_capacity {
            return Err(SecretEncodeError::OutputFull {
                required: maximum_encoded_len,
                available: output_capacity,
            });
        }
        Ok(Self {
            mapper: SecretMapper::new(*settings.alphabet()),
            padding: settings.encode_padding(),
            maximum_input_len,
            maximum_encoded_len,
            input_len: 0,
            output_len: 0,
            tail: [0; 3],
            tail_len: 0,
            phase: Phase::Active,
            #[cfg(test)]
            mapping_work: 0,
        })
    }

    /// Returns the public maximum input bytes.
    #[must_use]
    pub const fn maximum_input_len(&self) -> usize {
        self.maximum_input_len
    }

    /// Returns the public maximum encoded bytes.
    #[must_use]
    pub const fn maximum_encoded_len(&self) -> usize {
        self.maximum_encoded_len
    }

    /// Returns the public input bytes accepted so far.
    #[must_use]
    pub const fn input_len(&self) -> usize {
        self.input_len
    }

    /// Returns the public encoded bytes initialized so far.
    #[must_use]
    pub const fn output_len(&self) -> usize {
        self.output_len
    }

    /// Returns whether this encoder has entered an absorbing failure.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self.phase, Phase::Failed)
    }

    pub(super) fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Progress, SecretEncodeError> {
        self.require_active()?;
        let attempted = self
            .input_len
            .checked_add(input.len())
            .ok_or_else(|| self.fail(SecretEncodeError::LengthOverflow))?;
        if attempted > self.maximum_input_len {
            return Err(self.fail(SecretEncodeError::InputTooLarge {
                input_len: attempted,
                maximum_input_len: self.maximum_input_len,
            }));
        }

        let before = self.output_len;
        for &byte in input {
            self.tail[self.tail_len] = byte;
            self.tail_len += 1;
            if self.tail_len == 3 {
                self.encode_complete_quantum(output)?;
            }
        }
        self.input_len = attempted;
        Ok(Progress::new(input.len(), self.output_len - before))
    }

    pub(super) fn finish(&mut self, output: &mut [u8]) -> Result<usize, SecretEncodeError> {
        self.require_active()?;
        if self.tail_len != 0 {
            self.encode_final_quantum(output)?;
        }
        crate::wipe_tail(output, self.output_len);
        self.phase = Phase::Complete;
        Ok(self.output_len)
    }

    fn encode_complete_quantum(&mut self, output: &mut [u8]) -> Result<(), SecretEncodeError> {
        let end = self
            .output_len
            .checked_add(4)
            .ok_or_else(|| self.fail(SecretEncodeError::LengthOverflow))?;
        if end > output.len() {
            return Err(self.fail(SecretEncodeError::OutputFull {
                required: end,
                available: output.len(),
            }));
        }
        let values = six_bit_values(self.tail);
        for (slot, value) in output[self.output_len..end].iter_mut().zip(values) {
            *slot = self.map(value);
        }
        self.output_len = end;
        self.clear_tail();
        Ok(())
    }

    fn encode_final_quantum(&mut self, output: &mut [u8]) -> Result<(), SecretEncodeError> {
        let first = self.tail[0];
        let second = self.tail[1];
        let values = [
            first >> 2,
            ((first & 0x03) << 4) | (second >> 4),
            (second & 0x0f) << 2,
        ];
        let produced = final_quantum_output_len(self.tail_len, self.padding);
        let end = self
            .output_len
            .checked_add(produced)
            .ok_or_else(|| self.fail(SecretEncodeError::LengthOverflow))?;
        if end > output.len() {
            return Err(self.fail(SecretEncodeError::OutputFull {
                required: end,
                available: output.len(),
            }));
        }

        output[self.output_len] = self.map(values[0]);
        output[self.output_len + 1] = self.map(values[1]);
        if self.tail_len == 2 {
            output[self.output_len + 2] = self.map(values[2]);
        } else if self.padding == EncodePadding::Padded {
            output[self.output_len + 2] = b'=';
        }
        if self.padding == EncodePadding::Padded {
            output[self.output_len + 3] = b'=';
        }
        self.output_len = end;
        self.clear_tail();
        Ok(())
    }

    fn map(&mut self, value: u8) -> u8 {
        #[cfg(test)]
        {
            self.mapping_work += match self.mapper {
                SecretMapper::Standard | SecretMapper::UrlSafe => 1,
                SecretMapper::Scanned(_) => 64,
            };
        }
        self.mapper.map(value)
    }

    fn clear_tail(&mut self) {
        crate::wipe_bytes(&mut self.tail);
        self.tail_len = 0;
    }

    fn require_active(&self) -> Result<(), SecretEncodeError> {
        match self.phase {
            Phase::Active => Ok(()),
            Phase::Failed => Err(SecretEncodeError::Failed),
            Phase::Complete => Err(SecretEncodeError::Complete),
        }
    }

    fn fail(&mut self, error: SecretEncodeError) -> SecretEncodeError {
        self.phase = Phase::Failed;
        self.clear_tail();
        error
    }

    pub(super) fn latch_external_failure(&mut self) {
        self.phase = Phase::Failed;
        self.clear_tail();
    }
}

impl Drop for SecretEncoderState {
    fn drop(&mut self) {
        self.clear_tail();
        self.input_len = 0;
        self.output_len = 0;
    }
}

impl core::fmt::Debug for SecretEncoderState {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretEncoderState")
            .field("tail", &"<redacted>")
            .field("input_len", &self.input_len)
            .field("output_len", &self.output_len)
            .field("maximum_input_len", &self.maximum_input_len)
            .field("maximum_encoded_len", &self.maximum_encoded_len)
            .field("failed", &self.is_failed())
            .finish_non_exhaustive()
    }
}

fn six_bit_values(input: [u8; 3]) -> [u8; 4] {
    [
        input[0] >> 2,
        ((input[0] & 0x03) << 4) | (input[1] >> 4),
        ((input[1] & 0x0f) << 2) | (input[2] >> 6),
        input[2] & 0x3f,
    ]
}

const fn final_quantum_output_len(tail_len: usize, padding: EncodePadding) -> usize {
    if matches!(padding, EncodePadding::Padded) {
        4
    } else {
        tail_len + 1
    }
}

#[inline(never)]
fn secret_encode_ascii(value: u8, value_62: u8, value_63: u8) -> u8 {
    let upper = crate::ct_mask_lt_u8(value, 26);
    let lower = crate::ct_mask_lt_u8(value.wrapping_sub(26), 26);
    let digit = crate::ct_mask_lt_u8(value.wrapping_sub(52), 10);
    let is_62 = crate::ct_mask_eq_u8(value, 62);
    let is_63 = crate::ct_mask_eq_u8(value, 63);
    core::hint::black_box(
        (value.wrapping_add(b'A') & upper)
            | (value.wrapping_sub(26).wrapping_add(b'a') & lower)
            | (value.wrapping_sub(52).wrapping_add(b'0') & digit)
            | (value_62 & is_62)
            | (value_63 & is_63),
    )
}

#[inline(never)]
fn secret_encode_scan(value: u8, alphabet: &ValidatedAlphabet) -> u8 {
    let mut output = 0u8;
    let mut candidate = 0u8;
    while candidate < 64 {
        let selected = core::hint::black_box(crate::ct_mask_eq_u8(
            core::hint::black_box(value),
            core::hint::black_box(candidate),
        ));
        output = crate::ct_accumulate_u8(
            output,
            core::hint::black_box(alphabet.as_array()[usize::from(candidate)] & selected),
        );
        candidate += 1;
    }
    output
}

pub(super) fn require_disjoint(left: &[u8], right: &[u8]) -> Result<(), SecretEncodeError> {
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
) -> Result<(), SecretEncodeError> {
    let left_end = left_start
        .checked_add(left_len)
        .ok_or(SecretEncodeError::AddressRangeOverflow)?;
    let right_end = right_start
        .checked_add(right_len)
        .ok_or(SecretEncodeError::AddressRangeOverflow)?;
    if left_len != 0 && right_len != 0 && left_start < right_end && right_start < left_end {
        Err(SecretEncodeError::OverlappingBuffers)
    } else {
        Ok(())
    }
}

#[cfg(kani)]
pub(crate) const fn final_quantum_output_len_for_proof(
    tail_len: usize,
    padding: EncodePadding,
) -> usize {
    final_quantum_output_len(tail_len, padding)
}

#[cfg(kani)]
pub(crate) fn require_disjoint_ranges_for_proof(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), SecretEncodeError> {
    require_disjoint_ranges(left_start, left_len, right_start, right_len)
}

#[cfg(test)]
pub(super) fn require_disjoint_ranges_for_test(
    left_start: usize,
    left_len: usize,
    right_start: usize,
    right_len: usize,
) -> Result<(), SecretEncodeError> {
    require_disjoint_ranges(left_start, left_len, right_start, right_len)
}

#[cfg(test)]
impl SecretEncoderState {
    pub(super) const fn mapping_work_for_test(&self) -> usize {
        self.mapping_work
    }
}

#[cfg(test)]
pub(super) fn map_value_for_test(settings: CodecSettings, value: u8) -> u8 {
    SecretMapper::new(*settings.alphabet()).map(value)
}
