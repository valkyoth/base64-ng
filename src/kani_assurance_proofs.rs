//! Abstract bounded proofs for protected teardown and journal invariants.
//!
//! This is an in-memory protocol model. It is not a persistence, crash
//! recovery, allocator, operating-system, or unsafe-provider proof.

use crate::v2::assurance::{
    AccountingPosture, AllocationPresence, JournalDisposition, LifecyclePosture, PendingStage,
    PhysicalProtection, ProviderOperationResult, WipeEvidence,
};

#[derive(Clone, Copy)]
struct TeardownModel {
    wipe: WipeEvidence,
    protection: PhysicalProtection,
    accounting: AccountingPosture,
    lifecycle: LifecyclePosture,
    addressable_owner: bool,
    disposal: JournalDisposition,
}

fn wipe_satisfies(attested: bool, wipe: WipeEvidence) -> bool {
    if attested {
        matches!(wipe, WipeEvidence::WipedAttested)
    } else {
        matches!(
            wipe,
            WipeEvidence::WipedBestEffort | WipeEvidence::WipedAttested
        )
    }
}

fn pending_stage(model: TeardownModel, attested: bool) -> Option<PendingStage> {
    if !wipe_satisfies(attested, model.wipe) {
        Some(PendingStage::Wipe)
    } else if !matches!(
        model.protection,
        PhysicalProtection::ProtectionConfirmedAbsent
    ) {
        Some(PendingStage::ProtectionRemoval)
    } else if !matches!(model.accounting, AccountingPosture::Reconciled) {
        Some(PendingStage::AccountingReconciliation)
    } else if !matches!(model.disposal, JournalDisposition::Applied) || model.addressable_owner {
        Some(PendingStage::Disposal)
    } else {
        None
    }
}

fn finish_teardown(mut model: TeardownModel, attested: bool) -> TeardownModel {
    if let Some(stage) = pending_stage(model, attested) {
        model.lifecycle = if matches!(model.disposal, JournalDisposition::Indeterminate) {
            model.addressable_owner = false;
            LifecyclePosture::Tombstoned {
                last_stage: stage,
                disposition: AllocationPresence::Unknown,
            }
        } else {
            LifecyclePosture::Quarantined {
                pending_stage: stage,
            }
        };
    } else {
        model.lifecycle = LifecyclePosture::Closed;
    }
    model
}

fn arbitrary_wipe() -> WipeEvidence {
    match kani::any::<u8>() % 3 {
        0 => WipeEvidence::WipeNotCompleted,
        1 => WipeEvidence::WipedBestEffort,
        _ => WipeEvidence::WipedAttested,
    }
}

fn arbitrary_protection() -> PhysicalProtection {
    match kani::any::<u8>() % 3 {
        0 => PhysicalProtection::ProtectionAttested,
        1 => PhysicalProtection::ProtectionConfirmedAbsent,
        _ => PhysicalProtection::ProtectionUnknown,
    }
}

fn arbitrary_disposition() -> JournalDisposition {
    match kani::any::<u8>() % 3 {
        0 => JournalDisposition::NotApplied,
        1 => JournalDisposition::Applied,
        _ => JournalDisposition::Indeterminate,
    }
}

#[kani::proof]
fn closed_requires_all_four_teardown_axes() {
    let attested = kani::any::<bool>();
    let model = TeardownModel {
        wipe: arbitrary_wipe(),
        protection: arbitrary_protection(),
        accounting: if kani::any::<bool>() {
            AccountingPosture::Charged
        } else {
            AccountingPosture::Reconciled
        },
        lifecycle: LifecyclePosture::Live,
        addressable_owner: kani::any::<bool>(),
        disposal: arbitrary_disposition(),
    };
    let finished = finish_teardown(model, attested);

    if matches!(finished.lifecycle, LifecyclePosture::Closed) {
        assert!(wipe_satisfies(attested, finished.wipe));
        assert!(matches!(
            finished.protection,
            PhysicalProtection::ProtectionConfirmedAbsent
        ));
        assert!(matches!(finished.accounting, AccountingPosture::Reconciled));
        assert!(matches!(finished.disposal, JournalDisposition::Applied));
        assert!(!finished.addressable_owner);
    }
}

#[kani::proof]
fn quarantine_preserves_exact_pending_stage_and_wipe_evidence() {
    let attested = kani::any::<bool>();
    let original_wipe = arbitrary_wipe();
    let model = TeardownModel {
        wipe: original_wipe,
        protection: arbitrary_protection(),
        accounting: if kani::any::<bool>() {
            AccountingPosture::Charged
        } else {
            AccountingPosture::Reconciled
        },
        lifecycle: LifecyclePosture::Live,
        addressable_owner: true,
        disposal: JournalDisposition::NotApplied,
    };
    let expected = pending_stage(model, attested).expect("a live owner still requires disposal");
    let finished = finish_teardown(model, attested);

    assert!(finished.wipe == original_wipe);
    assert!(matches!(
        finished.lifecycle,
        LifecyclePosture::Quarantined { pending_stage } if pending_stage == expected
    ));
    assert!(finished.addressable_owner);
    if matches!(original_wipe, WipeEvidence::WipeNotCompleted) {
        assert!(expected == PendingStage::Wipe);
    }
}

