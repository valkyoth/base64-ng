use super::{
    Backend, BackendPolicy, BackendReport, CandidateDetectionMode, CtGatePosture,
    OperationBackendReport, OperationKind, OperationSecurityPosture, SecurityPosture,
    WasmArtifactPosture, WasmRuntimePosture, WipePosture, backend_report,
};
use crate::{Standard, decode_backend, encode_backend};

#[test]
fn per_operation_reports_match_qualifying_dispatch_counters() {
    initialize_runtime_backend_health();
    // Encode can report AVX-512 on capable hosts. Commit 34 keeps automatic
    // strict decode on AVX2 even when AVX-512 is the detected candidate. Keep
    // this input large enough to exercise the final operation-specific tiers.
    let input = [0x5au8; 12 * 1024];
    let mut encoded = [0u8; 16 * 1024];
    let encoded_len = encode_backend::encode_slice::<Standard, true>(&input, &mut encoded).unwrap();
    let mut decoded = [0u8; 12 * 1024];
    let decoded_len =
        decode_backend::decode_slice::<Standard, true>(&encoded[..encoded_len], &mut decoded)
            .unwrap();
    assert_eq!(decoded_len, input.len());
    assert_eq!(decoded, input);

    let report = backend_report();
    assert_eq!(
        report.encode_backend.backend.as_str(),
        encode_id(encode_backend::last_test_execution())
    );
    assert_eq!(
        report.strict_decode_backend.backend.as_str(),
        decode_id(decode_backend::last_test_execution())
    );
    assert_eq!(
        report.secret_decode_backend.backend.as_str(),
        "scalar-constant-time-oriented"
    );
    assert_eq!(
        report.secret_decode_backend.security_posture,
        OperationSecurityPosture::ScalarConstantTimeOriented
    );
}

#[test]
fn secret_decode_posture_is_fixed_outside_simd_dispatch() {
    let secret = backend_report().secret_decode_backend;
    assert_eq!(secret.operation, OperationKind::SecretDecode);
    assert_eq!(secret.backend.as_str(), "scalar-constant-time-oriented");
    assert_eq!(
        secret.security_posture,
        OperationSecurityPosture::ScalarConstantTimeOriented
    );
    assert_eq!(
        secret.health_posture,
        super::BackendHealthPosture::SecretPolicyFixed
    );
    assert_eq!(secret.backend_fault, None);
}

#[test]
fn scalar_execution_policy_rejects_transient_scalar_fallbacks() {
    let mut report = scalar_report(CtGatePosture::HardwareSpeculationBarrier);
    report.simd_feature_enabled = true;
    report.candidate = Backend::Avx2;

    for posture in [
        super::BackendHealthPosture::ScalarFallback,
        super::BackendHealthPosture::NeverRun,
        super::BackendHealthPosture::Testing,
        super::BackendHealthPosture::Healthy,
        super::BackendHealthPosture::Quarantined,
        super::BackendHealthPosture::SynchronizationUnavailable,
    ] {
        report.encode_backend.health_posture = posture;
        report.strict_decode_backend.health_posture = posture;
        assert!(!report.satisfies(BackendPolicy::ScalarExecutionOnly));
    }

    report.encode_backend.health_posture = super::BackendHealthPosture::ScalarFixed;
    report.strict_decode_backend.health_posture = super::BackendHealthPosture::ScalarFixed;
    assert!(report.satisfies(BackendPolicy::ScalarExecutionOnly));
    report.ordinary_acceleration_active = true;
    assert!(!report.satisfies(BackendPolicy::ScalarExecutionOnly));
    report.ordinary_acceleration_active = false;
    report.encode_backend.security_posture = OperationSecurityPosture::OrdinaryAccelerated;
    assert!(!report.satisfies(BackendPolicy::ScalarExecutionOnly));
    report.encode_backend.security_posture = OperationSecurityPosture::OrdinaryScalar;
    report.strict_decode_backend.security_posture = OperationSecurityPosture::OrdinaryAccelerated;
    assert!(!report.satisfies(BackendPolicy::ScalarExecutionOnly));
}

