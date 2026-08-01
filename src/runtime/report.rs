use super::{
    Backend, BackendPolicy, CandidateDetectionMode, CtGatePosture, MemoryLockPosture,
    OperationBackendReport, OperationKind, OperationSecurityPosture, SecurityPosture,
    WasmArtifactPosture, WasmRuntimePosture, WipePosture,
    operation::{wasm_artifact_posture, wasm_runtime_posture},
};

/// Runtime backend policy failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendPolicyError {
    /// Policy that was requested.
    pub policy: BackendPolicy,
    /// Backend report observed when the policy failed.
    pub report: BackendReport,
}

impl core::fmt::Display for BackendPolicyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "runtime backend policy `{}` was not satisfied ({})",
            self.policy, self.report,
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BackendPolicyError {}

/// Backend report for the current build and target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct BackendReport {
    /// Compatibility alias for the ordinary encode backend.
    ///
    /// New code should use [`Self::encode_backend`] and inspect the separate
    /// strict- and secret-decode reports when those operations matter.
    pub active: Backend,
    /// Compatibility alias reporting whether ordinary encode is accelerated.
    ///
    /// This does not describe strict decode. New code should inspect the
    /// operation-specific reports.
    pub accelerated_backend_active: bool,
    /// Compatibility posture for the ordinary encode backend and candidate.
    ///
    /// New code should use the operation-specific security postures.
    pub security_posture: SecurityPosture,
    /// Selected ordinary encode backend.
    pub encode_backend: OperationBackendReport,
    /// Selected ordinary strict-decode backend.
    pub strict_decode_backend: OperationBackendReport,
    /// Secret decode backend, independently fixed to the scalar
    /// constant-time-oriented boundary.
    pub secret_decode_backend: OperationBackendReport,
    /// Strongest backend candidate visible to the current build.
    pub candidate: Backend,
    /// Whether candidate visibility came from runtime CPU probing,
    /// compile-time target features, or a disabled SIMD feature.
    pub candidate_detection_mode: CandidateDetectionMode,
    /// Whether the `simd` feature is enabled in this build.
    pub simd_feature_enabled: bool,
    /// Whether either ordinary operation selects acceleration.
    pub ordinary_acceleration_active: bool,
    /// Whether this build keeps the high-assurance scalar unsafe boundary.
    ///
    /// This is a conservative compile-time posture signal. It is `true`
    /// only when the reserved `simd` feature is disabled; `simd` builds
    /// expose additional private prototype boundaries and must use the
    /// release evidence scripts for boundary validation.
    pub unsafe_boundary_enforced: bool,
    /// Wasm artifact selection, distinct from native runtime CPU dispatch.
    pub wasm_artifact_posture: WasmArtifactPosture,
    /// Wasm host-runtime identification posture.
    pub wasm_runtime_posture: WasmRuntimePosture,
    /// Current wipe-barrier posture.
    pub wipe_posture: WipePosture,
    /// Current constant-time result-gate barrier posture.
    pub ct_gate_posture: CtGatePosture,
}

/// Compact structured backend snapshot for logging and policy evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct BackendSnapshot {
    /// Stable compatibility alias for the ordinary encode backend.
    pub active: &'static str,
    /// Compatibility alias reporting whether ordinary encode is accelerated.
    pub accelerated_backend_active: bool,
    /// Stable compatibility posture for ordinary encode and its candidate.
    pub security_posture: &'static str,
    /// Stable ordinary encode report.
    pub encode_backend: super::OperationBackendSnapshot,
    /// Stable ordinary strict-decode report.
    pub strict_decode_backend: super::OperationBackendSnapshot,
    /// Stable secret decode report.
    pub secret_decode_backend: super::OperationBackendSnapshot,
    /// Stable detected candidate identifier.
    pub candidate: &'static str,
    /// Stable SIMD candidate detection-mode identifier.
    pub candidate_detection_mode: &'static str,
    /// CPU features required by the detected candidate.
    pub candidate_required_cpu_features: &'static [&'static str],
    /// Whether the `simd` feature is enabled in this build.
    pub simd_feature_enabled: bool,
    /// Whether either ordinary operation selects acceleration.
    pub ordinary_acceleration_active: bool,
    /// Whether this build keeps the high-assurance scalar unsafe boundary.
    ///
    /// This is `false` for `simd` builds even while execution remains
    /// scalar-only, because those builds include additional private
    /// prototype boundaries.
    pub unsafe_boundary_enforced: bool,
    /// Stable Wasm artifact posture identifier.
    pub wasm_artifact_posture: &'static str,
    /// Stable Wasm host-runtime posture identifier.
    pub wasm_runtime_posture: &'static str,
    /// Stable wipe-barrier posture identifier.
    pub wipe_posture: &'static str,
    /// Stable constant-time result-gate barrier posture identifier.
    pub ct_gate_posture: &'static str,
}

