//! Redacted assurance, provider, and teardown state.

/// Strength of the completed logical-allocation wipe.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WipeEvidence {
    /// No policy-usable complete overwrite is known.
    WipeNotCompleted,
    /// The full logical range and best-effort barrier completed.
    WipedBestEffort,
    /// Best-effort wiping completed with current matching platform evidence.
    WipedAttested,
}

/// Physical protection independently reported by the provider.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PhysicalProtection {
    /// Current evidence confirms the requested protection.
    ProtectionAttested,
    /// Current evidence conclusively says protection is absent.
    ProtectionConfirmedAbsent,
    /// The provider cannot currently establish physical posture.
    ProtectionUnknown,
}

/// Conservative provider accounting posture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AccountingPosture {
    /// The identity, bytes, and effective pages remain charged.
    Charged,
    /// All accounting transitions completed conclusively.
    Reconciled,
}

/// Ordered teardown stage that remains incomplete.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PendingStage {
    /// Complete logical-range wipe and barrier.
    Wipe,
    /// Protection removal or page unlock.
    ProtectionRemoval,
    /// Provider accounting reconciliation.
    AccountingReconciliation,
    /// Conclusive disposal or deallocation.
    Disposal,
}

/// Redacted allocation lifecycle posture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LifecyclePosture {
    /// One public protected owner is live.
    Live,
    /// A consuming teardown is executing.
    Closing {
        /// Stage currently executing.
        stage: PendingStage,
    },
    /// Provider quarantine owns an incomplete teardown.
    Quarantined {
        /// Earliest stage that must resume.
        pending_stage: PendingStage,
    },
    /// Retry limits were exhausted and in-process recovery is forbidden.
    PermanentlyQuarantined {
        /// Earliest permanently incomplete stage.
        pending_stage: PendingStage,
    },
    /// Allocation existence became indeterminate and no pointer remains.
    Tombstoned {
        /// Last operation attempted before addressability was destroyed.
        last_stage: PendingStage,
        /// Conservative allocation-presence disposition.
        disposition: AllocationPresence,
    },
    /// Teardown completed and no allocation capability remains.
    Closed,
}

/// Terminal allocation-presence result used by tombstones.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AllocationPresence {
    /// The provider cannot establish whether disposal occurred.
    Unknown,
}

/// Provider-wide admission health.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProviderHealth {
    /// New protected admission is permitted.
    Healthy,
    /// A failure blocks new high-assurance admission.
    Degraded,
    /// A finite provider budget is exhausted.
    Exhausted,
    /// The provider instance cannot admit or recover more work.
    Shutdown,
}

/// Finite provider resource category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ResourceKind {
    /// Combined active, reserved, quarantined, and tombstoned identities.
    Identities,
    /// Complete logical allocation bytes.
    LogicalBytes,
    /// Effective page-rounded storage.
    EffectivePages,
    /// Pre-reserved quarantine registry slots.
    RegistryEntries,
    /// Lifetime retry attempts.
    RetryAttempts,
    /// Work permitted in one maintenance call.
    MaintenanceWork,
}

/// Finite provider configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProviderLimits {
    /// Maximum combined identities.
    pub max_identities: usize,
    /// Maximum combined logical bytes.
    pub max_logical_bytes: usize,
    /// Maximum combined effective pages.
    pub max_effective_pages: usize,
    /// Maximum pre-reserved registry entries.
    pub max_registry_entries: usize,
    /// Maximum retry attempts per quarantined identity.
    pub max_retry_attempts: usize,
    /// Maximum entries examined by one maintenance call.
    pub max_maintenance_work: usize,
    /// Provider page size used for checked reservations.
    pub page_size: usize,
}

/// Allocation admission request made before plaintext can materialize.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProtectionRequest {
    logical_bytes: usize,
    reserved_pages: usize,
    attested: bool,
}

impl ProtectionRequest {
    pub(crate) fn new(
        logical_bytes: usize,
        page_size: usize,
        attested: bool,
    ) -> Result<Self, ProtectionError> {
        if page_size == 0 {
            return Err(ProtectionError::InvalidLimits);
        }
        let reserved_pages = if logical_bytes == 0 {
            0
        } else {
            logical_bytes
                .checked_add(page_size - 1)
                .and_then(|len| len.checked_add(page_size - 1))
                .map(|worst_case| worst_case / page_size)
                .ok_or(ProtectionError::LengthOverflow)?
        };
        Ok(Self {
            logical_bytes,
            reserved_pages,
            attested,
        })
    }

    /// Requested logical allocation bytes.
    #[must_use]
    pub const fn logical_bytes(self) -> usize {
        self.logical_bytes
    }

    /// Conservatively reserved effective pages.
    #[must_use]
    pub const fn reserved_pages(self) -> usize {
        self.reserved_pages
    }

