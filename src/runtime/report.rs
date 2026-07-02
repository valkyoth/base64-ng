use super::{
    Backend, BackendPolicy, CandidateDetectionMode, CtGatePosture, MemoryLockPosture,
    SecurityPosture, WipePosture,
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
pub struct BackendReport {
    /// Backend currently used for admitted runtime dispatch.
    ///
    /// This field reports the primary active backend used by the established
    /// encode dispatch boundary. Decode has its own narrower admission path;
    /// use [`Self::active_decode_backend`] to inspect decode dispatch.
    pub active: Backend,
    /// Strongest backend candidate visible to the current build.
    pub candidate: Backend,
    /// Whether candidate visibility came from runtime CPU probing,
    /// compile-time target features, or a disabled SIMD feature.
    pub candidate_detection_mode: CandidateDetectionMode,
    /// Whether the `simd` feature is enabled in this build.
    pub simd_feature_enabled: bool,
    /// Whether an accelerated SIMD backend is active.
    pub accelerated_backend_active: bool,
    /// Whether this build keeps the high-assurance scalar unsafe boundary.
    ///
    /// This is a conservative compile-time posture signal. It is `true`
    /// only when the reserved `simd` feature is disabled; `simd` builds
    /// expose additional private prototype boundaries and must use the
    /// release evidence scripts for boundary validation.
    pub unsafe_boundary_enforced: bool,
    /// Current security posture.
    pub security_posture: SecurityPosture,
    /// Current wipe-barrier posture.
    pub wipe_posture: WipePosture,
    /// Current constant-time result-gate barrier posture.
    pub ct_gate_posture: CtGatePosture,
}

/// Compact structured backend snapshot for logging and policy evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendSnapshot {
    /// Stable active backend identifier.
    ///
    /// Non-scalar active values describe the primary admitted encode backend.
    /// Decode dispatch can be queried separately through
    /// [`BackendReport::active_decode_backend`].
    pub active: &'static str,
    /// Stable detected candidate identifier.
    pub candidate: &'static str,
    /// Stable SIMD candidate detection-mode identifier.
    pub candidate_detection_mode: &'static str,
    /// CPU features required by the detected candidate.
    pub candidate_required_cpu_features: &'static [&'static str],
    /// Whether the `simd` feature is enabled in this build.
    pub simd_feature_enabled: bool,
    /// Whether an accelerated SIMD backend is active.
    pub accelerated_backend_active: bool,
    /// Whether this build keeps the high-assurance scalar unsafe boundary.
    ///
    /// This is `false` for `simd` builds even while execution remains
    /// scalar-only, because those builds include additional private
    /// prototype boundaries.
    pub unsafe_boundary_enforced: bool,
    /// Stable security posture identifier.
    pub security_posture: &'static str,
    /// Stable wipe-barrier posture identifier.
    pub wipe_posture: &'static str,
    /// Stable constant-time result-gate barrier posture identifier.
    pub ct_gate_posture: &'static str,
}

impl core::fmt::Display for BackendReport {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "active={} candidate={} candidate_detection_mode={} candidate_required_cpu_features=",
            self.active, self.candidate, self.candidate_detection_mode,
        )?;
        write_feature_list(formatter, self.candidate_required_cpu_features())?;
        write!(
            formatter,
            " simd_feature_enabled={} accelerated_backend_active={} unsafe_boundary_enforced={} security_posture={} wipe_posture={} ct_gate_posture={}",
            self.simd_feature_enabled,
            self.accelerated_backend_active,
            self.unsafe_boundary_enforced,
            self.security_posture,
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
    /// assert_eq!(scalar_only, !report.accelerated_backend_active);
    /// ```
    #[must_use]
    pub const fn satisfies(self, policy: BackendPolicy) -> bool {
        match policy {
            BackendPolicy::ScalarExecutionOnly => {
                matches!(self.active, Backend::Scalar) && !self.accelerated_backend_active
            }
            BackendPolicy::SimdFeatureDisabled => !self.simd_feature_enabled,
            BackendPolicy::NoDetectedSimdCandidate => matches!(self.candidate, Backend::Scalar),
            BackendPolicy::HighAssuranceScalarOnly => {
                matches!(self.active, Backend::Scalar)
                    && matches!(self.candidate, Backend::Scalar)
                    && !self.simd_feature_enabled
                    && !self.accelerated_backend_active
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

    /// Returns the active backend for the normal strict decode boundary.
    ///
    /// This is intentionally separate from [`Self::active`]. In the `1.3.0`
    /// decode line, strict decode may admit a narrower backend than encode;
    /// unsupported decode surfaces still return scalar here through the public
    /// API fallback rules.
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
    ///     snapshot.accelerated_backend_active,
    ///     snapshot.active != "scalar",
    /// );
    /// ```
    #[must_use]
    pub const fn snapshot(self) -> BackendSnapshot {
        BackendSnapshot {
            active: self.active.as_str(),
            candidate: self.candidate.as_str(),
            candidate_detection_mode: self.candidate_detection_mode.as_str(),
            candidate_required_cpu_features: self.candidate_required_cpu_features(),
            simd_feature_enabled: self.simd_feature_enabled,
            accelerated_backend_active: self.accelerated_backend_active,
            unsafe_boundary_enforced: self.unsafe_boundary_enforced,
            security_posture: self.security_posture.as_str(),
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
/// if report.accelerated_backend_active {
///     assert_ne!(report.active, base64_ng::runtime::Backend::Scalar);
/// }
/// ```
#[must_use]
pub fn backend_report() -> BackendReport {
    let active = active_backend();
    let candidate = detected_candidate();
    let candidate_detection_mode = candidate_detection_mode();
    let accelerated_backend_active = active != Backend::Scalar;
    let unsafe_boundary_enforced = !cfg!(feature = "simd");
    let security_posture = if accelerated_backend_active {
        SecurityPosture::Accelerated
    } else if candidate != Backend::Scalar {
        SecurityPosture::SimdCandidateScalarActive
    } else {
        SecurityPosture::ScalarOnly
    };

    BackendReport {
        active,
        candidate,
        candidate_detection_mode,
        simd_feature_enabled: cfg!(feature = "simd"),
        accelerated_backend_active,
        unsafe_boundary_enforced,
        security_posture,
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
/// if base64_ng::runtime::backend_report().accelerated_backend_active {
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
        #[cfg(all(feature = "std", target_arch = "aarch64"))]
        crate::simd::ActiveBackend::Neon => Backend::Neon,
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
