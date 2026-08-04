//! Bounded owners around the fixed-work secret decoder state.

#[cfg(feature = "alloc")]
use super::SecretVec;
use super::{SecretArray, SecretInput, SecretOutput};
use crate::v2::{
    Progress,
    secret_decoder::{
        FinalCandidate, MAX_SECRET_STACK_DECODED, SecretDecodeError, SecretDecoderState,
        require_disjoint,
    },
    specifications::{Base64, Codec},
};

/// Stack-backed secret decode with disjoint private and final arrays.
pub struct SecretArrayFrame<const N: usize> {
    state: SecretDecoderState,
    staging: [u8; N],
    output: [u8; N],
}

impl<const N: usize> SecretArrayFrame<N> {
    const CAPACITY_ASSERT: () = enforce_stack_capacity::<N>();

    /// Creates one empty fixed-capacity secret decode frame.
    pub fn new<S: Codec>(codec: &Base64<S>) -> Result<Self, SecretDecodeError> {
        const { enforce_stack_capacity::<N>() }
        let () = Self::CAPACITY_ASSERT;
        Ok(Self {
            state: SecretDecoderState::new(codec.settings(), N)?,
            staging: [0; N],
            output: [0; N],
        })
    }

    /// Consumes one classified chunk without releasing plaintext.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretDecodeError> {
        if let Err(error) = self.check_input(input.classified_bytes()) {
            self.state.latch_external_failure();
            self.fail_storage();
            return Err(error);
        }
        match self
            .state
            .update(input.classified_bytes(), &mut self.staging)
        {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.fail_storage();
                Err(error)
            }
        }
    }

    /// Applies the result gate and returns secret output only on success.
    pub fn finish(mut self) -> Result<SecretArray<N>, SecretDecodeError> {
        let final_candidate = match self.state.finish() {
            Ok(candidate) => candidate,
            Err(error) => {
                self.fail_storage();
                return Err(error);
            }
        };
        release(&self.staging, &final_candidate, &mut self.output);
        crate::wipe_bytes(&mut self.staging);
        let output = core::mem::replace(&mut self.output, [0; N]);
        SecretArray::from_frame(output, final_candidate.written()).map_err(|error| {
            SecretDecodeError::OutputFull {
                required: error.length(),
                available: error.capacity(),
            }
        })
    }

    /// Returns public decoder metadata without exposing staged bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretDecoderState {
        &self.state
    }

    fn check_input(&self, input: &[u8]) -> Result<(), SecretDecodeError> {
        require_disjoint(input, &self.staging)?;
        require_disjoint(input, &self.output)
    }

    fn fail_storage(&mut self) {
        crate::wipe_bytes(&mut self.staging);
        crate::wipe_bytes(&mut self.output);
    }

    #[cfg(test)]
    pub(crate) const fn storage_for_test(&self) -> (&[u8; N], &[u8; N]) {
        (&self.staging, &self.output)
    }
}

#[allow(clippy::manual_assert)]
const fn enforce_stack_capacity<const N: usize>() {
    if N > MAX_SECRET_STACK_DECODED {
        panic!("SecretArrayFrame decoded capacity exceeds 1024-byte stack limit");
    }
}

impl<const N: usize> Drop for SecretArrayFrame<N> {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

impl<const N: usize> core::fmt::Debug for SecretArrayFrame<N> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretArrayFrame")
            .field("storage", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// Borrowed frame over caller-provided private staging and final output.
pub struct SecretFrame<'a> {
    state: SecretDecoderState,
    staging: &'a mut [u8],
    output: Option<&'a mut [u8]>,
}

impl<'a> SecretFrame<'a> {
    /// Binds disjoint staging and final output before decoding begins.
    pub fn new<S: Codec>(
        codec: &Base64<S>,
        maximum_decoded_len: usize,
        staging: &'a mut [u8],
        output: &'a mut [u8],
    ) -> Result<Self, SecretDecodeError> {
        let state = SecretDecoderState::new(codec.settings(), maximum_decoded_len)?;
        require_capacity(maximum_decoded_len, staging.len())?;
        require_capacity(maximum_decoded_len, output.len())?;
        require_disjoint(staging, output)?;
        crate::wipe_bytes(staging);
        crate::wipe_bytes(output);
        Ok(Self {
            state,
            staging,
            output: Some(output),
        })
    }

