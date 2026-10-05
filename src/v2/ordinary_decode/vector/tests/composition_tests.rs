use super::*;
use crate::{DecodeFallback, DecodeValidator};

#[test]
fn composition_reports_actual_validation_writer_and_recovery() {
    let Some(backend) = ready_backend(4096) else {
        return;
    };
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for fault in [Fault::None, Fault::Unavailable, Fault::WriteReject] {
            inject(fault, || {
                let mut output = [0xa5; 3080];
                let (len, report) = CODEC
                    .decode_into_with_report(&[b'A'; 4096], &mut output, policy)
                    .unwrap();
                assert_eq!(len, 3072);
                assert_eq!(output[..3072], [0; 3072]);
                assert_eq!(output[3072..], [0xa5; 8]);
                assert_eq!(report.requested_validation(), policy);
                assert_eq!(report.selected_backend(), Some(backend));
                assert_eq!(
                    report.validator(),
                    if policy == DecodeValidation::Auto {
                        DecodeValidator::Vector(backend)
                    } else {
                        DecodeValidator::ScalarReference
                    }
                );
                assert_eq!(
                    report.checked_validation(),
                    cfg!(feature = "checked-backend") && policy == DecodeValidation::Auto
                );
                assert_eq!(
                    report.checked_output(),
                    cfg!(feature = "checked-backend") && fault != Fault::Unavailable
                );
                assert_eq!(
                    report.output_backend(),
                    if fault == Fault::None {
                        backend
                    } else {
                        Backend::Scalar
                    }
                );
                assert_eq!(
                    report.fallback(),
                    match fault {
                        Fault::None => DecodeFallback::None,
                        Fault::Unavailable => DecodeFallback::BackendUnavailable,
                        _ => DecodeFallback::BackendRejected,
                    }
                );
                if fault == Fault::WriteReject {
                    assert_quarantined(BackendFault::OutputMismatch);
                }
            });
        }
    }
}

#[test]
fn composition_false_rejection_returns_no_report_or_mutation() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::Reject, || {
        let mut output = [0xa5; 3080];
        assert_eq!(
            CODEC.decode_into_with_report(&[b'A'; 4096], &mut output, DecodeValidation::Auto),
            Err(OneShotError::Backend(BackendFault::ImpossibleState))
        );
        assert_eq!(output, [0xa5; 3080]);
        assert_eq!(STATE.with(Cell::get).writes, 0);
        assert_quarantined(BackendFault::ImpossibleState);
    });
}

#[cfg(feature = "checked-backend")]
#[test]
fn composition_checked_faults_cannot_be_disabled_by_policy() {
    if ready_backend(4096).is_none() {
        return;
    }
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        inject(Fault::Accept, || {
            let mut input = [b'A'; 4096];
            input[1024] = b'!';
            let mut output = [0xa5; 3072];
            let error = CODEC
                .decode_into_with_report(&input, &mut output, policy)
                .unwrap_err();
            if policy == DecodeValidation::Auto {
                assert_eq!(error, OneShotError::Backend(BackendFault::ImpossibleState));
                assert_quarantined(BackendFault::ImpossibleState);
            } else {
                assert!(matches!(error, OneShotError::Input(_)));
                assert_eq!(STATE.with(Cell::get).validation, 0);
            }
            assert_eq!(output, [0xa5; 3072]);
            assert_eq!(STATE.with(Cell::get).writes, 0);
        });
        inject(Fault::WriteCorrupt, || {
            let mut output = [0xa5; 3072];
            let (_, report) = CODEC
                .decode_into_with_report(&[b'A'; 4096], &mut output, policy)
                .unwrap();
            assert_eq!(output, [0; 3072]);
            assert!(report.checked_output());
            assert_eq!(report.output_backend(), Backend::Scalar);
            assert_eq!(report.fallback(), DecodeFallback::BackendRejected);
            assert_quarantined(BackendFault::OutputMismatch);
        });
    }
}

