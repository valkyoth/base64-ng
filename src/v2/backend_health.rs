//! Backend health, self-test, quarantine, and fallback ownership boundary.

use crate::runtime::{Backend, OperationKind};

use super::contracts::BackendFault;

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "simd")]
mod kat;

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
const NEVER_RUN: usize = 0;
#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const TESTING: usize = 1;
#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const HEALTHY: usize = 2;
#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const QUARANTINED: usize = 3;

/// Runtime state of an ordinary accelerated backend.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BackendHealthState {
    /// No known-answer test has completed.
    NeverRun,
    /// One thread is running the known-answer test.
    Testing,
    /// The known-answer test passed and the backend may execute.
    Healthy,
    /// The backend failed an integrity check and is disabled for this process.
    Quarantined,
}

impl BackendHealthState {
    /// Returns the stable lowercase identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NeverRun => "never-run",
            Self::Testing => "testing",
            Self::Healthy => "healthy",
            Self::Quarantined => "quarantined",
        }
    }
}

/// Atomic snapshot of one backend's health latch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendHealthSnapshot {
    /// Operation whose implementation is covered by the latch.
    pub operation: OperationKind,
    /// Backend covered by the latch.
    pub backend: Backend,
    /// Current process-local health state.
    pub state: BackendHealthState,
    /// Monotonic process-local state generation.
    pub generation: usize,
    /// Most recent integrity fault, when quarantined.
    pub fault: Option<BackendFault>,
}

/// Summary returned by explicit startup backend initialization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BackendInitializationReport {
    /// Operation/backend pairs whose KAT was requested.
    pub tested: usize,
    /// Pairs that are healthy after initialization.
    pub healthy: usize,
    /// Pairs that are permanently quarantined.
    pub quarantined: usize,
    /// Pairs unavailable on this CPU, build, or synchronization target.
    pub unavailable: usize,
}

impl BackendInitializationReport {
    fn record(&mut self, result: InitializationResult) {
        match result {
            InitializationResult::Healthy => {
                self.tested += 1;
                self.healthy += 1;
            }
            InitializationResult::Quarantined => {
                self.tested += 1;
                self.quarantined += 1;
            }
            InitializationResult::Unavailable => self.unavailable += 1,
        }
    }
}

