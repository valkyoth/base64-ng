//! Allocation-specific protected typestate owner and ordered teardown.

use core::{cell::UnsafeCell, marker::PhantomData};

use super::{
    AccountingPosture, AllocationPresence, AssuranceContext, AssuranceGenerations, AssuranceLevel,
    AssuranceToken, AttestationEvidence, CleanupError, CleanupOutcome, CleanupReport,
    DisposalResult, LifecyclePosture, PendingStage, PhysicalProtection, ProtectedMemoryProvider,
    ProtectionError, ProtectionRequest, ProviderAccess, ProviderHealth, ProviderOperationResult,
    QuarantineRecord, TeardownCursor, TeardownOperation, ThreadMovableProvider, WipeEvidence,
};

/// Protected allocation has not yet accepted plaintext.
pub struct Uninitialized {
    _private: (),
}

/// Protected allocation contains plaintext not yet released by a result gate.
pub struct Unvalidated {
    _private: (),
}

/// Protected allocation passed its assured operation and result gate.
pub struct Validated {
    _private: (),
}

/// Exact provider-owned protected allocation in one typestate.
///
/// Moving this handle may not move the provider's backing bytes. The type is
/// `!Sync`, `!UnwindSafe`, and `!RefUnwindSafe`. Those traits communicate API
/// friction only; cleanup remains correct when callers use `AssertUnwindSafe`.
pub struct ProtectedSecret<'provider, P, State, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    provider: &'provider P,
    handle: Option<P::Handle>,
    context: &'provider AssuranceContext,
    generations: AssuranceGenerations,
    attestation: Option<AttestationEvidence>,
    provider_generation: usize,
    health_generation: usize,
    protection_generation: usize,
    initialized_len: usize,
    _state: PhantomData<State>,
    _level: PhantomData<Level>,
    _not_sync_or_unwind_safe: PhantomData<(UnsafeCell<()>, &'provider mut dyn FnMut())>,
}

impl<'provider, P, Level> ProtectedSecret<'provider, P, Uninitialized, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    /// Reserves all bounded resources and creates zeroed protected storage.
    pub fn try_new(
        provider: &'provider P,
        token: &AssuranceToken<'provider, Level>,
        logical_bytes: usize,
    ) -> Result<Self, ProtectionError> {
        token
            .revalidate()
            .map_err(|_| ProtectionError::StaleAssurance)?;
        if provider.health() != ProviderHealth::Healthy {
            return Err(ProtectionError::ProviderUnavailable);
        }
        if let Some(evidence) = token.evidence()
            && (evidence.provider_identity() != provider.provider_identity()
                || evidence.provider_generation() != provider.provider_generation())
        {
            return Err(ProtectionError::StaleAssurance);
        }
        let request = ProtectionRequest::new(
            logical_bytes,
            provider.limits().page_size,
            AssuranceToken::<Level>::requires_attestation(),
        )?;
        let access = ProviderAccess::new();
        let reservation = provider.reserve(&access, request)?;
        let handle = provider.materialize(&access, reservation)?;
        if provider.logical_len(&access, &handle) != logical_bytes {
            let mut owner = Self::from_handle(provider, token, handle);
            let _ = owner.close_inner();
            return Err(ProtectionError::ActualRangeExceededReservation);
        }
        if AssuranceToken::<Level>::requires_attestation()
            && provider.physical_protection(&access, &handle)
                != PhysicalProtection::ProtectionAttested
        {
            let mut owner = Self::from_handle(provider, token, handle);
            let _ = owner.close_inner();
            return Err(ProtectionError::ProtectionUnavailable);
        }
        Ok(Self::from_handle(provider, token, handle))
    }

    fn from_handle(
        provider: &'provider P,
        token: &AssuranceToken<'provider, Level>,
        handle: P::Handle,
    ) -> Self {
        Self {
            provider,
            handle: Some(handle),
            context: token.context(),
            generations: token.generations(),
            attestation: token.evidence(),
            provider_generation: provider.provider_generation(),
            health_generation: provider.health_generation(),
            protection_generation: provider.protection_generation(),
            initialized_len: 0,
            _state: PhantomData,
            _level: PhantomData,
            _not_sync_or_unwind_safe: PhantomData,
        }
    }

    pub(crate) fn begin_unvalidated(
        mut self,
        token: &AssuranceToken<'provider, Level>,
    ) -> Result<ProtectedSecret<'provider, P, Unvalidated, Level>, (ProtectionError, Self)> {
        if let Err(error) = self.revalidate(token) {
            return Err((error, self));
        }
        match self.transition() {
            Ok(next) => Ok(next),
            Err(error) => Err((error, self)),
        }
    }
}