impl core::fmt::Display for BackendReport {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "active={} accelerated_backend_active={} security_posture={} encode_backend={} strict_decode_backend={} secret_decode_backend={} candidate={} candidate_detection_mode={} candidate_required_cpu_features=",
            self.active,
            self.accelerated_backend_active,
            self.security_posture,
            self.encode_backend.backend,
            self.strict_decode_backend.backend,
            self.secret_decode_backend.backend,
            self.candidate,
            self.candidate_detection_mode,
        )?;
        write_feature_list(formatter, self.candidate_required_cpu_features())?;
        write!(
            formatter,
            " simd_feature_enabled={} ordinary_acceleration_active={} unsafe_boundary_enforced={} wasm_artifact_posture={} wasm_runtime_posture={} wipe_posture={} ct_gate_posture={}",
            self.simd_feature_enabled,
            self.ordinary_acceleration_active,
            self.unsafe_boundary_enforced,
            self.wasm_artifact_posture.as_str(),
            self.wasm_runtime_posture.as_str(),
            self.wipe_posture,
            self.ct_gate_posture,
        )
    }
}

impl BackendReport {
    /// Returns whether this report satisfies `policy`.
    ///
    /// ```
    /// let report = base64_ng::runtime::backend_report();
    ///
    /// let scalar_only =
    ///     report.satisfies(base64_ng::runtime::BackendPolicy::ScalarExecutionOnly);
    /// assert_eq!(scalar_only, !report.ordinary_acceleration_active);
    /// ```
    #[must_use]
    pub const fn satisfies(self, policy: BackendPolicy) -> bool {
        match policy {
            BackendPolicy::ScalarExecutionOnly => {
                matches!(
                    self.encode_backend.security_posture,
                    OperationSecurityPosture::OrdinaryScalar
                ) && matches!(
                    self.strict_decode_backend.security_posture,
                    OperationSecurityPosture::OrdinaryScalar
                ) && !self.ordinary_acceleration_active
            }
            BackendPolicy::SimdFeatureDisabled => !self.simd_feature_enabled,
            BackendPolicy::NoDetectedSimdCandidate => matches!(self.candidate, Backend::Scalar),
            BackendPolicy::HighAssuranceScalarOnly => {
                matches!(
                    self.encode_backend.security_posture,
                    OperationSecurityPosture::OrdinaryScalar
                ) && matches!(
                    self.strict_decode_backend.security_posture,
                    OperationSecurityPosture::OrdinaryScalar
                ) && matches!(
                    self.secret_decode_backend.security_posture,
                    OperationSecurityPosture::ScalarConstantTimeOriented
                ) && matches!(self.candidate, Backend::Scalar)
                    && !self.simd_feature_enabled
                    && !self.ordinary_acceleration_active
                    && self.unsafe_boundary_enforced
                    && matches!(
                        self.ct_gate_posture,
                        CtGatePosture::HardwareSpeculationBarrier
                            | CtGatePosture::HardwareSpeculationBarrierBuildAsserted
                    )
            }
        }
    }