#[derive(Clone, Copy)]
enum InitializationResult {
    Healthy,
    Quarantined,
    Unavailable,
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
struct HealthCell {
    state: AtomicUsize,
    generation: AtomicUsize,
    fault: AtomicUsize,
    #[cfg(all(feature = "std", unix))]
    process_id: AtomicUsize,
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
impl HealthCell {
    const fn new() -> Self {
        Self {
            state: AtomicUsize::new(NEVER_RUN),
            generation: AtomicUsize::new(1),
            fault: AtomicUsize::new(0),
            #[cfg(all(feature = "std", unix))]
            process_id: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self, operation: OperationKind, backend: Backend) -> BackendHealthSnapshot {
        self.refresh_after_fork();
        BackendHealthSnapshot {
            operation,
            backend,
            state: decode_state(self.state.load(Ordering::Acquire)),
            generation: self.generation.load(Ordering::Acquire),
            fault: decode_fault(self.fault.load(Ordering::Acquire)),
        }
    }

    fn ensure(&self, operation: OperationKind, backend: Backend) -> bool {
        self.ensure_with(|| kat::run(operation, backend))
    }

    fn ensure_with(&self, run: impl FnOnce() -> bool) -> bool {
        self.refresh_after_fork();
        let mut run = Some(run);
        loop {
            match self.state.load(Ordering::Acquire) {
                HEALTHY => return true,
                QUARANTINED | TESTING => return false,
                NEVER_RUN => {
                    if self
                        .state
                        .compare_exchange(NEVER_RUN, TESTING, Ordering::AcqRel, Ordering::Acquire)
                        .is_err()
                    {
                        continue;
                    }
                    bump_generation(&self.generation);
                    self.remember_process();
                    let Some(initializer) = run.take() else {
                        self.quarantine(BackendFault::ImpossibleState);
                        return false;
                    };
                    let passed = run_catching_panics(initializer);
                    if passed {
                        self.state.store(HEALTHY, Ordering::Release);
                    } else {
                        self.fault.store(
                            encode_fault(BackendFault::SelfTestFailed),
                            Ordering::Release,
                        );
                        self.state.store(QUARANTINED, Ordering::Release);
                    }
                    bump_generation(&self.generation);
                    return passed;
                }
                _ => {
                    self.quarantine(BackendFault::ImpossibleState);
                    return false;
                }
            }
        }
    }

    fn quarantine(&self, fault: BackendFault) {
        self.fault.store(encode_fault(fault), Ordering::Release);
        if self.state.swap(QUARANTINED, Ordering::AcqRel) != QUARANTINED {
            bump_generation(&self.generation);
        }
    }

    #[cfg(all(feature = "std", unix))]
    fn remember_process(&self) {
        self.process_id
            .store(std::process::id() as usize, Ordering::Release);
    }

    #[cfg(not(all(feature = "std", unix)))]
    const fn remember_process(&self) {
        let _ = self;
    }

    #[cfg(all(feature = "std", unix))]
    fn refresh_after_fork(&self) {
        let current = std::process::id() as usize;
        let recorded = self.process_id.load(Ordering::Acquire);
        if recorded != 0
            && recorded != current
            && self
                .state
                .compare_exchange(TESTING, NEVER_RUN, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            self.process_id.store(current, Ordering::Release);
            bump_generation(&self.generation);
        }
    }

    #[cfg(not(all(feature = "std", unix)))]
    const fn refresh_after_fork(&self) {
        let _ = self;
    }
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
fn bump_generation(generation: &AtomicUsize) {
    let mut current = generation.load(Ordering::Relaxed);
    while current != usize::MAX {
        match generation.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const fn decode_state(state: usize) -> BackendHealthState {
    match state {
        TESTING => BackendHealthState::Testing,
        HEALTHY => BackendHealthState::Healthy,
        QUARANTINED => BackendHealthState::Quarantined,
        _ => BackendHealthState::NeverRun,
    }
}

#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const fn encode_fault(fault: BackendFault) -> usize {
    match fault {
        BackendFault::SelfTestFailed => 1,
        BackendFault::OutputMismatch => 2,
        BackendFault::ImpossibleState => 3,
        BackendFault::ScalarRetryFailed => 4,
    }
}

#[cfg(any(test, all(feature = "simd", target_has_atomic = "ptr")))]
const fn decode_fault(fault: usize) -> Option<BackendFault> {
    match fault {
        1 => Some(BackendFault::SelfTestFailed),
        2 => Some(BackendFault::OutputMismatch),
        3 => Some(BackendFault::ImpossibleState),
        4 => Some(BackendFault::ScalarRetryFailed),
        _ => None,
    }
}

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
fn run_catching_panics(run: impl FnOnce() -> bool) -> bool {
    #[cfg(feature = "std")]
    {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or(false)
    }
    #[cfg(not(feature = "std"))]
    {
        run()
    }
}

macro_rules! health_cells {
    ($($name:ident),+ $(,)?) => {
        $(
            #[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
            static $name: HealthCell = HealthCell::new();
        )+
    };
}

health_cells!(
    ENCODE_AVX512,
    DECODE_AVX512,
    ENCODE_AVX2,
    DECODE_AVX2,
    ENCODE_SSSE3,
    DECODE_SSSE3,
    ENCODE_NEON,
    DECODE_NEON,
    ENCODE_WASM,
    DECODE_WASM,
);

#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
fn cell(operation: OperationKind, backend: Backend) -> Option<&'static HealthCell> {
    match (operation, backend) {
        (OperationKind::Encode, Backend::Avx512Vbmi) => Some(&ENCODE_AVX512),
        (OperationKind::StrictDecode, Backend::Avx512Vbmi) => Some(&DECODE_AVX512),
        (OperationKind::Encode, Backend::Avx2) => Some(&ENCODE_AVX2),
        (OperationKind::StrictDecode, Backend::Avx2) => Some(&DECODE_AVX2),
        (OperationKind::Encode, Backend::Ssse3Sse41) => Some(&ENCODE_SSSE3),
        (OperationKind::StrictDecode, Backend::Ssse3Sse41) => Some(&DECODE_SSSE3),
        (OperationKind::Encode, Backend::Neon) => Some(&ENCODE_NEON),
        (OperationKind::StrictDecode, Backend::Neon) => Some(&DECODE_NEON),
        (OperationKind::Encode, Backend::WasmSimd128) => Some(&ENCODE_WASM),
        (OperationKind::StrictDecode, Backend::WasmSimd128) => Some(&DECODE_WASM),
        _ => None,
    }
}

pub(crate) fn admit(operation: OperationKind, backend: Backend) -> bool {
    if backend == Backend::Scalar {
        return true;
    }
    #[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
    if kat::available(backend)
        && let Some(cell) = cell(operation, backend)
    {
        return cell.ensure(operation, backend);
    }
    let _ = operation;
    false
}

/// Admits a target-compatible backend after an unsafe deployment attestation.
///
/// The only caller is `StaticBackendToken::assume_supported`, whose public
/// unsafe contract owns the CPU and OS evidence for executing the direct KAT.
#[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
pub(crate) fn admit_deployment_attested(operation: OperationKind, backend: Backend) -> bool {
    cell(operation, backend).is_some_and(|cell| cell.ensure(operation, backend))
}

#[cfg(all(feature = "simd", not(target_has_atomic = "ptr")))]
pub(crate) const fn admit_deployment_attested(
    _operation: OperationKind,
    _backend: Backend,
) -> bool {
    false
}

#[cfg(feature = "checked-backend")]
pub(crate) fn quarantine(operation: OperationKind, backend: Backend, fault: BackendFault) {
    #[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
    if let Some(cell) = cell(operation, backend) {
        cell.quarantine(fault);
    }
    let _ = (operation, backend, fault);
}

#[cfg(feature = "checked-backend")]
pub(crate) fn direct_encode<A: crate::Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Option<usize> {
    kat::direct_encode::<A, PAD>(backend, input, output)
}

#[cfg(feature = "checked-backend")]
pub(crate) fn direct_decode<A: crate::Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Option<usize> {
    kat::direct_decode::<A, PAD>(backend, input, output)
}

pub(crate) fn snapshot(operation: OperationKind, backend: Backend) -> BackendHealthSnapshot {
    #[cfg(all(feature = "simd", target_has_atomic = "ptr"))]
    if let Some(cell) = cell(operation, backend) {
        return cell.snapshot(operation, backend);
    }
    BackendHealthSnapshot {
        operation,
        backend,
        state: if backend == Backend::Scalar {
            BackendHealthState::Healthy
        } else {
            BackendHealthState::NeverRun
        },
        generation: 1,
        fault: None,
    }
}

/// Runs KAT initialization for every accelerated backend available now.
#[must_use]
pub fn initialize_backends() -> BackendInitializationReport {
    let mut report = BackendInitializationReport::default();
    for backend in candidate_backends() {
        for operation in [OperationKind::Encode, OperationKind::StrictDecode] {
            let result = if !backend_available(*backend) {
                InitializationResult::Unavailable
            } else if admit(operation, *backend) {
                InitializationResult::Healthy
            } else if snapshot(operation, *backend).state == BackendHealthState::Quarantined {
                InitializationResult::Quarantined
            } else {
                InitializationResult::Unavailable
            };
            report.record(result);
        }
    }
    report
}

fn backend_available(backend: Backend) -> bool {
    #[cfg(feature = "simd")]
    {
        kat::available(backend)
    }
    #[cfg(not(feature = "simd"))]
    {
        let _ = backend;
        false
    }
}

const fn candidate_backends() -> &'static [Backend] {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    return &[Backend::Avx512Vbmi, Backend::Avx2, Backend::Ssse3Sse41];
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    return &[Backend::Neon];
    #[cfg(target_arch = "wasm32")]
    return &[Backend::WasmSimd128];
    #[allow(unreachable_code)]
    &[]
}

#[cfg(test)]
mod tests;
