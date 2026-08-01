//! Stable redacted assurance and allocation-specific operation reports.

use super::{
    AccountingPosture, AssuranceGenerations, AssuranceLevel, AssuranceToken, CleanupError,
    CleanupReport, JournalDisposition, LifecyclePosture, PendingStage, PhysicalProtection,
    ProviderHealth, WipeEvidence,
};
use crate::runtime::{CtGatePosture, OperationBackendReport, WipePosture};

/// Secret operation recorded for one protected allocation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SecretOperation {
    /// No encode or decode has started.
    NotStarted,
    /// Constant-time-oriented scalar encode.
    Encode,
    /// Constant-time-oriented scalar decode.
    Decode,
}

impl SecretOperation {
    /// Returns the stable operation identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::Encode => "secret-encode",
            Self::Decode => "secret-decode",
        }
    }
}

/// Attestation attached to the exact token and operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AttestationPosture {
    /// No platform attestation is attached.
    NotAttested,
    /// Matching platform and provider attestation is attached.
    Attested,
}

impl AttestationPosture {
    /// Returns the stable posture identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotAttested => "not-attested",
            Self::Attested => "attested",
        }
    }
}

/// Secret-operation policy represented by one token.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SecretPolicyPosture {
    /// Best-effort secret policy without hardware attestation.
    BestEffort,
    /// High-assurance policy with matching runtime attestation.
    HighAssuranceAttested,
}

impl SecretPolicyPosture {
    /// Returns the stable policy identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BestEffort => "best-effort",
            Self::HighAssuranceAttested => "high-assurance-attested",
        }
    }
}

/// Address-free allocation-presence posture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AllocationPosture {
    /// The exact protected owner is live and addressable only through its type.
    Present,
    /// Teardown conclusively removed the allocation capability.
    Absent,
    /// Disposal became indeterminate and no address is retained.
    UnknownNoAddressRetained,
}

impl AllocationPosture {
    /// Returns the stable posture identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::UnknownNoAddressRetained => "unknown-no-address-retained",
        }
    }
}

/// Token-scoped assurance report without allocation claims.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssuranceReport {
    /// Ordinary encode backend selection.
    pub encode_backend: OperationBackendReport,
    /// Ordinary strict-decode backend selection.
    pub strict_decode_backend: OperationBackendReport,
    /// Secret decode backend selection.
    pub secret_decode_backend: OperationBackendReport,
    /// Independent context generations captured by the token.
    pub generations: AssuranceGenerations,
    /// Wipe primitive/barrier posture.
    pub wipe_posture: WipePosture,
    /// Secret result-gate posture.
    pub result_gate_posture: CtGatePosture,
    /// Token attestation posture.
    pub attestation_posture: AttestationPosture,
    /// Secret policy posture.
    pub secret_policy_posture: SecretPolicyPosture,
    /// Wasm artifact posture inherited from ordinary runtime reporting.
    pub wasm_artifact_posture: crate::runtime::WasmArtifactPosture,
    /// Wasm host-runtime identification posture.
    pub wasm_runtime_posture: crate::runtime::WasmRuntimePosture,
}

impl AssuranceReport {
    /// Returns a stable logging snapshot.
    #[must_use]
    pub const fn snapshot(self) -> AssuranceSnapshot {
        AssuranceSnapshot {
            encode_backend: self.encode_backend.snapshot(),
            strict_decode_backend: self.strict_decode_backend.snapshot(),
            secret_decode_backend: self.secret_decode_backend.snapshot(),
            ordinary_backend_generation: self.generations.ordinary_backend,
            secret_algorithm_generation: self.generations.secret_algorithm,
            wipe_barrier_generation: self.generations.wipe_barrier,
            speculation_generation: self.generations.speculation,
            wipe_posture: self.wipe_posture.as_str(),
            result_gate_posture: self.result_gate_posture.as_str(),
            attestation_posture: self.attestation_posture.as_str(),
            secret_policy_posture: self.secret_policy_posture.as_str(),
            wasm_artifact_posture: self.wasm_artifact_posture.as_str(),
            wasm_runtime_posture: self.wasm_runtime_posture.as_str(),
        }
    }
}

