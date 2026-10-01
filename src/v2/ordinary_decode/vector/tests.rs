use super::*;
use crate::{DecodeValidation, OneShotError, STRICT_STANDARD_PADDED as CODEC};
use core::cell::Cell;

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum Fault {
    #[default]
    None,
    Reject,
    #[cfg(feature = "checked-backend")]
    Accept,
    WriteReject,
    #[cfg(feature = "checked-backend")]
    WriteCorrupt,
    Unavailable,
}

#[derive(Clone, Copy, Default)]
struct State {
    fault: Fault,
    validation: usize,
    writes: usize,
    backend: Option<Backend>,
    quarantined: Option<(Backend, BackendFault)>,
}
std::thread_local! { static STATE: Cell<State> = const { Cell::new(State {
    fault: Fault::None, validation: 0, writes: 0, backend: None, quarantined: None,
}) }; }

fn observe(update: impl FnOnce(&mut State)) -> State {
    STATE.with(|state| {
        let mut next = state.get();
        update(&mut next);
        state.set(next);
        next
    })
}

pub(super) fn validate(backend: Backend) -> Option<bool> {
    let state = observe(|state| {
        state.validation += 1;
        state.backend = Some(backend);
    });
    match state.fault {
        Fault::Reject => Some(false),
        #[cfg(feature = "checked-backend")]
        Fault::Accept => Some(true),
        _ => None,
    }
}

pub(super) fn decode(backend: Backend, output: &mut [u8]) -> Option<bool> {
    let state = observe(|state| {
        state.writes += 1;
        state.backend = Some(backend);
    });
    match state.fault {
        Fault::WriteReject => {
            output.fill(0xff);
            Some(false)
        }
        #[cfg(feature = "checked-backend")]
        Fault::WriteCorrupt => {
            output.fill(0xff);
            Some(true)
        }
        _ => None,
    }
}

pub(super) fn unavailable() -> bool {
    STATE.with(|state| state.get().fault == Fault::Unavailable)
}

pub(super) fn quarantine(backend: Backend, fault: BackendFault) -> bool {
    if STATE.with(|state| state.get().fault == Fault::None) {
        return false;
    }
    observe(|state| state.quarantined = Some((backend, fault)));
    true
}

fn inject(fault: Fault, action: impl FnOnce()) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            STATE.with(|s| s.set(State::default()));
        }
    }
    STATE.with(|state| {
        state.set(State {
            fault,
            ..State::default()
        });
    });
    let _reset = Reset;
    action();
}

fn assert_quarantined(fault: BackendFault) {
    let state = STATE.with(Cell::get);
    let backend = state
        .backend
        .expect("the actual invocation must use a backend");
    assert!(matches!(
        backend,
        Backend::Avx2 | Backend::Ssse3Sse41 | Backend::Neon | Backend::WasmSimd128 | Backend::Rvv
    ));
    assert_eq!(state.quarantined, Some((backend, fault)));
}

