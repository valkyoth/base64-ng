#![cfg(all(feature = "secrets", feature = "alloc"))]

use std::{
    cell::{Cell, RefCell},
    vec::Vec,
};

use base64_ng::{
    STRICT_STANDARD_PADDED,
    assurance::{
        AccountingPosture, AllocationPosture, AllocationPresence, AssuranceContext,
        AttestationEvidence, AttestationPosture, BestEffort, CleanupError, DisposalResult,
        JournalDisposition, LifecyclePosture, PendingStage, PhysicalProtection,
        PlatformAttestation, ProtectedMemoryProvider, ProtectedSecret, ProtectionError,
        ProtectionRequest, ProviderAccess, ProviderHealth, ProviderLimits, ProviderOperationResult,
        ProviderReport, QuarantineRecord, ResourceKind, SecretOperation, SecretPolicyPosture,
        TargetAttestation, TeardownCursor, WipeAttestation, WipeConfirmation, WipeEvidence,
    },
    runtime::{CtGatePosture, WipePosture},
    secret::SecretInput,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    Wipe,
    Protection,
    ProtectionIndeterminate,
    Accounting,
    Disposal,
    DisposalIndeterminate,
}

struct Reservation {
    len: usize,
}

struct Handle {
    bytes: Vec<u8>,
}

#[derive(Default)]
struct State {
    active: bool,
    quarantined: Option<Vec<u8>>,
    tombstoned: bool,
    health_generation: usize,
    protection_generation: usize,
    trace: Vec<&'static str>,
    last_record: Option<QuarantineRecord>,
}

struct FaultProvider {
    fault: Cell<Fault>,
    attested: bool,
    state: RefCell<State>,
}

impl FaultProvider {
    fn new(fault: Fault) -> Self {
        Self {
            fault: Cell::new(fault),
            attested: false,
            state: RefCell::new(State {
                health_generation: 1,
                protection_generation: 1,
                ..State::default()
            }),
        }
    }

    fn attested() -> Self {
        let mut provider = Self::new(Fault::None);
        provider.attested = true;
        provider
    }

    fn trace(&self) -> Vec<&'static str> {
        self.state.borrow().trace.clone()
    }

    fn record(&self) -> Option<QuarantineRecord> {
        self.state.borrow().last_record
    }

    #[cfg(base64_ng_require_high_assurance)]
    fn invalidate_health_generation(&self) {
        self.state.borrow_mut().health_generation += 1;
    }
}

unsafe impl ProtectedMemoryProvider for FaultProvider {
    type Handle = Handle;
    type Reservation = Reservation;

