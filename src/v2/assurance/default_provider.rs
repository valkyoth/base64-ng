//! Bounded local best-effort provider used by portable deployments and tests.

mod helpers;

use helpers::{check_limit, effective_pages, next_generation, report_from_state};

use alloc::vec::Vec;
use core::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{
    AttestationEvidence, DisposalResult, JournalDisposition, PhysicalProtection,
    ProtectedMemoryProvider, ProtectionError, ProtectionRequest, ProviderAccess, ProviderHealth,
    ProviderLimits, ProviderOperationResult, ProviderReport, QuarantineRecord, ResourceKind,
    TeardownCursor, WipeConfirmation, WipeEvidence,
};

static NEXT_PROVIDER_IDENTITY: AtomicUsize = AtomicUsize::new(1);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotLifecycle {
    Free,
    Reserved,
    Active,
    Quarantined,
    PermanentlyQuarantined,
}

struct Slot {
    generation: usize,
    lifecycle: SlotLifecycle,
    logical_bytes: usize,
    effective_pages: usize,
    retries: usize,
    storage: Option<Vec<u8>>,
    cursor: TeardownCursor,
}

impl Slot {
    const fn free() -> Self {
        Self {
            generation: 1,
            lifecycle: SlotLifecycle::Free,
            logical_bytes: 0,
            effective_pages: 0,
            retries: 0,
            storage: None,
            cursor: TeardownCursor::new(),
        }
    }

    fn reset(&mut self) {
        self.generation = self.generation.checked_add(1).unwrap_or(0);
        self.lifecycle = SlotLifecycle::Free;
        self.logical_bytes = 0;
        self.effective_pages = 0;
        self.retries = 0;
        self.storage = None;
        self.cursor = TeardownCursor::new();
    }
}

struct ProviderState<const SLOTS: usize> {
    health: ProviderHealth,
    health_generation: usize,
    protection_generation: usize,
    slots: [Slot; SLOTS],
}

impl<const SLOTS: usize> Drop for ProviderState<SLOTS> {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            if let Some(storage) = slot.storage.as_deref_mut() {
                crate::wipe_bytes(storage);
            }
        }
    }
}

#[doc(hidden)]
/// Reservation made before a best-effort allocation exists.
pub struct BestEffortReservation {
    slot: usize,
    generation: usize,
    request: ProtectionRequest,
}

#[doc(hidden)]
/// Unique active allocation handle.
pub struct BestEffortHandle {
    slot: usize,
    generation: usize,
    reserved_pages: usize,
    bytes: Vec<u8>,
    _not_thread_or_unwind_safe: PhantomData<(UnsafeCell<()>, &'static mut dyn FnMut())>,
}

/// Finite local provider with best-effort wiping and no OS memory protection.
///
/// The provider is intentionally `!Sync`. It pre-reserves one registry slot
/// and all count/byte/page budget before allocating zeroed storage. Its default
/// hooks are conclusive, so quarantine is reachable only through recovery of a
/// previously transferred record or fault-injected provider wrappers.
pub struct BestEffortProvider<const SLOTS: usize> {
    identity: usize,
    limits: ProviderLimits,
    state: RefCell<ProviderState<SLOTS>>,
}
impl<const SLOTS: usize> BestEffortProvider<SLOTS> {
    /// Creates a bounded provider.
    pub fn new(limits: ProviderLimits) -> Result<Self, ProtectionError> {
        if limits.page_size == 0
            || limits.max_identities > SLOTS
            || limits.max_registry_entries > SLOTS
            || limits.max_retry_attempts == 0
            || limits.max_maintenance_work == 0
        {
            return Err(ProtectionError::InvalidLimits);
        }
        let identity = NEXT_PROVIDER_IDENTITY
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ProtectionError::ProviderUnavailable)?;
        Ok(Self {
            identity,
            limits,
            state: RefCell::new(ProviderState {
                health: ProviderHealth::Healthy,
                health_generation: 1,
                protection_generation: 1,
                slots: core::array::from_fn(|_| Slot::free()),
            }),
        })
    }

    /// Runs bounded in-process recovery for quarantined entries.
    ///
    /// Every wipe retry restarts at byte zero. This default provider has no
    /// fallible later hooks; successful retries close their slots. Health is
    /// not upgraded automatically.
    pub fn maintain(&self) -> usize {
        let mut state = self.state.borrow_mut();
        let mut work = 0;
        let mut shutdown = false;
        for slot in &mut state.slots {
            if work >= self.limits.max_maintenance_work {
                break;
            }
            if slot.lifecycle != SlotLifecycle::Quarantined {
                continue;
            }
            work += 1;
            if slot.retries >= self.limits.max_retry_attempts {
                slot.lifecycle = SlotLifecycle::PermanentlyQuarantined;
                shutdown = true;
                continue;
            }
            slot.retries += 1;
            if let Some(storage) = slot.storage.as_deref_mut() {
                crate::wipe_bytes(storage);
            }
            slot.storage = None;
            slot.reset();
        }
        if shutdown {
            state.health = ProviderHealth::Shutdown;
            state.health_generation = next_generation(state.health_generation);
        }
        work
    }