    /// Returns the CPU features required by the detected candidate.
    ///
    /// ```
    /// let report = base64_ng::runtime::backend_report();
    ///
    /// assert_eq!(
    ///     report.candidate_required_cpu_features(),
    ///     report.candidate.required_cpu_features(),
    /// );
    /// ```
    #[must_use]
    pub const fn candidate_required_cpu_features(self) -> &'static [&'static str] {
        self.candidate.required_cpu_features()
    }

    /// Returns the typed backend selected by ordinary strict decode.
    ///
    /// Prefer [`Self::strict_decode_backend`] for stable logging.
    #[must_use]
    pub fn active_decode_backend(self) -> Backend {
        let _ = self;
        active_decode_backend()
    }

    /// Returns whether `base64-ng` itself locks secret buffers into physical
    /// memory.
    ///
    /// This crate intentionally has no OS-specific `mlock`/`VirtualLock`
    /// integration. High-assurance deployments should pair secret buffers with
    /// their own platform-approved memory-locking, swap, hibernation, and
    /// crash-dump controls.
    #[must_use]
    pub const fn memory_lock_posture(self) -> MemoryLockPosture {
        let _ = self;
        MemoryLockPosture::NotProvided
    }

    /// Returns a compact structured snapshot with stable string values.
    ///
    /// ```
    /// let snapshot = base64_ng::runtime::backend_report().snapshot();
    ///
    /// assert_eq!(
    ///     snapshot.ordinary_acceleration_active,
    ///     snapshot.encode_backend.backend != "scalar"
    ///         || snapshot.strict_decode_backend.backend != "scalar",
    /// );
    /// ```
    #[must_use]
    pub const fn snapshot(self) -> BackendSnapshot {
        BackendSnapshot {
            active: self.active.as_str(),
            accelerated_backend_active: self.accelerated_backend_active,
            security_posture: self.security_posture.as_str(),
            encode_backend: self.encode_backend.snapshot(),
            strict_decode_backend: self.strict_decode_backend.snapshot(),
            secret_decode_backend: self.secret_decode_backend.snapshot(),
            candidate: self.candidate.as_str(),
            candidate_detection_mode: self.candidate_detection_mode.as_str(),
            candidate_required_cpu_features: self.candidate_required_cpu_features(),
            simd_feature_enabled: self.simd_feature_enabled,
            ordinary_acceleration_active: self.ordinary_acceleration_active,
            unsafe_boundary_enforced: self.unsafe_boundary_enforced,
            wasm_artifact_posture: self.wasm_artifact_posture.as_str(),
            wasm_runtime_posture: self.wasm_runtime_posture.as_str(),
            wipe_posture: self.wipe_posture.as_str(),
            ct_gate_posture: self.ct_gate_posture.as_str(),
        }
    }
}

/// Returns the runtime backend report for this build and target.
///
/// ```
/// let report = base64_ng::runtime::backend_report();
///
/// assert_eq!(
///     report.secret_decode_backend.backend.as_str(),
///     "scalar-constant-time-oriented",
/// );
/// ```
#[must_use]
pub fn backend_report() -> BackendReport {
    let encode = active_backend();
    let strict_decode = active_decode_backend();
    let candidate = detected_candidate();
    let candidate_detection_mode = candidate_detection_mode();
    let accelerated_backend_active = encode != Backend::Scalar;
    let ordinary_acceleration_active =
        encode != Backend::Scalar || strict_decode != Backend::Scalar;
    let unsafe_boundary_enforced = !cfg!(feature = "simd");
    let security_posture = if accelerated_backend_active {
        SecurityPosture::Accelerated
    } else if candidate == Backend::Scalar {
        SecurityPosture::ScalarOnly
    } else {
        SecurityPosture::SimdCandidateScalarActive
    };

    BackendReport {
        active: encode,
        accelerated_backend_active,
        security_posture,
        encode_backend: OperationBackendReport::ordinary(OperationKind::Encode, encode),
        strict_decode_backend: OperationBackendReport::ordinary(
            OperationKind::StrictDecode,
            strict_decode,
        ),
        secret_decode_backend: OperationBackendReport::secret_decode(),
        candidate,
        candidate_detection_mode,
        simd_feature_enabled: cfg!(feature = "simd"),
        ordinary_acceleration_active,
        unsafe_boundary_enforced,
        wasm_artifact_posture: wasm_artifact_posture(),
        wasm_runtime_posture: wasm_runtime_posture(),
        wipe_posture: wipe_posture(),
        ct_gate_posture: ct_gate_posture(),
    }
}

const fn wipe_posture() -> WipePosture {
    if cfg!(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "x86",
        target_arch = "x86_64",
    )) {
        WipePosture::HardwareFence
    } else {
        WipePosture::CompilerFenceOnly
    }
}

const fn ct_gate_posture() -> CtGatePosture {
    if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        CtGatePosture::HardwareSpeculationBarrier
    } else if cfg!(all(
        target_arch = "aarch64",
        base64_ng_aarch64_csdb_attested
    )) {
        CtGatePosture::HardwareSpeculationBarrierBuildAsserted
    } else if cfg!(target_arch = "aarch64") {
        CtGatePosture::HardwareSpeculationBarrierUnattested
    } else if cfg!(any(
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64"
    )) {
        CtGatePosture::OrderingFence
    } else {
        CtGatePosture::CompilerFenceOnly
    }
}