    fn provider_identity(&self) -> usize {
        77
    }
    fn provider_generation(&self) -> usize {
        1
    }
    fn health_generation(&self) -> usize {
        self.state.borrow().health_generation
    }
    fn protection_generation(&self) -> usize {
        self.state.borrow().protection_generation
    }
    fn health(&self) -> ProviderHealth {
        let state = self.state.borrow();
        if state.tombstoned {
            ProviderHealth::Shutdown
        } else if state.quarantined.is_some() {
            ProviderHealth::Degraded
        } else {
            ProviderHealth::Healthy
        }
    }
    fn limits(&self) -> ProviderLimits {
        ProviderLimits {
            max_identities: 1,
            max_logical_bytes: 64,
            max_effective_pages: 2,
            max_registry_entries: 1,
            max_retry_attempts: 1,
            max_maintenance_work: 1,
            page_size: 64,
        }
    }
    fn report(&self) -> ProviderReport {
        let state = self.state.borrow();
        let charged = usize::from(state.active || state.quarantined.is_some() || state.tombstoned);
        ProviderReport {
            health: self.health(),
            health_generation: state.health_generation,
            protection_generation: state.protection_generation,
            active_and_reserved: usize::from(state.active),
            quarantined: usize::from(state.quarantined.is_some()),
            permanently_quarantined: 0,
            tombstoned: usize::from(state.tombstoned),
            charged_logical_bytes: charged * 16,
            charged_effective_pages: charged,
        }
    }
    fn reserve(
        &self,
        _access: &ProviderAccess,
        request: ProtectionRequest,
    ) -> Result<Self::Reservation, ProtectionError> {
        if self.state.borrow().active {
            return Err(ProtectionError::ProtectionResourceExhausted(
                ResourceKind::Identities,
            ));
        }
        Ok(Reservation {
            len: request.logical_bytes(),
        })
    }
    fn materialize(
        &self,
        _access: &ProviderAccess,
        reservation: Self::Reservation,
    ) -> Result<Self::Handle, ProtectionError> {
        self.state.borrow_mut().active = true;
        Ok(Handle {
            bytes: vec![0; reservation.len],
        })
    }
    fn logical_len(&self, _access: &ProviderAccess, handle: &Self::Handle) -> usize {
        handle.bytes.len()
    }
    fn physical_protection(
        &self,
        _access: &ProviderAccess,
        _handle: &Self::Handle,
    ) -> PhysicalProtection {
        if self.attested {
            PhysicalProtection::ProtectionAttested
        } else {
            PhysicalProtection::ProtectionConfirmedAbsent
        }
    }
    fn bytes<'a>(&self, _access: &ProviderAccess, handle: &'a Self::Handle) -> &'a [u8] {
        &handle.bytes
    }
    fn bytes_mut<'a>(
        &self,
        _access: &ProviderAccess,
        handle: &'a mut Self::Handle,
    ) -> &'a mut [u8] {
        &mut handle.bytes
    }
    fn confirm_wipe(
        &self,
        _access: &ProviderAccess,
        _handle: &Self::Handle,
        _attestation: Option<AttestationEvidence>,
        cursor: &mut TeardownCursor,
    ) -> WipeConfirmation {
        self.state.borrow_mut().trace.push("wipe");
        if self.fault.get() == Fault::Wipe {
            WipeConfirmation {
                result: ProviderOperationResult::NotApplied,
                evidence: WipeEvidence::WipeNotCompleted,
            }
        } else {
            cursor.disposition = JournalDisposition::Applied;
            WipeConfirmation {
                result: ProviderOperationResult::Applied,
                evidence: if self.attested {
                    WipeEvidence::WipedAttested
                } else {
                    WipeEvidence::WipedBestEffort
                },
            }
        }
    }
    fn remove_protection(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        _cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        self.state.borrow_mut().trace.push("protection");
        match self.fault.get() {
            Fault::Protection => ProviderOperationResult::NotApplied,
            Fault::ProtectionIndeterminate => ProviderOperationResult::Indeterminate,
            _ => ProviderOperationResult::Applied,
        }
    }
    fn reconcile_accounting(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        _cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        self.state.borrow_mut().trace.push("accounting");
        if self.fault.get() == Fault::Accounting {
            ProviderOperationResult::NotApplied
        } else {
            ProviderOperationResult::Applied
        }
    }
    fn dispose(
        &self,
        _access: &ProviderAccess,
        handle: Self::Handle,
        _cursor: &mut TeardownCursor,
    ) -> DisposalResult<Self::Handle> {
        self.state.borrow_mut().trace.push("disposal");
        match self.fault.get() {
            Fault::Disposal => DisposalResult::NotApplied(handle),
            Fault::DisposalIndeterminate => {
                let mut state = self.state.borrow_mut();
                state.active = false;
                state.tombstoned = true;
                state.health_generation += 1;
                std::mem::forget(handle);
                DisposalResult::AllocationPresenceUnknown
            }
            _ => {
                self.state.borrow_mut().active = false;
                drop(handle);
                DisposalResult::Applied
            }
        }
    }
    fn quarantine(&self, _access: &ProviderAccess, handle: Self::Handle, record: QuarantineRecord) {
        let mut state = self.state.borrow_mut();
        state.active = false;
        state.quarantined = Some(handle.bytes);
        state.last_record = Some(record);
        state.health_generation += 1;
    }
}