// Production admission is intentionally nonblocking while another thread runs
// KATs. Tests wait for that finite initialization, rather than silently skipping
// fault coverage on a capable host during parallel startup.
fn ready_backend(len: usize) -> Option<Backend> {
    if width(Backend::Avx2).is_none()
        && width(Backend::Ssse3Sse41).is_none()
        && width(Backend::Neon).is_none()
        && width(Backend::WasmSimd128).is_none()
        && width(Backend::Rvv).is_none()
    {
        return None;
    }
    let start = std::time::Instant::now();
    loop {
        if let Some(backend) = select(len) {
            return Some(backend);
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "available backend did not become healthy"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn small_inputs_do_not_initialize_or_execute_the_vector_route() {
    for len in [0, 4, 16, 32, 64, 256, 508, 511] {
        assert_eq!(select(len), None);
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    for len in [512, 1024, 1368, 2048, 4095] {
        assert_eq!(select(len), None);
    }
    inject(Fault::Reject, || {
        let mut empty_output = [0xa5; 8];
        assert_eq!(CODEC.decode_into(b"", &mut empty_output), Ok(0));
        assert_eq!(empty_output, [0xa5; 8]);
        let mut output = [0xa5; 381];
        assert_eq!(CODEC.decode_into(&[b'A'; 508], &mut output), Ok(381));
        assert_eq!(STATE.with(Cell::get).validation, 0);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}

#[test]
fn public_validation_and_writing_reach_health_gated_simd_not_avx512() {
    let Some(backend) = ready_backend(4096) else {
        return;
    };
    assert!(matches!(
        backend,
        Backend::Avx2 | Backend::Ssse3Sse41 | Backend::Neon | Backend::WasmSimd128 | Backend::Rvv
    ));
    #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
    assert_eq!(backend, Backend::Neon);
    #[cfg(target_arch = "wasm32")]
    assert_eq!(backend, Backend::WasmSimd128);
    let input = [b'A'; 4096];
    let mut output = [0xff; 3075];
    inject(Fault::None, || {
        assert_eq!(CODEC.validate(&input), Ok(()));
        assert_eq!(CODEC.decoded_len(&input), Ok(3072));
        assert_eq!(CODEC.decode_into(&input, &mut output), Ok(3072));
        let state = STATE.with(Cell::get);
        assert_eq!(state.validation, 3);
        assert!(state.writes > 0);
        assert!(matches!(
            state.backend,
            Some(
                Backend::Avx2
                    | Backend::Ssse3Sse41
                    | Backend::Neon
                    | Backend::WasmSimd128
                    | Backend::Rvv
            )
        ));
        assert_eq!(output[..3072], [0; 3072]);
        assert_eq!(output[3072..], [0xff; 3]);
        assert_eq!(crate::STANDARD.decode_slice(&input, &mut output), Ok(3072));
        assert_eq!(STATE.with(Cell::get).validation, 4);
        let before = STATE.with(Cell::get);
        assert_eq!(
            CODEC.decode_into_with_validation(
                &input,
                &mut output,
                DecodeValidation::ScalarReference
            ),
            Ok(3072)
        );
        assert_eq!(STATE.with(Cell::get).validation, before.validation);
        assert!(STATE.with(Cell::get).writes > before.writes);
    });
}

#[test]
fn false_rejection_quarantines_before_capacity_or_mutation() {
    if ready_backend(4096).is_none() {
        return;
    }
    for capacity in [0, 3071, 3072, 3084] {
        inject(Fault::Reject, || {
            let mut output = [0xa5; 3084];
            assert_eq!(
                CODEC.decode_into(&[b'A'; 4096], &mut output[..capacity]),
                Err(OneShotError::Backend(BackendFault::ImpossibleState))
            );
            assert_eq!(output, [0xa5; 3084]);
            assert_quarantined(BackendFault::ImpossibleState);
        });
    }
}

#[test]
fn rejected_kernel_partial_stores_are_overwritten_and_backend_quarantined() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::WriteReject, || {
        let mut output = [0xa5; 3080];
        assert_eq!(CODEC.decode_into(&[b'A'; 4096], &mut output), Ok(3072));
        assert_eq!(output[..3072], [0; 3072]);
        assert_eq!(output[3072..], [0xa5; 8]);
        assert_quarantined(BackendFault::OutputMismatch);
    });
}

#[test]
fn backend_unavailable_after_preflight_uses_scalar_writer() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::Unavailable, || {
        let mut output = [0xa5; 3072];
        assert_eq!(CODEC.decode_into(&[b'A'; 4096], &mut output), Ok(3072));
        assert_eq!(output, [0; 3072]);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}

#[cfg(feature = "checked-backend")]
#[test]
fn checked_validation_and_output_faults_are_detected() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::Accept, || {
        let mut input = [b'A'; 4096];
        input[0] = b'!';
        let mut output = [0xa5; 3072];
        assert_eq!(
            CODEC.decode_into(&input, &mut output),
            Err(OneShotError::Backend(BackendFault::ImpossibleState))
        );
        assert_eq!(output, [0xa5; 3072]);
        assert_quarantined(BackendFault::ImpossibleState);
    });
    inject(Fault::WriteCorrupt, || {
        let mut output = [0xa5; 3072];
        assert_eq!(CODEC.decode_into(&[b'A'; 4096], &mut output), Ok(3072));
        assert_eq!(output, [0; 3072]);
        assert_quarantined(BackendFault::OutputMismatch);
    });
}