/// Requires the current runtime backend report to satisfy `policy`.
///
/// ```
/// let result = base64_ng::runtime::require_backend_policy(
///     base64_ng::runtime::BackendPolicy::ScalarExecutionOnly,
/// );
///
/// if base64_ng::runtime::backend_report().ordinary_acceleration_active {
///     assert!(result.is_err());
/// } else {
///     assert!(result.is_ok());
/// }
/// ```
pub fn require_backend_policy(policy: BackendPolicy) -> Result<(), BackendPolicyError> {
    let report = backend_report();
    if report.satisfies(policy) {
        Ok(())
    } else {
        Err(BackendPolicyError { policy, report })
    }
}

fn write_feature_list(
    formatter: &mut core::fmt::Formatter<'_>,
    features: &[&str],
) -> core::fmt::Result {
    formatter.write_str("[")?;
    let mut index = 0;
    while index < features.len() {
        if index != 0 {
            formatter.write_str(",")?;
        }
        formatter.write_str(features[index])?;
        index += 1;
    }
    formatter.write_str("]")
}

#[cfg(feature = "simd")]
fn active_backend() -> Backend {
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => Backend::Scalar,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx512Vbmi => Backend::Avx512Vbmi,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx2 => Backend::Avx2,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Ssse3Sse41 => Backend::Ssse3Sse41,
        #[cfg(all(feature = "std", target_arch = "aarch64", target_endian = "little"))]
        crate::simd::ActiveBackend::Neon => Backend::Neon,
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        crate::simd::ActiveBackend::WasmSimd128 => Backend::WasmSimd128,
    }
}

#[cfg(not(feature = "simd"))]
const fn active_backend() -> Backend {
    Backend::Scalar
}

#[cfg(feature = "simd")]
fn active_decode_backend() -> Backend {
    match crate::decode_backend::active_decode_backend() {
        crate::decode_backend::DecodeBackend::Scalar => Backend::Scalar,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::decode_backend::DecodeBackend::Avx512Vbmi => Backend::Avx512Vbmi,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::decode_backend::DecodeBackend::Avx2 => Backend::Avx2,
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::decode_backend::DecodeBackend::Ssse3Sse41 => Backend::Ssse3Sse41,
        #[cfg(all(feature = "std", target_arch = "aarch64", target_endian = "little"))]
        crate::decode_backend::DecodeBackend::Neon => Backend::Neon,
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        crate::decode_backend::DecodeBackend::WasmSimd128 => Backend::WasmSimd128,
    }
}

#[cfg(not(feature = "simd"))]
const fn active_decode_backend() -> Backend {
    Backend::Scalar
}

#[cfg(feature = "simd")]
fn detected_candidate() -> Backend {
    match crate::simd::detected_candidate() {
        crate::simd::Candidate::Scalar => Backend::Scalar,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        crate::simd::Candidate::Avx512Vbmi => Backend::Avx512Vbmi,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        crate::simd::Candidate::Avx2 => Backend::Avx2,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        crate::simd::Candidate::Ssse3Sse41 => Backend::Ssse3Sse41,
        #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
        crate::simd::Candidate::Neon => Backend::Neon,
        #[cfg(target_arch = "wasm32")]
        crate::simd::Candidate::WasmSimd128 => Backend::WasmSimd128,
    }
}

#[cfg(not(feature = "simd"))]
const fn detected_candidate() -> Backend {
    Backend::Scalar
}

#[cfg(all(
    feature = "simd",
    feature = "std",
    any(target_arch = "x86", target_arch = "x86_64")
))]
const fn candidate_detection_mode() -> CandidateDetectionMode {
    CandidateDetectionMode::RuntimeCpuFeatures
}

#[cfg(all(
    feature = "simd",
    not(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))
))]
const fn candidate_detection_mode() -> CandidateDetectionMode {
    CandidateDetectionMode::CompileTimeTargetFeatures
}

#[cfg(not(feature = "simd"))]
const fn candidate_detection_mode() -> CandidateDetectionMode {
    CandidateDetectionMode::SimdFeatureDisabled
}