#[test]
fn scalar_policy_checks_every_available_tier_for_each_operation() {
    use crate::BackendHealthState::{Healthy, NeverRun, Quarantined, Testing};
    let backends = [Backend::Avx512Vbmi, Backend::Avx2, Backend::Ssse3Sse41];
    let states = [NeverRun, Testing, Healthy, Quarantined];
    for operation in [OperationKind::Encode, OperationKind::StrictDecode] {
        for availability in 0u8..8 {
            for first in states {
                for second in states {
                    for third in states {
                        let health = [first, second, third];
                        let available = |backend| {
                            let index = backends.iter().position(|b| *b == backend).unwrap();
                            availability & (1 << index) != 0
                        };
                        let terminal = crate::v2::backend_health::terminally_scalar(
                            &backends,
                            available,
                            |backend| {
                                assert!(available(backend));
                                health[backends.iter().position(|b| *b == backend).unwrap()]
                            },
                        );
                        let expected = (0..3)
                            .all(|i| availability & (1 << i) == 0 || health[i] == Quarantined);
                        assert_eq!(terminal, expected);
                        let mut report = scalar_report(CtGatePosture::HardwareSpeculationBarrier);
                        report.candidate = Backend::Avx512Vbmi;
                        report.simd_feature_enabled = true;
                        let selected = OperationBackendReport::from_health(
                            crate::v2::backend_health::snapshot(operation, Backend::Scalar),
                            terminal,
                        );
                        match operation {
                            OperationKind::Encode => report.encode_backend = selected,
                            OperationKind::StrictDecode => report.strict_decode_backend = selected,
                            OperationKind::SecretDecode => unreachable!(),
                        }
                        assert_eq!(
                            report.satisfies(BackendPolicy::ScalarExecutionOnly),
                            expected
                        );
                        assert_eq!(selected.backend.as_str(), "scalar");
                        assert_eq!(selected.backend_fault, None);
                        assert_eq!(selected.health_generation, 1);
                        assert_eq!(
                            selected.snapshot().health_posture,
                            if expected {
                                "scalar-fixed"
                            } else {
                                "scalar-fallback"
                            }
                        );
                        // Policy evaluation uses captured evidence, never a fresh global read.
                        assert_eq!(
                            report.snapshot().encode_backend,
                            report.encode_backend.snapshot()
                        );
                        assert_eq!(
                            report.snapshot().strict_decode_backend,
                            report.strict_decode_backend.snapshot()
                        );
                    }
                }
            }
        }
    }
    assert!(crate::v2::backend_health::terminally_scalar(
        &[],
        |_| panic!("no backend"),
        |_| panic!("no health")
    ));
}

#[test]
fn operation_health_is_attributed_to_the_named_backend() {
    use crate::{BackendFault, BackendHealthSnapshot, BackendHealthState};
    for operation in [OperationKind::Encode, OperationKind::StrictDecode] {
        let upper = BackendHealthSnapshot {
            operation,
            backend: Backend::Avx512Vbmi,
            state: BackendHealthState::Quarantined,
            generation: 9,
            fault: Some(BackendFault::SelfTestFailed),
        };
        let selected = BackendHealthSnapshot {
            operation,
            backend: Backend::Avx2,
            state: BackendHealthState::Healthy,
            generation: 3,
            fault: None,
        };
        let upper_report = OperationBackendReport::from_health(upper, false);
        let selected_report = OperationBackendReport::from_health(selected, false);
        assert_eq!(upper_report.backend.as_str(), "avx512-vbmi");
        assert_eq!(upper_report.health_generation, 9);
        assert_eq!(upper_report.backend_fault, upper.fault);
        assert_eq!(selected_report.backend.as_str(), "avx2");
        assert_eq!(
            selected_report.health_posture,
            super::BackendHealthPosture::Healthy
        );
        assert_eq!(selected_report.health_generation, 3);
        assert_eq!(selected_report.backend_fault, None);
    }
}

