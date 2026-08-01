//! Wiping owners around the fixed-work secret encoder state.

#[cfg(feature = "alloc")]
use super::SecretVec;
use super::{SecretArray, SecretInput, SecretOutput};
use crate::v2::{
    Base64, Codec, Progress,
    secret_encoder::{SecretEncodeError, SecretEncoderState, require_disjoint},
};

/// Stack-backed incremental secret encoder.
///
/// `CAP` is encoded output capacity. The public maximum input length is bound
/// separately when the encoder is constructed.
pub struct SecretArrayEncoder<const CAP: usize> {
    state: SecretEncoderState,
    output: [u8; CAP],
}

impl<const CAP: usize> SecretArrayEncoder<CAP> {
    const CAPACITY_ASSERT: () = enforce_stack_capacity::<CAP>();

    /// Creates an empty bounded encoder over wiping stack storage.
    pub fn new<S: Codec>(
        codec: &Base64<S>,
        maximum_input_len: usize,
    ) -> Result<Self, SecretEncodeError> {
        const { enforce_stack_capacity::<CAP>() }
        let () = Self::CAPACITY_ASSERT;
        Ok(Self {
            state: SecretEncoderState::new(codec.settings(), maximum_input_len, CAP)?,
            output: [0; CAP],
        })
    }

    /// Encodes one classified chunk into private secret output storage.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretEncodeError> {
        if let Err(error) = require_disjoint(input.classified_bytes(), &self.output) {
            self.state.latch_external_failure();
            self.fail_storage();
            return Err(error);
        }
        match self
            .state
            .update(input.classified_bytes(), &mut self.output)
        {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.fail_storage();
                Err(error)
            }
        }
    }

    /// Completes encoding and returns redacted wiping storage.
    pub fn finish(mut self) -> Result<SecretArray<CAP>, SecretEncodeError> {
        let written = match self.state.finish(&mut self.output) {
            Ok(written) => written,
            Err(error) => {
                self.fail_storage();
                return Err(error);
            }
        };
        let output = core::mem::replace(&mut self.output, [0; CAP]);
        SecretArray::from_frame(output, written).map_err(|error| SecretEncodeError::OutputFull {
            required: error.length(),
            available: error.capacity(),
        })
    }

    /// Encodes one complete classified input into a secret array.
    pub fn encode<S: Codec>(
        codec: &Base64<S>,
        input: &SecretInput<'_>,
    ) -> Result<SecretArray<CAP>, SecretEncodeError> {
        let mut encoder = Self::new(codec, input.len())?;
        encoder.update(input)?;
        encoder.finish()
    }

    /// Returns public encoder metadata without exposing output bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretEncoderState {
        &self.state
    }

    fn fail_storage(&mut self) {
        crate::wipe_bytes(&mut self.output);
    }

    #[cfg(test)]
    pub(crate) const fn storage_for_test(&self) -> &[u8; CAP] {
        &self.output
    }
}

#[allow(clippy::manual_assert)]
const fn enforce_stack_capacity<const CAP: usize>() {
    if CAP > crate::v2::secret_encoder::MAX_SECRET_STACK_ENCODED {
        panic!("SecretArrayEncoder encoded capacity exceeds 1368-byte stack limit");
    }
}

impl<const CAP: usize> Drop for SecretArrayEncoder<CAP> {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

impl<const CAP: usize> core::fmt::Debug for SecretArrayEncoder<CAP> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretArrayEncoder")
            .field("output", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// Incremental secret encoder over caller-provided protected output storage.
pub struct SecretEncoder<'a> {
    state: SecretEncoderState,
    output: Option<&'a mut [u8]>,
}

impl<'a> SecretEncoder<'a> {
    /// Binds one complete protected output range before accepting input.
    pub fn new<S: Codec>(
        codec: &Base64<S>,
        maximum_input_len: usize,
        output: &'a mut [u8],
    ) -> Result<Self, SecretEncodeError> {
        let state = SecretEncoderState::new(codec.settings(), maximum_input_len, output.len())?;
        crate::wipe_bytes(output);
        Ok(Self {
            state,
            output: Some(output),
        })
    }

