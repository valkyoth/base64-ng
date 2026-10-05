//! Per-invocation ordinary decode observations, never deployment attestations.

use crate::{DecodeValidation, runtime::Backend};

// A zero-sized sink keeps report storage out of the ordinary writer's closure.
pub(crate) trait WriteObservation {
    fn unavailable(&mut self) {}
    fn checked(&mut self) {}
    fn written(&mut self, _backend: Backend) {}
    fn rejected(&mut self) {}
}

impl WriteObservation for () {}

impl WriteObservation for &mut DecodeReport {
    fn unavailable(&mut self) {
        self.fallback = DecodeFallback::BackendUnavailable;
    }
    fn checked(&mut self) {
        self.checked_output = cfg!(feature = "checked-backend");
    }
    fn written(&mut self, backend: Backend) {
        self.output_backend = backend;
    }
    fn rejected(&mut self) {
        self.fallback = DecodeFallback::BackendRejected;
    }
}

/// Validator that accepted a successful ordinary decode invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DecodeValidator {
    /// Empty input needed no grammar scan.
    Empty,
    /// The original scalar grammar validator.
    ScalarReference,
    /// The strict Standard/URL-safe scalar table validator.
    ScalarTable,
    /// SIMD interior validation with scalar grammar validation of the tail.
    Vector(Backend),
}

/// Why an attempted vector writer was replaced by the scalar writer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DecodeFallback {
    /// No writer replacement occurred. Scalar selection alone is not recovery.
    None,
    /// The selected backend was unavailable or unhealthy before writing.
    BackendUnavailable,
    /// The kernel rejected validated input or failed checked output comparison.
    /// Its partial output was completely replaced by the scalar writer.
    BackendRejected,
}

/// Actual work performed by one successful canonical ordinary decode call.
///
/// This is returned with the result, not reconstructed from global dispatch
/// state. It is not a promise about the next call, a health snapshot, or secret
/// processing evidence. Errors return no success report and leave output intact.
/// Scalar tails may accompany a vector writer. Recovery reports the final
/// scalar writer, not the discarded vector output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeReport {
    pub(crate) requested: DecodeValidation,
    pub(crate) validator: DecodeValidator,
    pub(crate) selected_backend: Option<Backend>,
    pub(crate) output_backend: Backend,
    pub(crate) checked_validation: bool,
    pub(crate) checked_output: bool,
    pub(crate) fallback: DecodeFallback,
}

impl DecodeReport {
    pub(crate) const fn new(requested: DecodeValidation) -> Self {
        Self {
            requested,
            validator: DecodeValidator::Empty,
            selected_backend: None,
            output_backend: Backend::Scalar,
            checked_validation: false,
            checked_output: false,
            fallback: DecodeFallback::None,
        }
    }

    /// The caller's requested validation policy.
    #[must_use]
    pub const fn requested_validation(&self) -> DecodeValidation {
        self.requested
    }

    /// The validator that actually accepted the complete input.
    #[must_use]
    pub const fn validator(&self) -> DecodeValidator {
        self.validator
    }

    /// The eligible backend selected before validation, if any.
    /// This need not be the final writer after a health change or rejection.
    #[must_use]
    pub const fn selected_backend(&self) -> Option<Backend> {
        self.selected_backend
    }

    /// Backend that produced the retained interior output; tails remain scalar.
    #[must_use]
    pub const fn output_backend(&self) -> Backend {
        self.output_backend
    }

    /// Whether a vector validation result received independent reference validation.
    #[must_use]
    pub const fn checked_validation(&self) -> bool {
        self.checked_validation
    }

    /// Whether redundant scalar comparison was attempted for vector output.
    /// This remains true if a mismatch caused complete scalar recovery.
    #[must_use]
    pub const fn checked_output(&self) -> bool {
        self.checked_output
    }

    /// Replacement of an attempted vector writer, distinct from initial scalar selection.
    #[must_use]
    pub const fn fallback(&self) -> DecodeFallback {
        self.fallback
    }
}