/// Stable token-scoped logging snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssuranceSnapshot {
    /// Stable encode backend snapshot.
    pub encode_backend: crate::runtime::OperationBackendSnapshot,
    /// Stable strict-decode backend snapshot.
    pub strict_decode_backend: crate::runtime::OperationBackendSnapshot,
    /// Stable secret-decode backend snapshot.
    pub secret_decode_backend: crate::runtime::OperationBackendSnapshot,
    /// Ordinary backend generation.
    pub ordinary_backend_generation: usize,
    /// Secret algorithm generation.
    pub secret_algorithm_generation: usize,
    /// Wipe/barrier generation.
    pub wipe_barrier_generation: usize,
    /// Result-gate/speculation generation.
    pub speculation_generation: usize,
    /// Stable wipe posture.
    pub wipe_posture: &'static str,
    /// Stable result-gate posture.
    pub result_gate_posture: &'static str,
    /// Stable attestation posture.
    pub attestation_posture: &'static str,
    /// Stable secret-policy posture.
    pub secret_policy_posture: &'static str,
    /// Stable Wasm artifact posture.
    pub wasm_artifact_posture: &'static str,
    /// Stable Wasm host-runtime posture.
    pub wasm_runtime_posture: &'static str,
}

/// Report for the exact allocation participating in one assured operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedOperationReport {
    /// Token and backend posture.
    pub assurance: AssuranceReport,
    /// Secret operation associated with this allocation.
    pub operation: SecretOperation,
    /// Wipe evidence currently established for this live allocation.
    pub wipe: WipeEvidence,
    /// Physical posture reported for this exact allocation.
    pub physical_protection: PhysicalProtection,
    /// Conservative resource-accounting posture.
    pub accounting: AccountingPosture,
    /// Allocation lifecycle, independently reported.
    pub lifecycle: LifecyclePosture,
    /// Address-free allocation-presence posture.
    pub allocation: AllocationPosture,
    /// Provider health at observation time.
    pub provider_health: ProviderHealth,
    /// Provider health generation.
    pub health_generation: usize,
    /// Provider protection generation.
    pub protection_generation: usize,
}

impl ProtectedOperationReport {
    pub(crate) const fn live(
        assurance: AssuranceReport,
        operation: SecretOperation,
        physical_protection: PhysicalProtection,
        provider_health: ProviderHealth,
        health_generation: usize,
        protection_generation: usize,
    ) -> Self {
        Self {
            assurance,
            operation,
            wipe: WipeEvidence::WipeNotCompleted,
            physical_protection,
            accounting: AccountingPosture::Charged,
            lifecycle: LifecyclePosture::Live,
            allocation: AllocationPosture::Present,
            provider_health,
            health_generation,
            protection_generation,
        }
    }

    /// Returns a stable, redacted logging snapshot.
    #[must_use]
    pub const fn snapshot(self) -> ProtectedOperationSnapshot {
        ProtectedOperationSnapshot {
            assurance: self.assurance.snapshot(),
            operation: self.operation.as_str(),
            wipe: wipe_id(self.wipe),
            physical_protection: physical_id(self.physical_protection),
            accounting: accounting_id(self.accounting),
            lifecycle: lifecycle_id(self.lifecycle),
            allocation: self.allocation.as_str(),
            provider_health: provider_health_id(self.provider_health),
            health_generation: self.health_generation,
            protection_generation: self.protection_generation,
        }
    }
}

