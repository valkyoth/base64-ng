//! Runtime assurance tokens and allocation-specific protected ownership.
//!
//! An [`AssuranceToken`] proves only the context policy generations captured
//! when it was issued. It never proves that an arbitrary allocation is locked
//! or otherwise protected. Assured operations therefore also require a
//! [`ProtectedSecret`] issued by one reviewed [`ProtectedMemoryProvider`].

mod context;
#[cfg(feature = "alloc")]
mod default_provider;
mod operations;
mod posture;
mod protected;
mod provider;

pub use context::{
    AssuranceContext, AssuranceError, AssuranceGenerations, AssuranceLevel, AssuranceToken,
    AttestationEvidence, Attested, BestEffort, PlatformAttestation, TargetAttestation,
    WipeAttestation,
};
#[cfg(feature = "alloc")]
pub use default_provider::{BestEffortHandle, BestEffortProvider, BestEffortReservation};
pub use operations::{AssuredDecodeError, AssuredEncodeError};
pub use posture::{
    AccountingPosture, AllocationPresence, CleanupError, CleanupOutcome, CleanupReport,
    DisposalDisposition, JournalDisposition, LifecyclePosture, PendingStage, PhysicalProtection,
    ProtectionError, ProtectionRequest, ProviderHealth, ProviderLimits, ProviderReport,
    ResourceKind, TeardownCursor, TeardownOperation, WipeEvidence,
};
pub use protected::{
    ExposedProtectedSecret, ProtectedSecret, Uninitialized, Unvalidated, Validated,
};
pub use provider::{
    DisposalResult, ProtectedMemoryProvider, ProviderAccess, ProviderOperationResult,
    QuarantineRecord, ThreadMovableProvider, WipeConfirmation,
};

#[cfg(test)]
mod tests;