#[test]
fn live_selected_reports_use_their_own_health_snapshot() {
    initialize_runtime_backend_health();
    for operation in [OperationKind::Encode, OperationKind::StrictDecode] {
        for backend in [
            Backend::Scalar,
            Backend::Avx512Vbmi,
            Backend::Avx2,
            Backend::Ssse3Sse41,
            Backend::Neon,
            Backend::WasmSimd128,
            Backend::Rvv,
        ] {
            let health = crate::v2::backend_health::snapshot(operation, backend);
            let report = OperationBackendReport::ordinary(operation, backend);
            assert_eq!(report.operation, operation);
            assert_eq!(report.backend.as_str(), backend.as_str());
            assert_eq!(report.health_generation, health.generation);
            assert_eq!(report.backend_fault, health.fault);
            if backend == Backend::Scalar {
                let terminal = crate::v2::backend_health::operation_is_terminally_scalar(operation);
                assert_eq!(
                    report.health_posture,
                    if terminal {
                        super::BackendHealthPosture::ScalarFixed
                    } else {
                        super::BackendHealthPosture::ScalarFallback
                    }
                );
            }
            if backend != Backend::Scalar && cfg!(target_has_atomic = "ptr") {
                assert_eq!(report.health_posture.as_str(), health.state.as_str());
            }
        }
    }
}

fn initialize_runtime_backend_health() {
    let _ = crate::initialize_backends();
    #[cfg(all(feature = "std", feature = "simd"))]
    {
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(10) {
            // Candidate health can hide a lower tier still running its KAT:
            // an AVX-512 candidate does not imply AVX2 decode is ready.
            if runtime_backend_health_settled(|operation, backend| {
                crate::v2::backend_health::snapshot(operation, backend).state
            }) {
                return;
            }
            std::thread::yield_now();
        }
        panic!("backend health initialization did not leave Testing");
    }
}

#[cfg(all(feature = "std", feature = "simd"))]
fn runtime_backend_health_settled(
    mut state: impl FnMut(OperationKind, Backend) -> crate::BackendHealthState,
) -> bool {
    for backend in [
        Backend::Avx512Vbmi,
        Backend::Avx2,
        Backend::Ssse3Sse41,
        Backend::Neon,
        Backend::WasmSimd128,
        Backend::Rvv,
    ] {
        for operation in [OperationKind::Encode, OperationKind::StrictDecode] {
            if state(operation, backend) == crate::BackendHealthState::Testing {
                return false;
            }
        }
    }
    true
}

#[test]
#[cfg(all(feature = "std", feature = "simd"))]
fn initialization_waits_for_testing_tiers_hidden_by_candidate_reports() {
    use crate::BackendHealthState::{Healthy, NeverRun, Quarantined, Testing};

    let mut pairs = std::vec::Vec::new();
    assert!(runtime_backend_health_settled(|operation, backend| {
        pairs.push((operation, backend));
        Healthy
    }));
    assert_eq!(pairs.len(), 12);
    assert!(pairs.contains(&(OperationKind::StrictDecode, Backend::Avx2)));
    for pending in &pairs {
        assert!(!runtime_backend_health_settled(|operation, backend| {
            if (operation, backend) == *pending {
                Testing
            } else {
                Healthy
            }
        }));
    }
    // Unsupported and quarantined backends cannot become healthy later in this
    // process; only an in-progress self-test requires waiting after initialization.
    for settled in [NeverRun, Healthy, Quarantined] {
        assert!(runtime_backend_health_settled(|_, _| settled));
    }
}