/// Stable redacted snapshot of one participating protected allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedOperationSnapshot {
    /// Token and backend snapshot.
    pub assurance: AssuranceSnapshot,
    /// Stable secret operation identifier.
    pub operation: &'static str,
    /// Stable wipe evidence identifier.
    pub wipe: &'static str,
    /// Stable physical protection identifier.
    pub physical_protection: &'static str,
    /// Stable accounting identifier.
    pub accounting: &'static str,
    /// Stable lifecycle identifier.
    pub lifecycle: &'static str,
    /// Stable allocation-presence identifier.
    pub allocation: &'static str,
    /// Stable provider-health identifier.
    pub provider_health: &'static str,
    /// Provider health generation.
    pub health_generation: usize,
    /// Provider protection generation.
    pub protection_generation: usize,
}

/// Stable redacted teardown snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanupSnapshot {
    /// Stable wipe evidence identifier.
    pub wipe: &'static str,
    /// Stable physical protection identifier.
    pub physical_protection: &'static str,
    /// Stable accounting identifier.
    pub accounting: &'static str,
    /// Stable lifecycle identifier.
    pub lifecycle: &'static str,
    /// Address-free allocation-presence identifier.
    pub allocation: &'static str,
    /// Earliest pending teardown stage, if any.
    pub pending_stage: Option<&'static str>,
    /// Redacted provider substage, if any.
    pub pending_substage: Option<&'static str>,
    /// Provider health after failure, if applicable.
    pub provider_health: Option<&'static str>,
    /// Retry attempt, if applicable.
    pub retry_attempt: Option<usize>,
}

/// Stable redacted provider-wide resource snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderSnapshot {
    /// Stable provider-health identifier.
    pub health: &'static str,
    /// Provider health generation.
    pub health_generation: usize,
    /// Provider protection generation.
    pub protection_generation: usize,
    /// Active and reserved identities.
    pub active_and_reserved: usize,
    /// Retryable quarantined identities.
    pub quarantined: usize,
    /// Permanently quarantined identities.
    pub permanently_quarantined: usize,
    /// Address-free tombstone identities.
    pub tombstoned: usize,
    /// Conservatively charged logical bytes.
    pub charged_logical_bytes: usize,
    /// Conservatively charged effective pages.
    pub charged_effective_pages: usize,
}

impl super::ProviderReport {
    /// Returns a stable provider snapshot without allocation addresses.
    #[must_use]
    pub const fn snapshot(self) -> ProviderSnapshot {
        ProviderSnapshot {
            health: provider_health_id(self.health),
            health_generation: self.health_generation,
            protection_generation: self.protection_generation,
            active_and_reserved: self.active_and_reserved,
            quarantined: self.quarantined,
            permanently_quarantined: self.permanently_quarantined,
            tombstoned: self.tombstoned,
            charged_logical_bytes: self.charged_logical_bytes,
            charged_effective_pages: self.charged_effective_pages,
        }
    }
}

impl<Level: AssuranceLevel> AssuranceToken<'_, Level> {
    /// Reports token-scoped posture without claiming allocation protection.
    #[must_use]
    pub fn report(&self) -> AssuranceReport {
        let runtime = crate::runtime::backend_report();
        let mut encode_backend = runtime.encode_backend;
        let mut strict_decode_backend = runtime.strict_decode_backend;
        let mut secret_decode_backend = runtime.secret_decode_backend;
        let generations = self.generations();
        encode_backend.health_generation = generations.ordinary_backend;
        strict_decode_backend.health_generation = generations.ordinary_backend;
        secret_decode_backend.health_generation = generations.secret_algorithm;
        let evidence = self.evidence();
        AssuranceReport {
            encode_backend,
            strict_decode_backend,
            secret_decode_backend,
            generations,
            wipe_posture: evidence.map_or(
                runtime.wipe_posture,
                super::context::AttestationEvidence::wipe_posture,
            ),
            result_gate_posture: evidence.map_or(
                runtime.ct_gate_posture,
                super::context::AttestationEvidence::speculation_posture,
            ),
            attestation_posture: if evidence.is_some() {
                AttestationPosture::Attested
            } else {
                AttestationPosture::NotAttested
            },
            secret_policy_posture: if Self::requires_attestation() {
                SecretPolicyPosture::HighAssuranceAttested
            } else {
                SecretPolicyPosture::BestEffort
            },
            wasm_artifact_posture: runtime.wasm_artifact_posture,
            wasm_runtime_posture: runtime.wasm_runtime_posture,
        }
    }
}