    /// Whether the operation requires attested protection.
    #[must_use]
    pub const fn requires_attestation(self) -> bool {
        self.attested
    }
}

/// Redacted protected-allocation admission failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProtectionError {
    /// Policy or attestation evidence is stale or mismatched.
    StaleAssurance,
    /// Required physical protection is unavailable.
    ProtectionUnavailable,
    /// The provider is not healthy enough for admission.
    ProviderUnavailable,
    /// A finite provider resource is exhausted.
    ProtectionResourceExhausted(ResourceKind),
    /// Length or page rounding overflowed.
    LengthOverflow,
    /// Provider limits are internally invalid.
    InvalidLimits,
    /// Actual protected pages exceeded the preflight reservation.
    ActualRangeExceededReservation,
}

impl core::fmt::Display for ProtectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::StaleAssurance => "stale assurance evidence",
            Self::ProtectionUnavailable => "required protected storage is unavailable",
            Self::ProviderUnavailable => "protected-memory provider is unavailable",
            Self::ProtectionResourceExhausted(_) => "protected-memory resource exhausted",
            Self::LengthOverflow => "protected-memory length overflow",
            Self::InvalidLimits => "invalid protected-memory limits",
            Self::ActualRangeExceededReservation => {
                "actual protected range exceeded its reservation"
            }
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProtectionError {}

/// One provider sub-operation recorded by the volatile journal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TeardownOperation {
    /// Complete wipe confirmation.
    Wipe,
    /// Physical protection removal.
    ProtectionRemoval,
    /// Accounting reconciliation.
    AccountingReconciliation,
    /// Disposal or deallocation.
    Disposal,
}

/// Monotonic provider-operation disposition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum JournalDisposition {
    /// The operation is conclusively not applied.
    NotApplied,
    /// The operation is conclusively applied.
    Applied,
    /// The provider cannot determine whether it applied.
    Indeterminate,
}

/// Fixed-size volatile teardown journal cursor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TeardownCursor {
    /// Current operation.
    pub operation: TeardownOperation,
    /// Current operation disposition.
    pub disposition: JournalDisposition,
    /// Bounded range/page progress cursor.
    pub progress: usize,
}

impl TeardownCursor {
    pub(crate) const fn new() -> Self {
        Self {
            operation: TeardownOperation::Wipe,
            disposition: JournalDisposition::NotApplied,
            progress: 0,
        }
    }

    pub(crate) fn begin(&mut self, operation: TeardownOperation) {
        self.operation = operation;
        self.disposition = JournalDisposition::NotApplied;
        self.progress = 0;
    }
}

/// Conclusive or ambiguous disposal result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DisposalDisposition {
    /// Disposal completed exactly once.
    Applied,
    /// Disposal conclusively did not occur.
    NotApplied,
    /// Allocation presence is no longer knowable.
    AllocationPresenceUnknown,
}

/// Redacted successful teardown outcome.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CleanupOutcome {
    /// Every required stage completed.
    Closed,
}

/// Redacted successful close report.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CleanupReport {
    /// Final cleanup outcome.
    pub outcome: CleanupOutcome,
    /// Wipe evidence established before close.
    pub wipe: WipeEvidence,
    /// Final physical protection posture.
    pub physical_protection: PhysicalProtection,
    /// Final accounting posture.
    pub accounting: AccountingPosture,
    /// Final lifecycle posture.
    pub lifecycle: LifecyclePosture,
}

/// Redacted cleanup failure after ownership transferred to the provider.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CleanupError {
    /// Earliest stage that remains incomplete.
    pub pending_stage: PendingStage,
    /// Strongest wipe evidence actually established.
    pub wipe: WipeEvidence,
    /// Last honest physical protection posture.
    pub physical_protection: PhysicalProtection,
    /// Conservative accounting posture.
    pub accounting: AccountingPosture,
    /// Provider-owned terminal lifecycle.
    pub lifecycle: LifecyclePosture,
    /// Retry attempt charged by the transfer.
    pub retry_attempt: usize,
    /// Provider health after the failure.
    pub provider_health: ProviderHealth,
}

impl core::fmt::Display for CleanupError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "protected cleanup stopped at {:?}",
            self.pending_stage
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CleanupError {}

/// Aggregate redacted provider report.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProviderReport {
    /// Current provider health.
    pub health: ProviderHealth,
    /// Current health generation.
    pub health_generation: usize,
    /// Current protection generation.
    pub protection_generation: usize,
    /// Combined active and reserved identities.
    pub active_and_reserved: usize,
    /// Quarantined identities.
    pub quarantined: usize,
    /// Terminal tombstone identities.
    pub tombstoned: usize,
    /// Conservatively charged logical bytes.
    pub charged_logical_bytes: usize,
    /// Conservatively charged effective pages.
    pub charged_effective_pages: usize,
}