#[test]
fn wasm_posture_never_claims_native_runtime_detection() {
    let report = backend_report();
    if report.wasm_artifact_posture == WasmArtifactPosture::Simd128Artifact {
        assert_eq!(
            report.wasm_artifact_posture,
            WasmArtifactPosture::Simd128Artifact
        );
        assert_eq!(
            report.candidate_detection_mode,
            CandidateDetectionMode::CompileTimeTargetFeatures
        );
        assert_eq!(
            report.wasm_runtime_posture,
            WasmRuntimePosture::HostRuntimeUnidentified
        );
    } else if cfg!(target_arch = "wasm32") {
        assert_eq!(
            report.wasm_artifact_posture,
            WasmArtifactPosture::ScalarArtifact
        );
    } else {
        assert_eq!(report.wasm_artifact_posture, WasmArtifactPosture::NotWasm);
        assert_eq!(report.wasm_runtime_posture, WasmRuntimePosture::NotWasm);
    }
}

#[test]
fn high_assurance_policy_rejects_weak_result_gates() {
    let ordering = scalar_report(CtGatePosture::OrderingFence);
    let unattested = scalar_report(CtGatePosture::HardwareSpeculationBarrierUnattested);
    let build_asserted = scalar_report(CtGatePosture::HardwareSpeculationBarrierBuildAsserted);
    assert!(!ordering.satisfies(BackendPolicy::HighAssuranceScalarOnly));
    assert!(!unattested.satisfies(BackendPolicy::HighAssuranceScalarOnly));
    assert!(build_asserted.satisfies(BackendPolicy::HighAssuranceScalarOnly));
}

fn scalar_report(ct_gate_posture: CtGatePosture) -> BackendReport {
    BackendReport {
        active: Backend::Scalar,
        accelerated_backend_active: false,
        security_posture: SecurityPosture::ScalarOnly,
        encode_backend: OperationBackendReport::from_health(
            crate::v2::backend_health::snapshot(OperationKind::Encode, Backend::Scalar),
            true,
        ),
        strict_decode_backend: OperationBackendReport::from_health(
            crate::v2::backend_health::snapshot(OperationKind::StrictDecode, Backend::Scalar),
            true,
        ),
        secret_decode_backend: OperationBackendReport::secret_decode(),
        candidate: Backend::Scalar,
        candidate_detection_mode: CandidateDetectionMode::SimdFeatureDisabled,
        simd_feature_enabled: false,
        ordinary_acceleration_active: false,
        unsafe_boundary_enforced: true,
        wasm_artifact_posture: WasmArtifactPosture::NotWasm,
        wasm_runtime_posture: WasmRuntimePosture::NotWasm,
        wipe_posture: WipePosture::HardwareFence,
        ct_gate_posture,
    }
}

fn encode_id(backend: encode_backend::EncodeBackend) -> &'static str {
    match backend {
        encode_backend::EncodeBackend::Scalar => "scalar",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        encode_backend::EncodeBackend::Avx512Vbmi => "avx512-vbmi",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        encode_backend::EncodeBackend::Avx2 => "avx2",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        encode_backend::EncodeBackend::Ssse3Sse41 => "ssse3-sse4.1",
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        encode_backend::EncodeBackend::Neon => "neon",
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        encode_backend::EncodeBackend::WasmSimd128 => "wasm-simd128",
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        encode_backend::EncodeBackend::Rvv => "rvv",
    }
}

fn decode_id(backend: decode_backend::DecodeBackend) -> &'static str {
    match backend {
        decode_backend::DecodeBackend::Scalar => "scalar",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        decode_backend::DecodeBackend::Avx512Vbmi => "avx512-vbmi",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        decode_backend::DecodeBackend::Avx2 => "avx2",
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        decode_backend::DecodeBackend::Ssse3Sse41 => "ssse3-sse4.1",
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        decode_backend::DecodeBackend::Neon => "neon",
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        decode_backend::DecodeBackend::WasmSimd128 => "wasm-simd128",
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        decode_backend::DecodeBackend::Rvv => "rvv",
    }
}
