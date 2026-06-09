//! Runtime backend reporting for security-sensitive deployments.
//!
//! This module does not enable acceleration. It exposes the backend posture so
//! callers can log, assert, or audit whether execution is scalar-only or merely
//! detecting future SIMD candidates.

/// A backend that can be reported by `base64-ng`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Backend {
    /// The audited scalar backend.
    Scalar,
    /// An AVX-512 VBMI candidate was detected.
    Avx512Vbmi,
    /// An AVX2 candidate was detected.
    Avx2,
    /// An SSSE3/SSE4.1 candidate was detected.
    Ssse3Sse41,
    /// An ARM NEON candidate was detected.
    Neon,
    /// A wasm `simd128` candidate was detected.
    WasmSimd128,
}

impl Backend {
    /// Returns the stable lowercase identifier for this backend.
    ///
    /// ```
    /// assert_eq!(base64_ng::runtime::Backend::Scalar.as_str(), "scalar");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Avx512Vbmi => "avx512-vbmi",
            Self::Avx2 => "avx2",
            Self::Ssse3Sse41 => "ssse3-sse4.1",
            Self::Neon => "neon",
            Self::WasmSimd128 => "wasm-simd128",
        }
    }

    /// Returns the CPU features required before this backend may be used.
    ///
    /// The active backend is still scalar-only. This method exists so
    /// security logs can record exactly which future backend feature bundle
    /// was detected.
    ///
    /// ```
    /// assert_eq!(
    ///     base64_ng::runtime::Backend::Avx512Vbmi.required_cpu_features(),
    ///     ["avx512f", "avx512bw", "avx512vl", "avx512vbmi"],
    /// );
    /// ```
    #[must_use]
    pub const fn required_cpu_features(self) -> &'static [&'static str] {
        match self {
            Self::Scalar => &[],
            Self::Avx512Vbmi => &["avx512f", "avx512bw", "avx512vl", "avx512vbmi"],
            Self::Avx2 => &["avx2"],
            Self::Ssse3Sse41 => &["ssse3", "sse4.1"],
            Self::Neon => &["neon"],
            Self::WasmSimd128 => &["simd128"],
        }
    }
}

impl core::fmt::Display for Backend {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// How SIMD backend candidates were detected for this build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CandidateDetectionMode {
    /// SIMD candidate detection is disabled because the `simd` feature is
    /// not enabled.
    SimdFeatureDisabled,
    /// Candidate detection uses runtime CPU feature probing.
    RuntimeCpuFeatures,
    /// Candidate detection uses compile-time target features.
    ///
    /// This mode does not prove that the deployment CPU has the reported
    /// feature; it only reflects how the binary was compiled.
    CompileTimeTargetFeatures,
}

impl CandidateDetectionMode {
    /// Returns the stable lowercase identifier for this detection mode.
    ///
    /// ```
    /// assert_eq!(
    ///     base64_ng::runtime::CandidateDetectionMode::SimdFeatureDisabled.as_str(),
    ///     "simd-feature-disabled",
    /// );
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SimdFeatureDisabled => "simd-feature-disabled",
            Self::RuntimeCpuFeatures => "runtime-cpu-features",
            Self::CompileTimeTargetFeatures => "compile-time-target-features",
        }
    }
}

impl core::fmt::Display for CandidateDetectionMode {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Security posture for the active runtime backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecurityPosture {
    /// No accelerated backend is active.
    ScalarOnly,
    /// SIMD support may be detected, but execution still uses scalar.
    SimdCandidateScalarActive,
    /// A SIMD backend is active.
    Accelerated,
}

impl SecurityPosture {
    /// Returns the stable lowercase identifier for this security posture.
    ///
    /// ```
    /// assert_eq!(
    ///     base64_ng::runtime::SecurityPosture::ScalarOnly.as_str(),
    ///     "scalar-only",
    /// );
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScalarOnly => "scalar-only",
            Self::SimdCandidateScalarActive => "simd-candidate-scalar-active",
            Self::Accelerated => "accelerated",
        }
    }
}

impl core::fmt::Display for SecurityPosture {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Wipe-barrier posture for this build and target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WipePosture {
    /// The target uses a native store-ordering hardware fence in addition
    /// to volatile writes and compiler fences.
    ///
    /// This describes wipe-store ordering only. It is separate from
    /// [`CtGatePosture`], which reports whether the constant-time result
    /// gate has a speculation barrier or only an ordering fence.
    HardwareFence,
    /// The target uses volatile writes and compiler fences only.
    CompilerFenceOnly,
}

impl WipePosture {
    /// Returns the stable lowercase identifier for this wipe posture.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HardwareFence => "hardware-fence",
            Self::CompilerFenceOnly => "compiler-fence-only",
        }
    }
}

