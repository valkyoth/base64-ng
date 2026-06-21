//! Runtime backend reporting for security-sensitive deployments.
//!
//! This module exposes backend posture so callers can log, assert, or audit
//! whether execution is scalar-only, using an admitted encode backend, or
//! merely detecting future SIMD candidates.

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
    /// Security logs can record exactly which CPU feature bundle is required by
    /// an active backend or visible candidate.
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

mod report;

pub use report::{
    BackendPolicyError, BackendReport, BackendSnapshot, backend_report, require_backend_policy,
};
