//! Unsafe protected-memory provider protocol.

use super::{
    AccountingPosture, AttestationEvidence, JournalDisposition, PhysicalProtection,
    ProtectionError, ProtectionRequest, ProviderHealth, ProviderLimits, ProviderReport,
    TeardownCursor, WipeEvidence,
};

/// Conclusive or indeterminate provider sub-operation result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProviderOperationResult {
    /// The operation conclusively applied.
    Applied,
    /// The operation conclusively did not apply.
    NotApplied,
    /// The provider cannot determine whether the operation applied.
    Indeterminate,
}

impl ProviderOperationResult {
    pub(crate) const fn journal_disposition(self) -> JournalDisposition {
        match self {
            Self::Applied => JournalDisposition::Applied,
            Self::NotApplied => JournalDisposition::NotApplied,
            Self::Indeterminate => JournalDisposition::Indeterminate,
        }
    }
}

/// Wipe confirmation returned after the crate overwrites the full range.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WipeConfirmation {
    /// Provider result for the modeled wipe/barrier operation.
    pub result: ProviderOperationResult,
    /// Strongest evidence independently established by the provider.
    pub evidence: WipeEvidence,
}

/// Disposal result that never makes an ambiguous handle retryable.
pub enum DisposalResult<Handle> {
    /// Disposal completed and no allocation capability remains.
    Applied,
    /// Disposal conclusively did not apply; the handle remains live.
    NotApplied(Handle),
    /// Allocation presence is unknown and no addressable handle remains.
    AllocationPresenceUnknown,
}

/// Provider-owned quarantine transfer record.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QuarantineRecord {
    /// Earliest teardown stage still required.
    pub pending_stage: super::PendingStage,
    /// Wipe evidence actually retained.
    pub wipe: WipeEvidence,
    /// Last honest physical-protection result.
    pub physical_protection: PhysicalProtection,
    /// Conservative accounting result.
    pub accounting: AccountingPosture,
    /// Monotonic volatile journal cursor.
    pub cursor: TeardownCursor,
    /// Retry attempt charged by this transfer.
    pub retry_attempt: usize,
}

/// Crate-issued access capability for ownership-sensitive provider hooks.
///
/// The type is public only because it appears in the unsafe provider protocol.
/// Its value cannot be constructed by external safe code, so a provider's raw
/// allocation handles cannot bypass [`super::ProtectedSecret`].
pub struct ProviderAccess {
    _private: (),
}

impl ProviderAccess {
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }
}

/// Protected allocation and bounded quarantine provider.
///
/// # Safety
///
/// An implementation must enforce unique live handles, stable backing ranges,
/// checked finite admission budgets, generation/ABA safety, and infallible
/// allocation-free quarantine transfer. Returned byte slices must name exactly
/// one live handle's complete logical allocation. Every hook must be
/// non-unwinding. Provider operations must obey their reported
/// applied/not-applied/indeterminate disposition; non-idempotent ambiguous work
/// must not be replayed. `AllocationPresenceUnknown` must destroy every pointer
/// capability and retain only a non-owning tombstone identity.
#[allow(unsafe_code)]
pub unsafe trait ProtectedMemoryProvider {
    /// Live allocation handle owned by one protected typestate value.
    type Handle;
    /// Pre-plaintext reservation carrying one registry slot and full budget.
    type Reservation;

    /// Stable opaque provider-instance identity.
    fn provider_identity(&self) -> usize;

    /// Provider-instance generation. Reconstruction must change it.
    fn provider_generation(&self) -> usize;

    /// Generation changed by provider health degradation.
    fn health_generation(&self) -> usize;

    /// Generation changed by physical protection transitions.
    fn protection_generation(&self) -> usize;

    /// Current provider health.
    fn health(&self) -> ProviderHealth;

    /// Finite provider limits.
    fn limits(&self) -> ProviderLimits;

    /// Redacted aggregate report.
    fn report(&self) -> ProviderReport;

    /// Reserves registry and complete resource budgets before allocation.
    fn reserve(
        &self,
        access: &ProviderAccess,
        request: ProtectionRequest,
    ) -> Result<Self::Reservation, ProtectionError>;

    /// Creates protected storage without plaintext and consumes the reservation.
    fn materialize(
        &self,
        access: &ProviderAccess,
        reservation: Self::Reservation,
    ) -> Result<Self::Handle, ProtectionError>;

    /// Complete logical allocation length.
    fn logical_len(&self, access: &ProviderAccess, handle: &Self::Handle) -> usize;

    /// Current physical protection posture for this exact handle.
    fn physical_protection(
        &self,
        access: &ProviderAccess,
        handle: &Self::Handle,
    ) -> PhysicalProtection;

    /// Immutable access tied to the unique live handle.
    fn bytes<'handle>(
        &self,
        access: &ProviderAccess,
        handle: &'handle Self::Handle,
    ) -> &'handle [u8];

    /// Exclusive access tied to the unique live handle.
    fn bytes_mut<'handle>(
        &self,
        access: &ProviderAccess,
        handle: &'handle mut Self::Handle,
    ) -> &'handle mut [u8];

    /// Confirms the completed overwrite/barrier at the requested assurance level.
    fn confirm_wipe(
        &self,
        access: &ProviderAccess,
        handle: &Self::Handle,
        attestation: Option<AttestationEvidence>,
        cursor: &mut TeardownCursor,
    ) -> WipeConfirmation;

    /// Removes physical protection conclusively or reports uncertainty.
    fn remove_protection(
        &self,
        access: &ProviderAccess,
        handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult;

    /// Reconciles page/reference/accounting state exactly once.
    fn reconcile_accounting(
        &self,
        access: &ProviderAccess,
        handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult;

    /// Disposes the allocation exactly once.
    fn dispose(
        &self,
        access: &ProviderAccess,
        handle: Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> DisposalResult<Self::Handle>;

    /// Transfers one still-live handle into its pre-reserved quarantine slot.
    ///
    /// This hook must be infallible, allocation-free, and non-unwinding.
    fn quarantine(&self, access: &ProviderAccess, handle: Self::Handle, record: QuarantineRecord);
}

mod sealed {
    pub trait ThreadMovable {}
}

/// Sealed proof that provider protection and teardown may move across threads.
///
/// This is implemented only for providers reviewed inside `base64-ng`.
pub trait ThreadMovableProvider: ProtectedMemoryProvider + sealed::ThreadMovable {}