unsafe impl PlatformAttestation for FaultProvider {
    fn attest(&self) -> Result<AttestationEvidence, base64_ng::assurance::AssuranceError> {
        let target = if cfg!(target_arch = "x86_64") {
            TargetAttestation::X86_64
        } else if cfg!(target_arch = "x86") {
            TargetAttestation::X86
        } else if cfg!(all(
            target_arch = "aarch64",
            base64_ng_aarch64_csdb_attested
        )) {
            TargetAttestation::Aarch64Csdb
        } else {
            TargetAttestation::ReviewedEmbedded
        };
        let speculation = if cfg!(target_arch = "aarch64") {
            CtGatePosture::HardwareSpeculationBarrierBuildAsserted
        } else {
            CtGatePosture::HardwareSpeculationBarrier
        };
        Ok(unsafe {
            AttestationEvidence::new(
                target,
                WipeAttestation::VolatileBytesAndSelectedBarrier,
                WipePosture::HardwareFence,
                speculation,
                self.provider_identity(),
                self.provider_generation(),
            )
        })
    }
}

fn close_with_fault(fault: Fault) -> (CleanupError, FaultProvider) {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = FaultProvider::new(fault);
    let allocation: ProtectedSecret<'_, _, _, BestEffort> =
        ProtectedSecret::try_new(&provider, &token, 16).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    let error = validated.try_close().unwrap_err();
    (error, provider)
}

#[test]
fn failed_wipe_calls_no_later_teardown_hook() {
    let (error, provider) = close_with_fault(Fault::Wipe);
    assert_eq!(provider.trace(), ["wipe"]);
    assert_eq!(error.pending_stage, PendingStage::Wipe);
    assert_eq!(error.wipe, WipeEvidence::WipeNotCompleted);
    assert_eq!(error.accounting, AccountingPosture::Charged);
    assert_eq!(provider.report().quarantined, 1);
}

#[test]
fn later_failures_preserve_wipe_and_exact_pending_stage() {
    for (fault, expected, trace) in [
        (
            Fault::Protection,
            PendingStage::ProtectionRemoval,
            &["wipe", "protection"][..],
        ),
        (
            Fault::Accounting,
            PendingStage::AccountingReconciliation,
            &["wipe", "protection", "accounting"][..],
        ),
        (
            Fault::Disposal,
            PendingStage::Disposal,
            &["wipe", "protection", "accounting", "disposal"][..],
        ),
    ] {
        let (error, provider) = close_with_fault(fault);
        assert_eq!(provider.trace(), trace);
        assert_eq!(error.pending_stage, expected);
        assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
        assert_eq!(provider.record().unwrap().pending_stage, expected);
    }
}

#[test]
fn indeterminate_protection_is_unknown_not_attested() {
    let (error, provider) = close_with_fault(Fault::ProtectionIndeterminate);
    assert_eq!(
        error.physical_protection,
        PhysicalProtection::ProtectionUnknown
    );
    assert_eq!(
        provider.record().unwrap().physical_protection,
        PhysicalProtection::ProtectionUnknown
    );
    let snapshot = error.snapshot();
    assert_eq!(snapshot.pending_stage, Some("protection-removal"));
    assert_eq!(snapshot.pending_substage, Some("indeterminate"));
    assert_eq!(snapshot.physical_protection, "protection-unknown");
    assert_eq!(snapshot.accounting, "charged");
}