    /// Restores admission only after quarantine is empty and no tombstone exists.
    pub fn restore_health_after_self_check(&self) -> Result<(), ProtectionError> {
        let mut state = self.state.borrow_mut();
        if state.slots.iter().any(|slot| {
            matches!(
                slot.lifecycle,
                SlotLifecycle::Quarantined | SlotLifecycle::PermanentlyQuarantined
            )
        }) {
            return Err(ProtectionError::ProviderUnavailable);
        }
        let health_generation = next_generation(state.health_generation);
        let protection_generation = next_generation(state.protection_generation);
        if health_generation == 0 || protection_generation == 0 {
            state.health = ProviderHealth::Shutdown;
            state.health_generation = health_generation;
            state.protection_generation = protection_generation;
            return Err(ProtectionError::ProviderUnavailable);
        }
        state.health = ProviderHealth::Healthy;
        state.health_generation = health_generation;
        state.protection_generation = protection_generation;
        Ok(())
    }

    fn release_reservation(&self, reservation: &BestEffortReservation) {
        let mut state = self.state.borrow_mut();
        if let Some(slot) = state.slots.get_mut(reservation.slot)
            && slot.generation == reservation.generation
            && slot.lifecycle == SlotLifecycle::Reserved
        {
            slot.reset();
        }
    }
}

#[allow(unsafe_code)]
unsafe impl<const SLOTS: usize> ProtectedMemoryProvider for BestEffortProvider<SLOTS> {
    type Handle = BestEffortHandle;
    type Reservation = BestEffortReservation;

    fn provider_identity(&self) -> usize {
        self.identity
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
        self.state.borrow().health
    }

    fn limits(&self) -> ProviderLimits {
        self.limits
    }

    fn report(&self) -> ProviderReport {
        let state = self.state.borrow();
        let mut report = ProviderReport {
            health: state.health,
            health_generation: state.health_generation,
            protection_generation: state.protection_generation,
            active_and_reserved: 0,
            quarantined: 0,
            tombstoned: 0,
            charged_logical_bytes: 0,
            charged_effective_pages: 0,
        };
        for slot in &state.slots {
            match slot.lifecycle {
                SlotLifecycle::Reserved | SlotLifecycle::Active => report.active_and_reserved += 1,
                SlotLifecycle::Quarantined | SlotLifecycle::PermanentlyQuarantined => {
                    report.quarantined += 1;
                }
                SlotLifecycle::Free => continue,
            }
            report.charged_logical_bytes += slot.logical_bytes;
            report.charged_effective_pages += slot.effective_pages;
        }
        report
    }