#[derive(Clone, Copy)]
struct JournalModel {
    generation: u8,
    dispositions: [JournalDisposition; 4],
    accounting_updates: u8,
}

fn operation_index(operation: u8) -> usize {
    usize::from(operation % 4)
}

fn apply_operation(
    mut journal: JournalModel,
    operation: u8,
    operation_generation: u8,
    result: ProviderOperationResult,
) -> (JournalModel, bool) {
    if journal.generation == 0 || operation_generation != journal.generation {
        return (journal, false);
    }
    let index = operation_index(operation);
    if !matches!(journal.dispositions[index], JournalDisposition::NotApplied) {
        return (journal, false);
    }
    journal.dispositions[index] = match result {
        ProviderOperationResult::Applied => JournalDisposition::Applied,
        ProviderOperationResult::NotApplied => JournalDisposition::NotApplied,
        ProviderOperationResult::Indeterminate => JournalDisposition::Indeterminate,
    };
    if index == 2 && matches!(result, ProviderOperationResult::Applied) {
        journal.accounting_updates += 1;
    }
    (journal, true)
}

fn arbitrary_operation_result() -> ProviderOperationResult {
    match kani::any::<u8>() % 3 {
        0 => ProviderOperationResult::Applied,
        1 => ProviderOperationResult::NotApplied,
        _ => ProviderOperationResult::Indeterminate,
    }
}

#[kani::proof]
fn journal_never_replays_applied_or_indeterminate_operation() {
    let operation = kani::any::<u8>();
    let generation = (kani::any::<u8>() % 254) + 1;
    let initial = JournalModel {
        generation,
        dispositions: [JournalDisposition::NotApplied; 4],
        accounting_updates: 0,
    };
    let result = arbitrary_operation_result();
    let (once, accepted) = apply_operation(initial, operation, generation, result);
    assert!(accepted);
    let (twice, replayed) = apply_operation(once, operation, generation, result);

    if matches!(
        result,
        ProviderOperationResult::Applied | ProviderOperationResult::Indeterminate
    ) {
        assert!(!replayed);
        assert!(twice.accounting_updates == once.accounting_updates);
        assert!(twice.dispositions == once.dispositions);
    }
    assert!(twice.accounting_updates <= 1);
}

#[kani::proof]
fn generation_termination_rejects_every_old_operation_identity() {
    let old = kani::any::<u8>();
    let next = old.checked_add(1).unwrap_or(0);
    let journal = JournalModel {
        generation: next,
        dispositions: [JournalDisposition::NotApplied; 4],
        accounting_updates: 0,
    };
    let (_, accepted) = apply_operation(
        journal,
        kani::any::<u8>(),
        old,
        ProviderOperationResult::Applied,
    );
    assert!(!accepted);
}

#[kani::proof]
fn accounting_transition_applies_exactly_once() {
    let generation = (kani::any::<u8>() % 254) + 1;
    let journal = JournalModel {
        generation,
        dispositions: [JournalDisposition::NotApplied; 4],
        accounting_updates: 0,
    };
    let (once, accepted) =
        apply_operation(journal, 2, generation, ProviderOperationResult::Applied);
    let (twice, replayed) = apply_operation(once, 2, generation, ProviderOperationResult::Applied);
    assert!(accepted);
    assert!(!replayed);
    assert!(twice.accounting_updates == 1);
}

#[kani::proof]
fn tombstone_identity_never_retains_addressable_ownership() {
    let model = TeardownModel {
        wipe: arbitrary_wipe(),
        protection: arbitrary_protection(),
        accounting: AccountingPosture::Charged,
        lifecycle: LifecyclePosture::Live,
        addressable_owner: true,
        disposal: JournalDisposition::Indeterminate,
    };
    let finished = finish_teardown(model, kani::any::<bool>());
    assert!(matches!(
        finished.lifecycle,
        LifecyclePosture::Tombstoned { .. }
    ));
    assert!(!finished.addressable_owner);
}

#[kani::proof]
fn bounded_journal_progress_is_monotonic() {
    let logical_len = usize::from(kani::any::<u8>());
    let previous = usize::from(kani::any::<u8>());
    let requested = usize::from(kani::any::<u8>());
    kani::assume(previous <= logical_len);
    let remaining = logical_len - previous;
    let advanced = requested.min(remaining);
    let next = previous + advanced;

    assert!(next >= previous);
    assert!(next <= logical_len);
    assert!(next - previous <= requested);
}