#[test]
fn indeterminate_disposal_is_terminal_and_non_addressable() {
    let (error, provider) = close_with_fault(Fault::DisposalIndeterminate);
    assert_eq!(
        error.lifecycle,
        LifecyclePosture::Tombstoned {
            last_stage: PendingStage::Disposal,
            disposition: AllocationPresence::Unknown,
        }
    );
    assert_eq!(provider.report().health, ProviderHealth::Shutdown);
    assert_eq!(provider.report().tombstoned, 1);
    assert_eq!(provider.report().quarantined, 0);
    let snapshot = error.snapshot();
    assert_eq!(snapshot.lifecycle, "tombstoned");
    assert_eq!(snapshot.allocation, "unknown-no-address-retained");
    assert_eq!(snapshot.pending_substage, Some("indeterminate"));
}

#[test]
fn successful_close_uses_the_exact_order_once() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = FaultProvider::new(Fault::None);
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    let operation = validated.operation_report(&token).unwrap();
    assert_eq!(operation.operation, SecretOperation::Encode);
    assert_eq!(operation.wipe, WipeEvidence::WipeNotCompleted);
    assert_eq!(
        operation.physical_protection,
        PhysicalProtection::ProtectionConfirmedAbsent
    );
    assert_eq!(operation.accounting, AccountingPosture::Charged);
    assert_eq!(operation.lifecycle, LifecyclePosture::Live);
    assert_eq!(operation.allocation, AllocationPosture::Present);
    assert_eq!(
        operation.assurance.attestation_posture,
        AttestationPosture::NotAttested
    );
    assert_eq!(
        operation.assurance.secret_policy_posture,
        SecretPolicyPosture::BestEffort
    );
    assert_eq!(
        operation.assurance.secret_decode_backend.backend.as_str(),
        "scalar-constant-time-oriented"
    );
    let close = validated.try_close().unwrap();
    let close_snapshot = close.snapshot();
    assert_eq!(close_snapshot.wipe, "wiped-best-effort");
    assert_eq!(
        close_snapshot.physical_protection,
        "protection-confirmed-absent"
    );
    assert_eq!(close_snapshot.accounting, "reconciled");
    assert_eq!(close_snapshot.lifecycle, "closed");
    assert_eq!(close_snapshot.allocation, "absent");
    assert_eq!(
        provider.trace(),
        ["wipe", "protection", "accounting", "disposal"]
    );
    assert_eq!(provider.report().active_and_reserved, 0);
}

#[cfg(not(base64_ng_require_high_assurance))]
#[test]
fn attested_token_requires_the_explicit_build_policy() {
    let context = AssuranceContext::new();
    let provider = FaultProvider::attested();
    assert_eq!(
        context.attested_token(&provider).unwrap_err(),
        base64_ng::assurance::AssuranceError::HighAssuranceBuildRequired,
    );
}

#[cfg(base64_ng_require_high_assurance)]
#[test]
fn attested_token_authorizes_only_matching_protected_storage() {
    let context = AssuranceContext::new();
    let provider = FaultProvider::attested();
    let token = context.attested_token(&provider).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    let operation = validated.operation_report(&token).unwrap();
    assert_eq!(
        operation.assurance.attestation_posture,
        AttestationPosture::Attested
    );
    assert_eq!(
        operation.assurance.secret_policy_posture,
        SecretPolicyPosture::HighAssuranceAttested
    );
    assert_eq!(
        operation.physical_protection,
        PhysicalProtection::ProtectionAttested
    );
    assert_eq!(validated.expose_secret().as_bytes(), b"c2VjcmV0");
    assert_eq!(
        validated.try_close().unwrap().wipe,
        WipeEvidence::WipedAttested
    );
}

#[cfg(base64_ng_require_high_assurance)]
#[test]
fn stale_attested_provider_generation_quarantines_before_later_hooks() {
    let context = AssuranceContext::new();
    let provider = FaultProvider::attested();
    let token = context.attested_token(&provider).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    provider.invalidate_health_generation();

    assert_eq!(
        validated.operation_report(&token),
        Err(ProtectionError::StaleAssurance)
    );
    let error = validated.try_close().unwrap_err();
    assert_eq!(provider.trace(), ["wipe"]);
    assert_eq!(error.pending_stage, PendingStage::Wipe);
    assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
}