    fn reserve(
        &self,
        _access: &ProviderAccess,
        request: ProtectionRequest,
    ) -> Result<Self::Reservation, ProtectionError> {
        if request.requires_attestation() {
            return Err(ProtectionError::ProtectionUnavailable);
        }
        let mut state = self.state.borrow_mut();
        if state.health != ProviderHealth::Healthy {
            return Err(ProtectionError::ProviderUnavailable);
        }
        let report = report_from_state(&state);
        let used_identities = report.active_and_reserved + report.quarantined + report.tombstoned;
        let limit_result = check_limit(
            used_identities,
            1,
            self.limits.max_identities,
            ResourceKind::Identities,
        )
        .and_then(|()| {
            check_limit(
                used_identities,
                1,
                self.limits.max_registry_entries,
                ResourceKind::RegistryEntries,
            )
        })
        .and_then(|()| {
            check_limit(
                report.charged_logical_bytes,
                request.logical_bytes(),
                self.limits.max_logical_bytes,
                ResourceKind::LogicalBytes,
            )
        })
        .and_then(|()| {
            check_limit(
                report.charged_effective_pages,
                request.reserved_pages(),
                self.limits.max_effective_pages,
                ResourceKind::EffectivePages,
            )
        });
        if let Err(error) = limit_result {
            state.health = ProviderHealth::Exhausted;
            state.health_generation = next_generation(state.health_generation);
            return Err(error);
        }
        let Some((slot_index, slot)) = state
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.lifecycle == SlotLifecycle::Free && slot.generation != 0)
        else {
            state.health = ProviderHealth::Exhausted;
            state.health_generation = next_generation(state.health_generation);
            return Err(ProtectionError::ProtectionResourceExhausted(
                ResourceKind::RegistryEntries,
            ));
        };
        slot.lifecycle = SlotLifecycle::Reserved;
        slot.logical_bytes = request.logical_bytes();
        slot.effective_pages = request.reserved_pages();
        Ok(BestEffortReservation {
            slot: slot_index,
            generation: slot.generation,
            request,
        })
    }

    fn materialize(
        &self,
        _access: &ProviderAccess,
        reservation: Self::Reservation,
    ) -> Result<Self::Handle, ProtectionError> {
        let mut bytes = Vec::new();
        if bytes
            .try_reserve_exact(reservation.request.logical_bytes())
            .is_err()
        {
            self.release_reservation(&reservation);
            return Err(ProtectionError::ProviderUnavailable);
        }
        bytes.resize(reservation.request.logical_bytes(), 0);
        let actual_pages =
            effective_pages(bytes.as_ptr() as usize, bytes.len(), self.limits.page_size)?;
        if actual_pages > reservation.request.reserved_pages() {
            crate::wipe_bytes(&mut bytes);
            self.release_reservation(&reservation);
            return Err(ProtectionError::ActualRangeExceededReservation);
        }
        let mut state = self.state.borrow_mut();
        let Some(slot) = state.slots.get_mut(reservation.slot) else {
            crate::wipe_bytes(&mut bytes);
            return Err(ProtectionError::ProviderUnavailable);
        };
        if slot.generation != reservation.generation || slot.lifecycle != SlotLifecycle::Reserved {
            crate::wipe_bytes(&mut bytes);
            return Err(ProtectionError::ProviderUnavailable);
        }
        slot.lifecycle = SlotLifecycle::Active;
        slot.effective_pages = actual_pages;
        Ok(BestEffortHandle {
            slot: reservation.slot,
            generation: reservation.generation,
            reserved_pages: actual_pages,
            bytes,
            _not_thread_or_unwind_safe: PhantomData,
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
        PhysicalProtection::ProtectionConfirmedAbsent
    }

    fn bytes<'handle>(
        &self,
        _access: &ProviderAccess,
        handle: &'handle Self::Handle,
    ) -> &'handle [u8] {
        &handle.bytes
    }

    fn bytes_mut<'handle>(
        &self,
        _access: &ProviderAccess,
        handle: &'handle mut Self::Handle,
    ) -> &'handle mut [u8] {
        &mut handle.bytes
    }

    fn confirm_wipe(
        &self,
        _access: &ProviderAccess,
        _handle: &Self::Handle,
        _attestation: Option<AttestationEvidence>,
        cursor: &mut TeardownCursor,
    ) -> WipeConfirmation {
        cursor.disposition = JournalDisposition::Applied;
        WipeConfirmation {
            result: ProviderOperationResult::Applied,
            evidence: WipeEvidence::WipedBestEffort,
        }
    }

    fn remove_protection(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        cursor.disposition = JournalDisposition::Applied;
        ProviderOperationResult::Applied
    }

    fn reconcile_accounting(
        &self,
        _access: &ProviderAccess,
        _handle: &mut Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> ProviderOperationResult {
        cursor.disposition = JournalDisposition::Applied;
        ProviderOperationResult::Applied
    }

    fn dispose(
        &self,
        _access: &ProviderAccess,
        mut handle: Self::Handle,
        cursor: &mut TeardownCursor,
    ) -> DisposalResult<Self::Handle> {
        cursor.disposition = JournalDisposition::Applied;
        crate::wipe_bytes(&mut handle.bytes);
        let mut state = self.state.borrow_mut();
        if let Some(slot) = state.slots.get_mut(handle.slot)
            && slot.generation == handle.generation
            && slot.lifecycle == SlotLifecycle::Active
        {
            slot.reset();
        }
        DisposalResult::Applied
    }

    fn quarantine(&self, _access: &ProviderAccess, handle: Self::Handle, record: QuarantineRecord) {
        let mut state = self.state.borrow_mut();
        state.health = ProviderHealth::Degraded;
        state.health_generation = next_generation(state.health_generation);
        if let Some(slot) = state.slots.get_mut(handle.slot)
            && slot.generation == handle.generation
        {
            slot.lifecycle = SlotLifecycle::Quarantined;
            slot.retries = record.retry_attempt;
            slot.cursor = record.cursor;
            slot.logical_bytes = handle.bytes.len();
            slot.effective_pages = handle.reserved_pages;
            slot.storage = Some(handle.bytes);
        }
    }
}
