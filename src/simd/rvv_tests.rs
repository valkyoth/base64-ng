use crate::runtime::{Backend, CandidateDetectionMode, SecurityPosture};
use crate::{Alphabet, Standard, UrlSafe, checked_encoded_len, scalar};
use core::sync::atomic::{AtomicU32, Ordering};

pub(super) static SIGNAL_ARMED: AtomicU32 = AtomicU32::new(0);
pub(super) static SIGNAL_DELIVERED: AtomicU32 = AtomicU32::new(0);

#[test]
fn runtime_distinguishes_exact_x60_admission_from_candidate_visibility() {
    assert!(super::rvv::candidate_available());
    let vlenb = super::rvv::vector_length_bytes();
    assert!(vlenb >= 16 && vlenb.is_power_of_two());
    eprintln!("RVV candidate VLEN={} bits", vlenb * 8);
    assert_eq!(super::detected_candidate(), super::Candidate::Rvv);

    let report = crate::runtime::backend_report();
    assert_eq!(report.candidate, Backend::Rvv);
    assert_eq!(
        report.candidate_detection_mode,
        CandidateDetectionMode::RuntimeCpuFeatures
    );
    if super::rvv::available() {
        assert_eq!(report.active, Backend::Rvv);
        assert_eq!(report.encode_backend.backend, Backend::Rvv);
        assert_eq!(report.strict_decode_backend.backend, Backend::Rvv);
        assert!(report.ordinary_acceleration_active);
        return;
    }
    assert_eq!(report.active, Backend::Scalar);
    assert_eq!(report.encode_backend.backend.as_str(), "scalar");
    assert_eq!(report.strict_decode_backend.backend.as_str(), "scalar");
    assert!(!report.ordinary_acceleration_active);
    assert_eq!(
        report.security_posture,
        SecurityPosture::SimdCandidateScalarActive
    );
}

#[test]
fn rvv_candidate_matches_scalar_for_profiles_blocks_and_tails() {
    assert_candidate_round_trips::<Standard, true>();
    assert_candidate_round_trips::<Standard, false>();
    assert_candidate_round_trips::<UrlSafe, true>();
    assert_candidate_round_trips::<UrlSafe, false>();
}

#[test]
fn rvv_candidate_rejects_before_writing_output() {
    let mut encoded = [b'A'; 64];
    encoded[47] = b'!';
    let mut output = [0x5a; 48];
    let before = output;

    assert!(super::rvv::decode_slice::<Standard, false>(&encoded, &mut output).is_err());
    assert_eq!(output, before);
}

#[test]
fn rvv_probe_fails_closed_across_kernel_and_vector_state_results() {
    use super::rvv::probe_allows_rvv;

    assert!(probe_allows_rvv(true, true, true, 2));
    assert!(!probe_allows_rvv(true, false, true, 2));
    assert!(!probe_allows_rvv(true, true, true, 0));
    assert!(probe_allows_rvv(false, false, true, -1));
    assert!(!probe_allows_rvv(false, false, false, -1));
    assert!(!probe_allows_rvv(false, false, true, 0));
}

#[test]
fn x60_admission_requires_every_identity_feature_and_thread_state_field() {
    use super::rvv::exact_x60_profile_allows_rvv;

    const GOOD: (bool, i64, u64, i64, u64, i64, u64, i64, u64, i32) = (
        true,
        0,
        0x710,
        1,
        0x8000_0000_5800_0001,
        2,
        0x1000_0000_4977_2200,
        4,
        1 << 2,
        2,
    );
    let allows = |values: (bool, i64, u64, i64, u64, i64, u64, i64, u64, i32)| {
        exact_x60_profile_allows_rvv(
            values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
            values.8, values.9,
        )
    };
    assert!(allows(GOOD));

    let mut cases = [GOOD; 10];
    cases[0].0 = false;
    cases[1].1 = -1;
    cases[2].2 ^= 1;
    cases[3].3 = -1;
    cases[4].4 ^= 1;
    cases[5].5 = -1;
    cases[6].6 ^= 1;
    cases[7].7 = -1;
    cases[8].8 = 0;
    cases[9].9 = 0;
    for rejected in cases {
        assert!(!allows(rejected));
    }
}

#[test]
fn exact_x60_profile_selects_public_rvv_only_at_measured_sizes() {
    if !super::rvv::available() {
        return;
    }
    assert_eq!(
        crate::encode_backend::active_encode_backend_for_input(191),
        crate::encode_backend::EncodeBackend::Scalar
    );
    assert_eq!(
        crate::decode_backend::active_decode_backend_for_input(191),
        crate::decode_backend::DecodeBackend::Scalar
    );
    assert_eq!(
        crate::encode_backend::active_encode_backend_for_input(192),
        crate::encode_backend::EncodeBackend::Rvv
    );
    assert_eq!(
        crate::decode_backend::active_decode_backend_for_input(192),
        crate::decode_backend::DecodeBackend::Rvv
    );
}