    /// Consumes one classified chunk without writing final output.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretDecodeError> {
        if let Err(error) = self.check_input(input.classified_bytes()) {
            self.state.latch_external_failure();
            self.fail_storage();
            return Err(error);
        }
        match self.state.update(input.classified_bytes(), self.staging) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.fail_storage();
                Err(error)
            }
        }
    }

    /// Applies the result gate and returns a wiping borrowed output guard.
    pub fn finish(mut self) -> Result<SecretOutput<'a>, SecretDecodeError> {
        let final_candidate = match self.state.finish() {
            Ok(candidate) => candidate,
            Err(error) => {
                self.fail_storage();
                return Err(error);
            }
        };
        let Some(output) = self.output.take() else {
            self.fail_storage();
            return Err(SecretDecodeError::Failed);
        };
        let available = output.len();
        release(self.staging, &final_candidate, output);
        crate::wipe_bytes(self.staging);
        SecretOutput::from_initialized(output, final_candidate.written()).map_err(|_| {
            SecretDecodeError::OutputFull {
                required: final_candidate.written(),
                available,
            }
        })
    }

    /// Returns public decoder metadata without exposing staged bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretDecoderState {
        &self.state
    }

    fn check_input(&self, input: &[u8]) -> Result<(), SecretDecodeError> {
        require_disjoint(input, self.staging)?;
        require_disjoint(input, self.output.as_deref().unwrap_or(&[]))
    }

    fn fail_storage(&mut self) {
        crate::wipe_bytes(self.staging);
        if let Some(output) = self.output.as_deref_mut() {
            crate::wipe_bytes(output);
        }
    }
}

impl Drop for SecretFrame<'_> {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

impl core::fmt::Debug for SecretFrame<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretFrame")
            .field("storage", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// Heap frame whose staging and final allocation are reserved before update.
#[cfg(feature = "alloc")]
pub struct SecretVecFrame {
    state: SecretDecoderState,
    staging: alloc::vec::Vec<u8>,
    output: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl SecretVecFrame {
    /// Preallocates both bounded ranges before plaintext can materialize.
    pub fn new<S: Codec>(
        codec: &Base64<S>,
        maximum_decoded_len: usize,
    ) -> Result<Self, SecretDecodeError> {
        let state = SecretDecoderState::new(codec.settings(), maximum_decoded_len)?;
        Ok(Self {
            state,
            staging: allocate_zeroed(maximum_decoded_len)?,
            output: allocate_zeroed(maximum_decoded_len)?,
        })
    }

    /// Consumes one classified chunk without releasing plaintext.
    pub fn update(&mut self, input: &SecretInput<'_>) -> Result<Progress, SecretDecodeError> {
        if let Err(error) = require_disjoint(input.classified_bytes(), &self.staging)
            .and_then(|()| require_disjoint(input.classified_bytes(), &self.output))
        {
            self.state.latch_external_failure();
            self.fail_storage();
            return Err(error);
        }
        match self
            .state
            .update(input.classified_bytes(), &mut self.staging)
        {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.fail_storage();
                Err(error)
            }
        }
    }

    /// Applies the result gate and returns bounded secret heap storage.
    pub fn finish(mut self) -> Result<SecretVec, SecretDecodeError> {
        let final_candidate = match self.state.finish() {
            Ok(candidate) => candidate,
            Err(error) => {
                self.fail_storage();
                return Err(error);
            }
        };
        release(&self.staging, &final_candidate, &mut self.output);
        crate::wipe_bytes(&mut self.staging);
        let output = core::mem::take(&mut self.output);
        Ok(SecretVec::from_frame(output, final_candidate.written()))
    }

    /// Returns public decoder metadata without exposing staged bytes.
    #[must_use]
    pub const fn state(&self) -> &SecretDecoderState {
        &self.state
    }

    #[cfg(test)]
    pub(crate) fn allocation_snapshot(&self) -> ((*const u8, usize), (*const u8, usize)) {
        (
            (self.staging.as_ptr(), self.staging.capacity()),
            (self.output.as_ptr(), self.output.capacity()),
        )
    }

    fn fail_storage(&mut self) {
        crate::wipe_bytes(&mut self.staging);
        crate::wipe_bytes(&mut self.output);
    }
}

#[cfg(feature = "alloc")]
impl Drop for SecretVecFrame {
    fn drop(&mut self) {
        self.fail_storage();
    }
}

#[cfg(feature = "alloc")]
impl core::fmt::Debug for SecretVecFrame {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretVecFrame")
            .field("storage", &"<redacted>")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

fn require_capacity(required: usize, available: usize) -> Result<(), SecretDecodeError> {
    if required > available {
        Err(SecretDecodeError::OutputFull {
            required,
            available,
        })
    } else {
        Ok(())
    }
}

fn release(staging: &[u8], final_candidate: &FinalCandidate, output: &mut [u8]) {
    let staged_len = final_candidate.staged_len;
    output[..staged_len].copy_from_slice(&staging[..staged_len]);
    output[staged_len..final_candidate.written()]
        .copy_from_slice(&final_candidate.bytes[..final_candidate.len]);
    crate::wipe_tail(output, final_candidate.written());
}

#[cfg(feature = "alloc")]
fn allocate_zeroed(capacity: usize) -> Result<alloc::vec::Vec<u8>, SecretDecodeError> {
    let mut bytes = alloc::vec::Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| SecretDecodeError::AllocationFailed)?;
    bytes.resize(capacity, 0);
    Ok(bytes)
}

/// Constructs a stack-backed secret frame while enforcing its capacity limit.
#[macro_export]
macro_rules! secret_array_frame {
    ($codec:expr, $capacity:expr) => {{ $crate::secret::SecretArrayFrame::<$capacity>::new(&$codec) }};
}