// Global quarantine is irreversible. Keep real health/generation mutations in
// a fresh process rather than weakening production latches for parallel tests.
#[cfg(all(
    feature = "simd",
    not(miri),
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[test]
fn composition_quarantine_between_validation_and_write_invalidates_tokens() {
    const CHILD: &str = "BASE64_NG_COMPOSITION_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "v2::ordinary_decode::vector::tests::composition_tests::composition_quarantine_between_validation_and_write_invalidates_tokens", "--nocapture"])
            .env(CHILD, "1").status().unwrap();
        assert!(result.success());
        return;
    }
    let Some(backend) = ready_backend(4096) else {
        return;
    };
    for available in [Backend::Avx2, Backend::Ssse3Sse41] {
        if width(available).is_some() {
            assert_eq!(
                crate::v2::backend_health::snapshot(OperationKind::StrictDecode, available).state,
                crate::BackendHealthState::Healthy
            );
        }
    }
    let input = [b'A'; 4096];
    let borrowed = crate::Base64Ref::parse(CODEC, &input).unwrap();
    let token = crate::StaticBackendToken::admitted_for_test(backend);
    let mut report = crate::DecodeReport::new(DecodeValidation::Auto);
    let proof =
        super::super::super::prepare(CODEC.settings(), &input, DecodeValidation::Auto).unwrap();
    let before = crate::v2::backend_health::snapshot(OperationKind::StrictDecode, backend);
    crate::v2::backend_health::quarantine(
        OperationKind::StrictDecode,
        backend,
        BackendFault::SelfTestFailed,
    );
    let after = crate::v2::backend_health::snapshot(OperationKind::StrictDecode, backend);
    assert!(after.generation > before.generation);
    assert_eq!(after.state, crate::BackendHealthState::Quarantined);
    let mut output = [0xa5; 3080];
    assert_eq!(
        super::super::super::write_observed(proof, &mut output, &mut report),
        Ok(3072)
    );
    assert_eq!(output[..3072], [0; 3072]);
    assert_eq!(output[3072..], [0xa5; 8]);
    assert_eq!(report.output_backend(), Backend::Scalar);
    assert_eq!(report.fallback(), DecodeFallback::BackendUnavailable);
    assert!(!crate::v2::backend_health::admit(
        OperationKind::StrictDecode,
        backend
    ));
    let reference_before = crate::decode_validation::observation::calls();
    output.fill(0xa5);
    assert_eq!(borrowed.decode_into(&mut output), Ok(3072));
    assert!(crate::decode_validation::observation::calls() > reference_before);
    assert_eq!(output[..3072], [0; 3072]);
    assert_eq!(output[3072..], [0xa5; 8]);
    if let Some(token) = token {
        assert!(!token.is_valid());
        let (_, report) = token
            .decode_standard_with_report::<true>(&input, &mut output, DecodeValidation::Auto)
            .unwrap();
        assert_eq!(report.selected_backend(), None);
        assert_eq!(report.output_backend(), Backend::Scalar);
    }
}

#[cfg(feature = "simd")]
#[test]
fn composition_static_calls_use_the_same_checked_recovery_boundary() {
    let Some(backend) = ready_backend(4096) else {
        return;
    };
    let Some(token) = crate::StaticBackendToken::admitted_for_test(backend) else {
        return;
    };
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let faults = [
            Fault::WriteReject,
            #[cfg(feature = "checked-backend")]
            Fault::WriteCorrupt,
        ];
        for fault in faults {
            inject(fault, || {
                let mut output = [0xa5; 3080];
                let (_, report) = token
                    .decode_standard_with_report::<true>(&[b'A'; 4096], &mut output, policy)
                    .unwrap();
                assert_eq!(output[..3072], [0; 3072]);
                assert_eq!(output[3072..], [0xa5; 8]);
                assert_eq!(report.selected_backend(), Some(backend));
                assert_eq!(report.output_backend(), Backend::Scalar);
                assert_eq!(report.fallback(), DecodeFallback::BackendRejected);
                assert_eq!(report.checked_output(), cfg!(feature = "checked-backend"));
                assert_quarantined(BackendFault::OutputMismatch);
            });
        }
    }
}