#[test]
fn unavailable_rvv_candidate_falls_back_to_scalar_without_assembly_entry() {
    let mut input = [0u8; 48];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(29).wrapping_add(7);
    }

    let mut expected_encoded = [0u8; 64];
    let mut fallback_encoded = [0u8; 64];
    let expected_written =
        scalar::encode_slice::<Standard, true>(&input, &mut expected_encoded).unwrap();
    let fallback_written = super::rvv::encode_slice_unavailable_for_test::<Standard, true>(
        &input,
        &mut fallback_encoded,
    )
    .unwrap();
    assert_eq!(fallback_written, expected_written);
    assert_eq!(fallback_encoded, expected_encoded);

    let mut expected_decoded = [0u8; 48];
    let mut fallback_decoded = [0u8; 48];
    let expected_decoded_len = scalar::decode_slice::<Standard, true>(
        &expected_encoded[..expected_written],
        &mut expected_decoded,
    )
    .unwrap();
    let fallback_decoded_len = super::rvv::decode_slice_unavailable_for_test::<Standard, true>(
        &fallback_encoded[..fallback_written],
        &mut fallback_decoded,
    )
    .unwrap();
    assert_eq!(fallback_decoded_len, expected_decoded_len);
    assert_eq!(fallback_decoded, expected_decoded);
}

#[test]
#[ignore = "requires native RVV hardware and a real Linux signal frame"]
fn rvv_state_survives_linux_signal_delivery() {
    const ITIMER_REAL: i32 = 0;
    const SIGALRM: i32 = 14;
    const SIG_ERR: usize = usize::MAX;
    #[repr(C)]
    struct TimeVal {
        seconds: i64,
        microseconds: i64,
    }
    #[repr(C)]
    struct IntervalTimer {
        interval: TimeVal,
        value: TimeVal,
    }
    unsafe extern "C" {
        fn signal(signal: i32, handler: usize) -> usize;
        fn setitimer(which: i32, value: *const IntervalTimer, old: *mut IntervalTimer) -> i32;
    }

    assert!(super::rvv::candidate_available());
    SIGNAL_ARMED.store(0, Ordering::SeqCst);
    SIGNAL_DELIVERED.store(0, Ordering::SeqCst);
    let timer = IntervalTimer {
        interval: TimeVal {
            seconds: 0,
            microseconds: 0,
        },
        value: TimeVal {
            seconds: 0,
            microseconds: 10_000,
        },
    };
    let disabled = IntervalTimer {
        interval: TimeVal {
            seconds: 0,
            microseconds: 0,
        },
        value: TimeVal {
            seconds: 0,
            microseconds: 0,
        },
    };
    // SAFETY: This test serially replaces SIGALRM, arms a one-shot timer,
    // invokes a syscall-free RVV leaf, disables the timer, and restores the
    // prior handler before inspecting the result.
    unsafe {
        let old = signal(SIGALRM, super::rvv::signal_clobber as *const () as usize);
        assert_ne!(old, SIG_ERR);
        assert_eq!(
            setitimer(ITIMER_REAL, &raw const timer, core::ptr::null_mut()),
            0
        );
        let mut observed = [0u8; 16];
        super::rvv::signal_context_round_trip(
            observed.as_mut_ptr(),
            SIGNAL_ARMED.as_ptr(),
            SIGNAL_DELIVERED.as_ptr(),
        );
        assert_eq!(
            setitimer(ITIMER_REAL, &raw const disabled, core::ptr::null_mut()),
            0
        );
        let restored = signal(SIGALRM, old);
        assert_ne!(restored, SIG_ERR);
        assert_eq!(SIGNAL_ARMED.load(Ordering::SeqCst), 0);
        assert_eq!(SIGNAL_DELIVERED.load(Ordering::SeqCst), 1);
        assert_eq!(observed, [0x5a; 16]);
    }
}

#[test]
fn rvv_candidate_survives_thread_context_switches() {
    let mut workers = std::vec::Vec::new();
    for worker in 0u8..8 {
        workers.push(std::thread::spawn(move || {
            let mut input = [0u8; 513];
            for (index, byte) in input.iter_mut().enumerate() {
                *byte = index.to_le_bytes()[0].wrapping_mul(73).wrapping_add(worker);
            }
            for _ in 0..256 {
                let mut encoded = [0u8; 684];
                let mut decoded = [0u8; 513];
                let written =
                    super::rvv::encode_slice::<Standard, true>(&input, &mut encoded).unwrap();
                let decoded_len =
                    super::rvv::decode_slice::<Standard, true>(&encoded[..written], &mut decoded)
                        .unwrap();
                assert_eq!(&decoded[..decoded_len], &input);
                std::thread::yield_now();
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
}

fn assert_candidate_round_trips<A: Alphabet, const PAD: bool>() {
    let mut input = [0u8; 513];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(73).wrapping_add(19);
    }

    for len in 0..=input.len() {
        let required = checked_encoded_len(len, PAD).unwrap();
        let mut expected_encoded = [0u8; 684];
        let mut candidate_encoded = [0u8; 684];
        let expected_written =
            scalar::encode_slice::<A, PAD>(&input[..len], &mut expected_encoded).unwrap();
        let candidate_written =
            super::rvv::encode_slice::<A, PAD>(&input[..len], &mut candidate_encoded).unwrap();
        assert_eq!(expected_written, required);
        assert_eq!(candidate_written, expected_written);
        assert_eq!(
            &candidate_encoded[..candidate_written],
            &expected_encoded[..expected_written]
        );

        let mut expected_decoded = [0u8; 513];
        let mut candidate_decoded = [0u8; 513];
        let expected_decoded_len = scalar::decode_slice::<A, PAD>(
            &expected_encoded[..expected_written],
            &mut expected_decoded,
        )
        .unwrap();
        let candidate_decoded_len = super::rvv::decode_slice::<A, PAD>(
            &candidate_encoded[..candidate_written],
            &mut candidate_decoded,
        )
        .unwrap();
        assert_eq!(candidate_decoded_len, expected_decoded_len);
        assert_eq!(&candidate_decoded[..candidate_decoded_len], &input[..len]);
    }
}