impl<'provider, P, Level> ProtectedSecret<'provider, P, Unvalidated, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    pub(crate) fn bytes_mut(&mut self) -> Result<&mut [u8], ProtectionError> {
        let handle = self
            .handle
            .as_mut()
            .ok_or(ProtectionError::ProviderUnavailable)?;
        Ok(self.provider.bytes_mut(&ProviderAccess::new(), handle))
    }

    pub(crate) fn set_initialized_len(&mut self, len: usize) -> Result<(), ProtectionError> {
        if len > self.capacity() {
            return Err(ProtectionError::ActualRangeExceededReservation);
        }
        self.initialized_len = len;
        Ok(())
    }

    pub(crate) fn validate(
        mut self,
        token: &AssuranceToken<'provider, Level>,
    ) -> Result<ProtectedSecret<'provider, P, Validated, Level>, (ProtectionError, Self)> {
        if let Err(error) = self.revalidate(token) {
            return Err((error, self));
        }
        match self.transition() {
            Ok(next) => Ok(next),
            Err(error) => Err((error, self)),
        }
    }
}

impl<P, Level> ProtectedSecret<'_, P, Validated, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    /// Creates an explicit redacted exposure guard over validated bytes.
    #[must_use]
    pub fn expose_secret(&self) -> ExposedProtectedSecret<'_> {
        let bytes = self.handle.as_ref().map_or(&[][..], |handle| {
            self.provider.bytes(&ProviderAccess::new(), handle)
        });
        ExposedProtectedSecret {
            bytes: &bytes[..self.initialized_len],
        }
    }
}

impl<'provider, P, State, Level> ProtectedSecret<'provider, P, State, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    /// Complete logical allocation capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.handle.as_ref().map_or(0, |handle| {
            self.provider.logical_len(&ProviderAccess::new(), handle)
        })
    }

    /// Initialized plaintext bytes tracked by this typestate.
    #[must_use]
    pub const fn initialized_len(&self) -> usize {
        self.initialized_len
    }

    /// Consumes the public owner and reports the ordered cleanup result.
    pub fn try_close(mut self) -> Result<CleanupReport, CleanupError> {
        self.close_inner()
    }

    fn transition<Next>(
        &mut self,
    ) -> Result<ProtectedSecret<'provider, P, Next, Level>, ProtectionError> {
        let handle = self
            .handle
            .take()
            .ok_or(ProtectionError::ProviderUnavailable)?;
        Ok(ProtectedSecret {
            provider: self.provider,
            handle: Some(handle),
            context: self.context,
            generations: self.generations,
            attestation: self.attestation,
            provider_generation: self.provider_generation,
            health_generation: self.health_generation,
            protection_generation: self.protection_generation,
            initialized_len: self.initialized_len,
            _state: PhantomData,
            _level: PhantomData,
            _not_sync_or_unwind_safe: PhantomData,
        })
    }

    fn revalidate(&self, token: &AssuranceToken<'provider, Level>) -> Result<(), ProtectionError> {
        token
            .revalidate()
            .and_then(|()| self.context.revalidate_snapshot::<Level>(self.generations))
            .map_err(|_| ProtectionError::StaleAssurance)?;
        if !core::ptr::eq(token.context(), self.context)
            || token.generations() != self.generations
            || self.provider.provider_generation() != self.provider_generation
            || self.provider.health_generation() != self.health_generation
            || self.provider.protection_generation() != self.protection_generation
            || self.provider.health() != ProviderHealth::Healthy
        {
            return Err(ProtectionError::StaleAssurance);
        }
        Ok(())
    }

    fn close_inner(&mut self) -> Result<CleanupReport, CleanupError> {
        let Some(mut handle) = self.handle.take() else {
            return Ok(closed_report(WipeEvidence::WipedBestEffort));
        };
        let mut cursor = TeardownCursor::new();
        cursor.begin(TeardownOperation::Wipe);
        let access = ProviderAccess::new();
        crate::wipe_bytes(self.provider.bytes_mut(&access, &mut handle));
        let context_current = self
            .context
            .revalidate_wipe_snapshot::<Level>(self.generations)
            .is_ok();
        let attestation_current = self.attestation.is_some_and(|evidence| {
            evidence.provider_identity() == self.provider.provider_identity()
                && evidence.provider_generation() == self.provider_generation
                && self.provider.provider_generation() == self.provider_generation
                && self.provider.health_generation() == self.health_generation
                && self.provider.protection_generation() == self.protection_generation
                && self.provider.health() == ProviderHealth::Healthy
        });
        let confirmation =
            self.provider
                .confirm_wipe(&access, &handle, self.attestation, &mut cursor);
        cursor.disposition = confirmation.result.journal_disposition();
        let required_attested = AssuranceToken::<Level>::requires_attestation();
        let wipe_sufficient = confirmation.result == ProviderOperationResult::Applied
            && context_current
            && (!required_attested
                || (confirmation.evidence == WipeEvidence::WipedAttested && attestation_current));
        if !wipe_sufficient {
            let wipe = if confirmation.result == ProviderOperationResult::Applied {
                if required_attested && (!context_current || !attestation_current) {
                    WipeEvidence::WipedBestEffort
                } else {
                    confirmation.evidence
                }
            } else {
                WipeEvidence::WipeNotCompleted
            };
            return Err(self.quarantine(handle, PendingStage::Wipe, wipe, cursor));
        }
        let wipe = confirmation.evidence;

        cursor.begin(TeardownOperation::ProtectionRemoval);
        let removal = self
            .provider
            .remove_protection(&access, &mut handle, &mut cursor);
        cursor.disposition = removal.journal_disposition();
        if removal != ProviderOperationResult::Applied {
            let physical = if removal == ProviderOperationResult::Indeterminate {
                PhysicalProtection::ProtectionUnknown
            } else {
                self.provider.physical_protection(&access, &handle)
            };
            return Err(self.quarantine_with_physical(
                handle,
                PendingStage::ProtectionRemoval,
                wipe,
                physical,
                cursor,
            ));
        }

        cursor.begin(TeardownOperation::AccountingReconciliation);
        let accounting = self
            .provider
            .reconcile_accounting(&access, &mut handle, &mut cursor);
        cursor.disposition = accounting.journal_disposition();
        if accounting != ProviderOperationResult::Applied {
            return Err(self.quarantine_with_physical(
                handle,
                PendingStage::AccountingReconciliation,
                wipe,
                PhysicalProtection::ProtectionConfirmedAbsent,
                cursor,
            ));
        }

        cursor.begin(TeardownOperation::Disposal);
        match self.provider.dispose(&access, handle, &mut cursor) {
            DisposalResult::Applied => Ok(closed_report(wipe)),
            DisposalResult::NotApplied(handle) => Err(self.quarantine_with_physical(
                handle,
                PendingStage::Disposal,
                wipe,
                PhysicalProtection::ProtectionConfirmedAbsent,
                cursor,
            )),
            DisposalResult::AllocationPresenceUnknown => Err(CleanupError {
                pending_stage: PendingStage::Disposal,
                wipe,
                physical_protection: PhysicalProtection::ProtectionUnknown,
                accounting: AccountingPosture::Charged,
                lifecycle: LifecyclePosture::Tombstoned {
                    last_stage: PendingStage::Disposal,
                    disposition: AllocationPresence::Unknown,
                },
                retry_attempt: 1,
                provider_health: ProviderHealth::Shutdown,
            }),
        }
    }

    fn quarantine(
        &self,
        handle: P::Handle,
        pending_stage: PendingStage,
        wipe: WipeEvidence,
        cursor: TeardownCursor,
    ) -> CleanupError {
        let physical = self
            .provider
            .physical_protection(&ProviderAccess::new(), &handle);
        self.quarantine_with_physical(handle, pending_stage, wipe, physical, cursor)
    }

    fn quarantine_with_physical(
        &self,
        handle: P::Handle,
        pending_stage: PendingStage,
        wipe: WipeEvidence,
        physical_protection: PhysicalProtection,
        cursor: TeardownCursor,
    ) -> CleanupError {
        let record = QuarantineRecord {
            pending_stage,
            wipe,
            physical_protection,
            accounting: AccountingPosture::Charged,
            cursor,
            retry_attempt: 1,
        };
        self.provider
            .quarantine(&ProviderAccess::new(), handle, record);
        let health = self.provider.health();
        CleanupError {
            pending_stage,
            wipe,
            physical_protection,
            accounting: AccountingPosture::Charged,
            lifecycle: LifecyclePosture::Quarantined { pending_stage },
            retry_attempt: 1,
            provider_health: health,
        }
    }
}

