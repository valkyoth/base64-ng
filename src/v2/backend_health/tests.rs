#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
use super::HealthCell;
use super::{BackendHealthState, decode_fault, decode_state, encode_fault};
use crate::BackendFault;
#[cfg(feature = "simd")]
use crate::runtime::{Backend, OperationKind};
#[cfg(all(feature = "std", feature = "simd", target_has_atomic = "ptr"))]
use core::sync::atomic::Ordering;
#[cfg(all(not(feature = "std"), feature = "simd", target_has_atomic = "ptr"))]
extern crate std;

#[test]
fn state_and_fault_encodings_are_closed() {
    assert_eq!(decode_state(0), BackendHealthState::NeverRun);
    assert_eq!(decode_state(1), BackendHealthState::Testing);
    assert_eq!(decode_state(2), BackendHealthState::Healthy);
    assert_eq!(decode_state(3), BackendHealthState::Quarantined);
    for fault in [
        BackendFault::SelfTestFailed,
        BackendFault::OutputMismatch,
        BackendFault::ImpossibleState,
        BackendFault::ScalarRetryFailed,
    ] {
        assert_eq!(decode_fault(encode_fault(fault)), Some(fault));
    }
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
#[test]
fn quarantine_is_permanent_and_generation_is_monotonic() {
    let cell = HealthCell::new();
    let before = cell.snapshot(OperationKind::Encode, Backend::Avx2);
    cell.quarantine(BackendFault::OutputMismatch);
    let after = cell.snapshot(OperationKind::Encode, Backend::Avx2);
    cell.quarantine(BackendFault::ImpossibleState);
    let repeated = cell.snapshot(OperationKind::Encode, Backend::Avx2);
    assert_eq!(before.state, BackendHealthState::NeverRun);
    assert_eq!(after.state, BackendHealthState::Quarantined);
    assert_eq!(after.fault, Some(BackendFault::OutputMismatch));
    assert!(after.generation > before.generation);
    assert_eq!(repeated.generation, after.generation);
    assert_eq!(repeated.fault, Some(BackendFault::ImpossibleState));
}

#[cfg(feature = "simd")]
#[test]
fn forced_direct_backends_pass_known_answers_when_available() {
    assert!(super::kat::run(OperationKind::Encode, Backend::Scalar));
    assert!(super::kat::run(
        OperationKind::StrictDecode,
        Backend::Scalar
    ));
    for backend in super::candidate_backends() {
        if super::kat::available(*backend) {
            assert!(super::kat::run(OperationKind::Encode, *backend));
            assert!(super::kat::run(OperationKind::StrictDecode, *backend));
        }
    }
}

#[cfg(feature = "simd")]
#[test]
fn explicit_initialization_accounts_for_every_operation_pair() {
    let report = super::initialize_backends();
    assert_eq!(
        report.tested + report.unavailable,
        super::candidate_backends().len() * 2
    );
    assert_eq!(report.tested, report.healthy + report.quarantined);
}

#[cfg(feature = "simd")]
#[test]
fn malformed_input_does_not_change_decode_health() {
    let backend = crate::decode_backend::candidate_decode_backend().reported();
    if backend == Backend::Scalar || !super::admit(OperationKind::StrictDecode, backend) {
        return;
    }
    let before = super::snapshot(OperationKind::StrictDecode, backend);
    let mut output = [0x55; 16];
    let _ = crate::decode_backend::decode_slice::<crate::Standard, true>(b"!!!!", &mut output);
    let after = super::snapshot(OperationKind::StrictDecode, backend);
    assert_eq!(after, before);
}

#[cfg(all(feature = "std", feature = "simd", target_has_atomic = "ptr"))]
#[test]
fn concurrent_first_use_converges_without_waiting_on_testing() {
    use std::sync::Arc;

    let cell = Arc::new(HealthCell::new());
    let mut threads = std::vec::Vec::new();
    for _ in 0..8 {
        let cell = Arc::clone(&cell);
        threads.push(std::thread::spawn(move || {
            cell.ensure(OperationKind::Encode, Backend::Scalar)
        }));
    }
    let mut admitted = 0;
    for thread in threads {
        if thread.join().unwrap() {
            admitted += 1;
        }
    }
    assert!(admitted >= 1);
    assert_eq!(
        cell.snapshot(OperationKind::Encode, Backend::Scalar).state,
        BackendHealthState::Healthy
    );
}

#[cfg(all(feature = "std", feature = "simd", target_has_atomic = "ptr"))]
#[test]
fn reentry_falls_back_and_initializer_panics_are_contained() {
    let cell = HealthCell::new();
    cell.state.store(super::TESTING, Ordering::Release);
    assert!(!cell.ensure(OperationKind::Encode, Backend::Scalar));
    let panic_cell = HealthCell::new();
    assert!(!panic_cell.ensure_with(|| panic!("injected KAT panic")));
    let snapshot = panic_cell.snapshot(OperationKind::Encode, Backend::Scalar);
    assert_eq!(snapshot.state, BackendHealthState::Quarantined);
    assert_eq!(snapshot.fault, Some(BackendFault::SelfTestFailed));
}

#[cfg(all(not(feature = "std"), feature = "simd", target_has_atomic = "ptr"))]
#[test]
fn no_std_unwind_quarantines_panicking_initializer() {
    let cell = HealthCell::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = cell.ensure_with(|| panic!("injected no_std KAT panic"));
    }));
    assert!(result.is_err());
    let snapshot = cell.snapshot(OperationKind::Encode, Backend::Avx2);
    assert_eq!(snapshot.state, BackendHealthState::Quarantined);
    assert_eq!(snapshot.fault, Some(BackendFault::SelfTestFailed));
    assert!(!cell.ensure_with(|| true));
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
#[test]
fn failed_self_test_is_permanently_quarantined() {
    let cell = HealthCell::new();
    assert!(!cell.ensure_with(|| false));
    let failed = cell.snapshot(OperationKind::StrictDecode, Backend::Avx2);
    assert_eq!(failed.state, BackendHealthState::Quarantined);
    assert_eq!(failed.fault, Some(BackendFault::SelfTestFailed));
    assert!(!cell.ensure_with(|| true));
    assert_eq!(
        cell.snapshot(OperationKind::StrictDecode, Backend::Avx2),
        failed
    );
}
