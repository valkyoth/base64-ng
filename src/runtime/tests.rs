use super::{
    Backend, BackendPolicy, BackendReport, CandidateDetectionMode, CtGatePosture,
    OperationBackendReport, OperationKind, OperationSecurityPosture, SecurityPosture,
    WasmArtifactPosture, WasmRuntimePosture, WipePosture, backend_report,
};
use crate::{Standard, decode_backend, encode_backend};

#[test]
fn per_operation_reports_match_qualifying_dispatch_counters() {
    initialize_runtime_backend_health();
    // AVX-512 encode is reported as the strongest healthy backend on capable
    // hosts, but automatic dispatch intentionally prefers AVX2 below the
    // measured 192-byte crossover. Keep this report-correlation input large
    // enough to qualify for every reported backend.
    let input = [0x5au8; 192];
    let mut encoded = [0u8; 256];
    let encoded_len = encode_backend::encode_slice::<Standard, true>(&input, &mut encoded).unwrap();
    let mut decoded = [0u8; 192];
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
        super::BackendHealthPosture::NeverRun,
        super::BackendHealthPosture::Testing,
        super::BackendHealthPosture::Healthy,
    ] {
        report.encode_backend.health_posture = posture;
        report.strict_decode_backend.health_posture = posture;
        assert!(!report.satisfies(BackendPolicy::ScalarExecutionOnly));
    }

    for posture in [
        super::BackendHealthPosture::ScalarFixed,
        super::BackendHealthPosture::Quarantined,
        super::BackendHealthPosture::SynchronizationUnavailable,
    ] {
        report.encode_backend.health_posture = posture;
        report.strict_decode_backend.health_posture = posture;
        assert!(report.satisfies(BackendPolicy::ScalarExecutionOnly));
    }
}

fn initialize_runtime_backend_health() {
    let _ = crate::initialize_backends();
    #[cfg(all(feature = "std", feature = "simd"))]
    for _ in 0..10_000 {
        let report = backend_report();
        if report.encode_backend.health_posture != super::BackendHealthPosture::Testing
            && report.strict_decode_backend.health_posture != super::BackendHealthPosture::Testing
        {
            return;
        }
        std::thread::yield_now();
    }
    #[cfg(all(feature = "std", feature = "simd"))]
    panic!("backend health initialization did not leave Testing");
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
        encode_backend: OperationBackendReport::ordinary(
            OperationKind::Encode,
            Backend::Scalar,
            Backend::Scalar,
        ),
        strict_decode_backend: OperationBackendReport::ordinary(
            OperationKind::StrictDecode,
            Backend::Scalar,
            Backend::Scalar,
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
    }
}