impl core::fmt::Display for WipePosture {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Constant-time result-gate barrier posture for this build and target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CtGatePosture {
    /// The target uses a native speculation barrier before public CT
    /// success/failure or equality-result branches.
    HardwareSpeculationBarrier,
    /// The target is treated as having an effective speculation barrier only
    /// because the build provided an explicit operator attestation cfg.
    ///
    /// On `AArch64`, this is reported when the build sets
    /// `base64_ng_aarch64_csdb_attested`. It remains distinct from
    /// [`Self::HardwareSpeculationBarrier`] so logs preserve the evidence
    /// chain instead of making a build assertion look like a native target
    /// guarantee.
    HardwareSpeculationBarrierBuildAsserted,
    /// The target emits a hardware speculation-barrier sequence whose
    /// effectiveness depends on platform or core-level attestation.
    ///
    /// On `AArch64` this uses `isb sy` plus the CSDB hint encoding. Full
    /// CSDB effectiveness depends on the deployed ARM architecture level;
    /// older cores may treat the hint as a no-op.
    HardwareSpeculationBarrierUnattested,
    /// The target uses an ordering fence where the base ISA does not
    /// provide a canonical speculation barrier.
    OrderingFence,
    /// The target uses compiler fences only.
    CompilerFenceOnly,
}

impl CtGatePosture {
    /// Returns the stable lowercase identifier for this CT gate posture.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HardwareSpeculationBarrier => "hardware-speculation-barrier",
            Self::HardwareSpeculationBarrierBuildAsserted => {
                "hardware-speculation-barrier-build-asserted"
            }
            Self::HardwareSpeculationBarrierUnattested => "hardware-speculation-barrier-unattested",
            Self::OrderingFence => "ordering-fence",
            Self::CompilerFenceOnly => "compiler-fence-only",
        }
    }
}

impl core::fmt::Display for CtGatePosture {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Whether this crate locks secret allocations into physical memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MemoryLockPosture {
    /// The crate does not lock memory. Deployments that need locked secret
    /// pages must use platform controls outside `base64-ng`.
    NotProvided,
}

impl MemoryLockPosture {
    /// Returns the stable lowercase identifier for this memory-locking posture.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotProvided => "not-provided",
        }
    }
}

impl core::fmt::Display for MemoryLockPosture {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Deployment policy for runtime backend assertions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BackendPolicy {
    /// Require encode/decode execution to use the scalar backend.
    ScalarExecutionOnly,
    /// Require the crate to be built without the `simd` feature.
    SimdFeatureDisabled,
    /// Require no SIMD candidate to be visible to this build and target.
    NoDetectedSimdCandidate,
    /// Require scalar execution, the `simd` feature disabled, no detected
    /// SIMD candidate, the unsafe boundary enforced, and a CT result gate
    /// classified as a native hardware speculation barrier.
    ///
    /// This policy intentionally rejects targets that report only an
    /// unattested hardware barrier, ordering fence, or compiler fence for the
    /// CT result gate. On `AArch64`, the crate emits `isb sy` plus the CSDB
    /// hint but reports that posture as unattested; deployments that rely on
    /// CSDB must carry platform evidence outside this built-in policy check.
    HighAssuranceScalarOnly,
}

impl BackendPolicy {
    /// Returns the stable lowercase identifier for this policy.
    ///
    /// ```
    /// assert_eq!(
    ///     base64_ng::runtime::BackendPolicy::HighAssuranceScalarOnly.as_str(),
    ///     "high-assurance-scalar-only",
    /// );
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScalarExecutionOnly => "scalar-execution-only",
            Self::SimdFeatureDisabled => "simd-feature-disabled",
            Self::NoDetectedSimdCandidate => "no-detected-simd-candidate",
            Self::HighAssuranceScalarOnly => "high-assurance-scalar-only",
        }
    }
}

impl core::fmt::Display for BackendPolicy {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

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
    /// Backend currently used for encode/decode dispatch.
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
    /// assert!(
    ///     report.satisfies(base64_ng::runtime::BackendPolicy::ScalarExecutionOnly)
    /// );
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
    /// assert_eq!(snapshot.active, "scalar");
    /// assert!(!snapshot.accelerated_backend_active);
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
/// assert_eq!(report.active, base64_ng::runtime::Backend::Scalar);
/// assert!(!report.accelerated_backend_active);
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
/// base64_ng::runtime::require_backend_policy(
///     base64_ng::runtime::BackendPolicy::ScalarExecutionOnly,
/// )
/// .unwrap();
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
    match super::simd::active_backend() {
        super::simd::ActiveBackend::Scalar => Backend::Scalar,
    }
}

#[cfg(not(feature = "simd"))]
const fn active_backend() -> Backend {
    Backend::Scalar
}

#[cfg(feature = "simd")]
fn detected_candidate() -> Backend {
    match super::simd::detected_candidate() {
        super::simd::Candidate::Scalar => Backend::Scalar,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        super::simd::Candidate::Avx512Vbmi => Backend::Avx512Vbmi,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        super::simd::Candidate::Avx2 => Backend::Avx2,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        super::simd::Candidate::Ssse3Sse41 => Backend::Ssse3Sse41,
        #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
        super::simd::Candidate::Neon => Backend::Neon,
        #[cfg(target_arch = "wasm32")]
        super::simd::Candidate::WasmSimd128 => Backend::WasmSimd128,
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