impl<P, State, Level> Drop for ProtectedSecret<'_, P, State, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    fn drop(&mut self) {
        let _ = self.close_inner();
    }
}

impl<P, State, Level> core::fmt::Debug for ProtectedSecret<'_, P, State, Level>
where
    P: ProtectedMemoryProvider,
    Level: AssuranceLevel,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ProtectedSecret")
            .field("bytes", &"<redacted>")
            .field("initialized_len", &self.initialized_len)
            .field("capacity", &self.capacity())
            .finish_non_exhaustive()
    }
}

#[allow(unsafe_code)]
unsafe impl<P, State, Level> Send for ProtectedSecret<'_, P, State, Level>
where
    P: ThreadMovableProvider + Sync,
    P::Handle: Send,
    State: Send,
    Level: AssuranceLevel + Send,
{
}

/// Explicit borrowed exposure of one validated protected secret.
pub struct ExposedProtectedSecret<'a> {
    bytes: &'a [u8],
}

impl ExposedProtectedSecret<'_> {
    /// Returns the explicitly exposed bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}

impl AsRef<[u8]> for ExposedProtectedSecret<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl core::fmt::Debug for ExposedProtectedSecret<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("ExposedProtectedSecret(<redacted>)")
    }
}

fn closed_report(wipe: WipeEvidence) -> CleanupReport {
    CleanupReport {
        outcome: CleanupOutcome::Closed,
        wipe,
        physical_protection: PhysicalProtection::ProtectionConfirmedAbsent,
        accounting: AccountingPosture::Reconciled,
        lifecycle: LifecyclePosture::Closed,
    }
}