    /// Encodes one classified chunk into the protected output range.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretEncodeError> {
        let output = self
            .output
            .as_deref_mut()
            .ok_or(SecretEncodeError::Failed)?;
        if let Err(error) = require_disjoint(input.classified_bytes(), output) {
            self.state.latch_external_failure();
            crate::wipe_bytes(output);
            return Err(error);
        }
        match self.state.update(input.classified_bytes(), output) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                crate::wipe_bytes(output);
                Err(error)
            }
        }
    }

    /// Completes encoding and returns a wiping borrowed output guard.
    pub fn finish(mut self) -> Result<SecretOutput<'a>, SecretEncodeError> {
        let Some(output) = self.output.take() else {
            return Err(SecretEncodeError::Failed);
        };
        let written = match self.state.finish(output) {
            Ok(written) => written,
            Err(error) => {
                crate::wipe_bytes(output);
                return Err(error);
            }
        };
        let available = output.len();
        SecretOutput::from_initialized(output, written).map_err(|_| SecretEncodeError::OutputFull {
            required: written,
            available,
        })
    }

    /// Returns public encoder metadata without exposing output bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretEncoderState {
        &self.state
    }

    fn fail_storage(&mut self) {
        if let Some(output) = self.output.as_deref_mut() {
            crate::wipe_bytes(output);
        }
    }
}

impl Drop for SecretEncoder<'_> {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

impl core::fmt::Debug for SecretEncoder<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretEncoder")
            .field("output", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// Preallocated heap-backed incremental secret encoder.
#[cfg(feature = "alloc")]
pub struct SecretVecEncoder {
    state: SecretEncoderState,
    output: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl SecretVecEncoder {
    /// Preallocates the complete encoded output before accepting input.
    pub fn new<S: Codec>(
        codec: &Base64<S>,
        maximum_input_len: usize,
    ) -> Result<Self, SecretEncodeError> {
        let padded = codec.settings().encode_padding() == crate::v2::EncodePadding::Padded;
        let required = crate::checked_encoded_len(maximum_input_len, padded)
            .ok_or(SecretEncodeError::LengthOverflow)?;
        let output = allocate_zeroed(required)?;
        Ok(Self {
            state: SecretEncoderState::new(codec.settings(), maximum_input_len, required)?,
            output,
        })
    }

    /// Encodes one classified chunk into preallocated secret storage.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretEncodeError> {
        if let Err(error) = require_disjoint(input.classified_bytes(), &self.output) {
            self.state.latch_external_failure();
            self.fail_storage();
            return Err(error);
        }
        match self
            .state
            .update(input.classified_bytes(), &mut self.output)
        {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.fail_storage();
                Err(error)
            }
        }
    }

    /// Completes encoding and returns redacted heap storage.
    pub fn finish(mut self) -> Result<SecretVec, SecretEncodeError> {
        let written = match self.state.finish(&mut self.output) {
            Ok(written) => written,
            Err(error) => {
                self.fail_storage();
                return Err(error);
            }
        };
        let output = core::mem::take(&mut self.output);
        Ok(SecretVec::from_frame(output, written))
    }

    /// Encodes one complete classified input into a secret vector.
    pub fn encode<S: Codec>(
        codec: &Base64<S>,
        input: &SecretInput<'_>,
    ) -> Result<SecretVec, SecretEncodeError> {
        let mut encoder = Self::new(codec, input.len())?;
        encoder.update(input)?;
        encoder.finish()
    }

    /// Returns public encoder metadata without exposing output bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretEncoderState {
        &self.state
    }

    fn fail_storage(&mut self) {
        crate::wipe_bytes(&mut self.output);
    }
}

#[cfg(feature = "alloc")]
impl Drop for SecretVecEncoder {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for SecretVecEncoder {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretVecEncoder")
            .field("output", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "alloc")]
fn allocate_zeroed(capacity: usize) -> Result<alloc::vec::Vec<u8>, SecretEncodeError> {
    let mut bytes = alloc::vec::Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| SecretEncodeError::AllocationFailed)?;
    bytes.resize(capacity, 0);
    Ok(bytes)
}

impl<S: Codec> Base64<S> {
    /// Encodes classified input into a bounded wiping array.
    pub fn encode_secret_array<const CAP: usize>(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<SecretArray<CAP>, SecretEncodeError> {
        SecretArrayEncoder::encode(self, input)
    }

    /// Encodes classified input into a wiping borrowed output guard.
    pub fn encode_secret_into<'a>(
        &self,
        input: &SecretInput<'_>,
        output: &'a mut [u8],
    ) -> Result<SecretOutput<'a>, SecretEncodeError> {
        require_disjoint(input.classified_bytes(), output)?;
        let mut encoder = SecretEncoder::new(self, input.len(), output)?;
        encoder.update(input)?;
        encoder.finish()
    }

    /// Encodes classified input into preallocated wiping heap storage.
    #[cfg(feature = "alloc")]
    pub fn encode_secret_vec(
        &self,
        input: &SecretInput<'_>,
    ) -> Result<SecretVec, SecretEncodeError> {
        SecretVecEncoder::encode(self, input)
    }
}