impl CleanupReport {
    /// Returns a stable snapshot without retaining allocation identity.
    #[must_use]
    pub const fn snapshot(self) -> CleanupSnapshot {
        CleanupSnapshot {
            wipe: wipe_id(self.wipe),
            physical_protection: physical_id(self.physical_protection),
            accounting: accounting_id(self.accounting),
            lifecycle: lifecycle_id(self.lifecycle),
            allocation: AllocationPosture::Absent.as_str(),
            pending_stage: None,
            pending_substage: None,
            provider_health: None,
            retry_attempt: None,
        }
    }
}

impl CleanupError {
    /// Returns a stable redacted snapshot without an allocation address.
    #[must_use]
    pub const fn snapshot(self) -> CleanupSnapshot {
        let allocation = if matches!(self.lifecycle, LifecyclePosture::Tombstoned { .. }) {
            AllocationPosture::UnknownNoAddressRetained
        } else {
            AllocationPosture::Present
        };
        CleanupSnapshot {
            wipe: wipe_id(self.wipe),
            physical_protection: physical_id(self.physical_protection),
            accounting: accounting_id(self.accounting),
            lifecycle: lifecycle_id(self.lifecycle),
            allocation: allocation.as_str(),
            pending_stage: Some(pending_stage_id(self.pending_stage)),
            pending_substage: Some(journal_id(self.pending_substage)),
            provider_health: Some(provider_health_id(self.provider_health)),
            retry_attempt: Some(self.retry_attempt),
        }
    }
}

const fn wipe_id(value: WipeEvidence) -> &'static str {
    match value {
        WipeEvidence::WipeNotCompleted => "wipe-not-completed",
        WipeEvidence::WipedBestEffort => "wiped-best-effort",
        WipeEvidence::WipedAttested => "wiped-attested",
    }
}

const fn physical_id(value: PhysicalProtection) -> &'static str {
    match value {
        PhysicalProtection::ProtectionAttested => "protection-attested",
        PhysicalProtection::ProtectionConfirmedAbsent => "protection-confirmed-absent",
        PhysicalProtection::ProtectionUnknown => "protection-unknown",
    }
}

const fn accounting_id(value: AccountingPosture) -> &'static str {
    match value {
        AccountingPosture::Charged => "charged",
        AccountingPosture::Reconciled => "reconciled",
    }
}

const fn lifecycle_id(value: LifecyclePosture) -> &'static str {
    match value {
        LifecyclePosture::Live => "live",
        LifecyclePosture::Closing { .. } => "closing",
        LifecyclePosture::Quarantined { .. } => "quarantined",
        LifecyclePosture::PermanentlyQuarantined { .. } => "permanently-quarantined",
        LifecyclePosture::Tombstoned { .. } => "tombstoned",
        LifecyclePosture::Closed => "closed",
    }
}

const fn pending_stage_id(value: PendingStage) -> &'static str {
    match value {
        PendingStage::Wipe => "wipe",
        PendingStage::ProtectionRemoval => "protection-removal",
        PendingStage::AccountingReconciliation => "accounting-reconciliation",
        PendingStage::Disposal => "disposal",
    }
}

const fn journal_id(value: JournalDisposition) -> &'static str {
    match value {
        JournalDisposition::NotApplied => "not-applied",
        JournalDisposition::Applied => "applied",
        JournalDisposition::Indeterminate => "indeterminate",
    }
}

const fn provider_health_id(value: ProviderHealth) -> &'static str {
    match value {
        ProviderHealth::Healthy => "healthy",
        ProviderHealth::Degraded => "degraded",
        ProviderHealth::Exhausted => "exhausted",
        ProviderHealth::Shutdown => "shutdown",
    }
}
