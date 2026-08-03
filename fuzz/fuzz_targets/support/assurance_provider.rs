use std::{cell::RefCell, vec::Vec};

use base64_ng::assurance::{
    AccountingPosture, AttestationEvidence, DisposalResult, JournalDisposition, PendingStage,
    PhysicalProtection, ProtectedMemoryProvider, ProtectionError, ProtectionRequest,
    ProviderAccess, ProviderHealth, ProviderLimits, ProviderOperationResult, ProviderReport,
    QuarantineRecord, ResourceKind, TeardownCursor, WipeConfirmation, WipeEvidence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fault {
    None,
    WipeNotApplied,
    WipeIndeterminate,
    ProtectionNotApplied,
    ProtectionIndeterminate,
    AccountingNotApplied,
    AccountingIndeterminate,
    DisposalNotApplied,
    DisposalIndeterminate,
}

impl Fault {
    pub const ALL: [Self; 9] = [
        Self::None,
        Self::WipeNotApplied,
        Self::WipeIndeterminate,
        Self::ProtectionNotApplied,
        Self::ProtectionIndeterminate,
        Self::AccountingNotApplied,
        Self::AccountingIndeterminate,
        Self::DisposalNotApplied,
        Self::DisposalIndeterminate,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Empty,
    Live,
    Wiped,
    Unprotected,
    Reconciled,
    Quarantined,
    Tombstoned,
    Closed,
}

pub struct Reservation {
    len: usize,
}

pub struct Handle {
    bytes: Vec<u8>,
}

struct State {
    phase: Phase,
    quarantine: Option<Vec<u8>>,
    record: Option<QuarantineRecord>,
    calls: [usize; 4],
}

pub struct ScheduledProvider {
    fault: Fault,
    state: RefCell<State>,
}

impl ScheduledProvider {
    pub fn new(fault: Fault) -> Self {
        Self {
            fault,
            state: RefCell::new(State {
                phase: Phase::Empty,
                quarantine: None,
                record: None,
                calls: [0; 4],
            }),
        }
    }

    pub fn assert_terminal_claims(&self, error: Option<&base64_ng::assurance::CleanupError>) {
        let state = self.state.borrow();
        match self.fault {
            Fault::None => {
                assert!(error.is_none());
                assert_eq!(state.phase, Phase::Closed);
                assert_eq!(state.calls, [1, 1, 1, 1]);
            }
            Fault::WipeNotApplied | Fault::WipeIndeterminate => {
                let error = error.unwrap();
                assert_eq!(error.pending_stage, PendingStage::Wipe);
                assert_eq!(error.wipe, WipeEvidence::WipeNotCompleted);
                assert_eq!(error.accounting, AccountingPosture::Charged);
                assert_eq!(state.calls, [1, 0, 0, 0]);
            }
            Fault::ProtectionNotApplied | Fault::ProtectionIndeterminate => {
                let error = error.unwrap();
                assert_eq!(error.pending_stage, PendingStage::ProtectionRemoval);
                assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
                assert_eq!(state.calls, [1, 1, 0, 0]);
                if self.fault == Fault::ProtectionIndeterminate {
                    assert_eq!(
                        error.physical_protection,
                        PhysicalProtection::ProtectionUnknown
                    );
                }
            }
            Fault::AccountingNotApplied | Fault::AccountingIndeterminate => {
                let error = error.unwrap();
                assert_eq!(error.pending_stage, PendingStage::AccountingReconciliation);
                assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
                assert_eq!(
                    error.physical_protection,
                    PhysicalProtection::ProtectionConfirmedAbsent
                );
                assert_eq!(error.accounting, AccountingPosture::Charged);
                assert_eq!(state.calls, [1, 1, 1, 0]);
            }
            Fault::DisposalNotApplied => {
                let error = error.unwrap();
                assert_eq!(error.pending_stage, PendingStage::Disposal);
                assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
                assert_eq!(error.accounting, AccountingPosture::Charged);
                assert_eq!(state.calls, [1, 1, 1, 1]);
                assert_eq!(state.phase, Phase::Quarantined);
            }
            Fault::DisposalIndeterminate => {
                let error = error.unwrap();
                assert_eq!(error.pending_stage, PendingStage::Disposal);
                assert_eq!(error.wipe, WipeEvidence::WipedBestEffort);
                assert_eq!(
                    error.physical_protection,
                    PhysicalProtection::ProtectionUnknown
                );
                assert_eq!(state.calls, [1, 1, 1, 1]);
                assert_eq!(state.phase, Phase::Tombstoned);
                assert!(state.quarantine.is_none());
            }
        }
        if let Some(bytes) = state.quarantine.as_deref() {
            assert!(bytes.iter().all(|byte| *byte == 0));
            assert!(state.record.is_some());
        }
    }

    fn operation_result(
        &self,
        not_applied: Fault,
        indeterminate: Fault,
    ) -> ProviderOperationResult {
        if self.fault == not_applied {
            ProviderOperationResult::NotApplied
        } else if self.fault == indeterminate {
            ProviderOperationResult::Indeterminate
        } else {
            ProviderOperationResult::Applied
        }
    }
}

// SAFETY: This fuzz-only provider owns one stable Vec handle, enforces unique
// phase transitions, never exposes aliases, has finite one-slot accounting,
// and makes every hook non-unwinding. Assertions are the fuzz oracle and abort
// an invalid crate call sequence before a false disposition can be returned.
unsafe impl ProtectedMemoryProvider for ScheduledProvider {
    type Handle = Handle;
    type Reservation = Reservation;

    fn provider_identity(&self) -> usize {
        0x51
    }

    fn provider_generation(&self) -> usize {
        1
    }

    fn health_generation(&self) -> usize {
        1
    }

    fn protection_generation(&self) -> usize {
        1
    }

    fn health(&self) -> ProviderHealth {
        match self.state.borrow().phase {
            Phase::Quarantined => ProviderHealth::Degraded,
            Phase::Tombstoned => ProviderHealth::Shutdown,
            _ => ProviderHealth::Healthy,
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
        let phase = self.state.borrow().phase;
        let charged = usize::from(matches!(
            phase,
            Phase::Live
                | Phase::Wiped
                | Phase::Unprotected
                | Phase::Reconciled
                | Phase::Quarantined
                | Phase::Tombstoned
        ));
        ProviderReport {
            health: self.health(),
            health_generation: 1,
            protection_generation: 1,
            active_and_reserved: usize::from(matches!(
                phase,
                Phase::Live | Phase::Wiped | Phase::Unprotected | Phase::Reconciled
            )),
            quarantined: usize::from(phase == Phase::Quarantined),
            permanently_quarantined: 0,
            tombstoned: usize::from(phase == Phase::Tombstoned),
            charged_logical_bytes: charged * 16,
            charged_effective_pages: charged,
        }
    }

    fn reserve(
        &self,
        _access: &ProviderAccess,
        request: ProtectionRequest,
    ) -> Result<Self::Reservation, ProtectionError> {
        if self.state.borrow().phase != Phase::Empty {
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
        self.state.borrow_mut().phase = Phase::Live;
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
        match self.state.borrow().phase {
            Phase::Unprotected | Phase::Reconciled | Phase::Closed => {
                PhysicalProtection::ProtectionConfirmedAbsent
            }
            Phase::Tombstoned => PhysicalProtection::ProtectionUnknown,
            _ => PhysicalProtection::ProtectionConfirmedAbsent,
        }
    }

    fn bytes<'handle>(
        &self,
        _access: &ProviderAccess,
        handle: &'handle Self::Handle,
    ) -> &'handle [u8] {
        assert!(!matches!(
            self.state.borrow().phase,
            Phase::Tombstoned | Phase::Closed
        ));
        &handle.bytes
    }

    fn bytes_mut<'handle>(
        &self,
        _access: &ProviderAccess,
        handle: &'handle mut Self::Handle,
    ) -> &'handle mut [u8] {
        assert!(!matches!(
            self.state.borrow().phase,
            Phase::Tombstoned | Phase::Closed
        ));
        &mut handle.bytes
    }

    fn confirm_wipe(
        &self,
        _access: &ProviderAccess,
        handle: &Self::Handle,
        _attestation: Option<AttestationEvidence>,
        cursor: &mut TeardownCursor,
    ) -> WipeConfirmation {
        let mut state = self.state.borrow_mut();
        assert_eq!(state.phase, Phase::Live);
        assert!(handle.bytes.iter().all(|byte| *byte == 0));
        state.calls[0] += 1;
        assert_eq!(state.calls[0], 1);
        let result = self.operation_result(Fault::WipeNotApplied, Fault::WipeIndeterminate);
        if result == ProviderOperationResult::Applied {
            state.phase = Phase::Wiped;
            cursor.disposition = JournalDisposition::Applied;
        }
        WipeConfirmation {
            result,
            evidence: if result == ProviderOperationResult::Applied {
                WipeEvidence::WipedBestEffort
            } else {
                WipeEvidence::WipeNotCompleted
            },
        }
    }

    fn remove_protection(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        let mut state = self.state.borrow_mut();
        assert_eq!(state.phase, Phase::Wiped);
        state.calls[1] += 1;
        assert_eq!(state.calls[1], 1);
        let result =
            self.operation_result(Fault::ProtectionNotApplied, Fault::ProtectionIndeterminate);
        if result == ProviderOperationResult::Applied {
            state.phase = Phase::Unprotected;
            cursor.disposition = JournalDisposition::Applied;
        }
        result
    }

    fn reconcile_accounting(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        let mut state = self.state.borrow_mut();
        assert_eq!(state.phase, Phase::Unprotected);
        state.calls[2] += 1;
        assert_eq!(state.calls[2], 1);
        let result =
            self.operation_result(Fault::AccountingNotApplied, Fault::AccountingIndeterminate);
        if result == ProviderOperationResult::Applied {
            state.phase = Phase::Reconciled;
            cursor.disposition = JournalDisposition::Applied;
        }
        result
    }

    fn dispose(
        &self,
        _access: &ProviderAccess,
        handle: Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> DisposalResult<Self::Handle> {
        let mut state = self.state.borrow_mut();
        assert_eq!(state.phase, Phase::Reconciled);
        state.calls[3] += 1;
        assert_eq!(state.calls[3], 1);
        match self.fault {
            Fault::DisposalNotApplied => DisposalResult::NotApplied(handle),
            Fault::DisposalIndeterminate => {
                state.phase = Phase::Tombstoned;
                cursor.disposition = JournalDisposition::Indeterminate;
                drop(handle);
                DisposalResult::AllocationPresenceUnknown
            }
            _ => {
                state.phase = Phase::Closed;
                cursor.disposition = JournalDisposition::Applied;
                drop(handle);
                DisposalResult::Applied
            }
        }
    }

    fn quarantine(&self, _access: &ProviderAccess, handle: Self::Handle, record: QuarantineRecord) {
        let mut state = self.state.borrow_mut();
        assert_ne!(state.phase, Phase::Tombstoned);
        assert_eq!(record.retry_attempt, 1);
        match record.pending_stage {
            PendingStage::Wipe => assert_eq!(state.phase, Phase::Live),
            PendingStage::ProtectionRemoval => assert_eq!(state.phase, Phase::Wiped),
            PendingStage::AccountingReconciliation => assert_eq!(state.phase, Phase::Unprotected),
            PendingStage::Disposal => assert_eq!(state.phase, Phase::Reconciled),
            _ => panic!("unknown teardown stage reached fuzz provider"),
        }
        assert!(handle.bytes.iter().all(|byte| *byte == 0));
        state.phase = Phase::Quarantined;
        state.quarantine = Some(handle.bytes);
        state.record = Some(record);
    }
}
