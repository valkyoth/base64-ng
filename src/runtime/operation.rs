//! Stable per-operation backend reporting primitives.

use super::Backend;

/// Operation whose backend is being reported.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum OperationKind {
    /// Ordinary Base64 encoding.
    Encode,
    /// Ordinary strict Base64 decoding.
    StrictDecode,
    /// Secret, fixed-work, constant-time-oriented decoding.
    SecretDecode,
}

impl OperationKind {
    /// Returns the stable operation identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::StrictDecode => "strict-decode",
            Self::SecretDecode => "secret-decode",
        }
    }
}

/// Opaque stable identifier for one operation backend.
///
/// Values are created by `base64-ng`; callers can log or compare the stable
/// string without assuming that future backends extend a public enum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BackendIdentifier(&'static str);

impl BackendIdentifier {
    pub(crate) const SCALAR_CT_ORIENTED: Self = Self("scalar-constant-time-oriented");

    pub(crate) const fn ordinary(backend: Backend) -> Self {
        Self(backend.as_str())
    }

    /// Returns the stable lowercase identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl core::fmt::Display for BackendIdentifier {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Security classification of one operation backend.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum OperationSecurityPosture {
    /// Ordinary scalar processing; no secret timing claim.
    OrdinaryScalar,
    /// Ordinary accelerated processing; no secret timing claim.
    OrdinaryAccelerated,
    /// Scalar fixed-work, constant-time-oriented secret processing.
    ScalarConstantTimeOriented,
}

impl OperationSecurityPosture {
    /// Returns the stable posture identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OrdinaryScalar => "ordinary-scalar",
            Self::OrdinaryAccelerated => "ordinary-accelerated",
            Self::ScalarConstantTimeOriented => "scalar-constant-time-oriented",
        }
    }
}

/// Current ordinary backend-health evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BackendHealthPosture {
    /// The backend passed the static admission policy only.
    ///
    /// Commit 24 adds runtime known-answer health and quarantine states. This
    /// value deliberately does not imply that such a self-test has run.
    AdmissionPolicyOnly,
    /// The secret scalar boundary is fixed by policy rather than selected by
    /// ordinary SIMD dispatch.
    SecretPolicyFixed,
}

impl BackendHealthPosture {
    /// Returns the stable health identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdmissionPolicyOnly => "admission-policy-only",
            Self::SecretPolicyFixed => "secret-policy-fixed",
        }
    }
}

/// Wasm artifact selection posture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WasmArtifactPosture {
    /// The current target is not Wasm.
    NotWasm,
    /// A scalar Wasm artifact was selected.
    ScalarArtifact,
    /// A `simd128` Wasm artifact was selected at compile time.
    Simd128Artifact,
}

impl WasmArtifactPosture {
    /// Returns the stable artifact identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotWasm => "not-wasm",
            Self::ScalarArtifact => "wasm-scalar-artifact",
            Self::Simd128Artifact => "wasm-simd128-artifact",
        }
    }
}

/// Wasm host-runtime identification posture.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WasmRuntimePosture {
    /// The current target is not Wasm.
    NotWasm,
    /// The guest cannot authenticate which host runtime/JIT selected it.
    HostRuntimeUnidentified,
}

impl WasmRuntimePosture {
    /// Returns the stable runtime identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotWasm => "not-wasm",
            Self::HostRuntimeUnidentified => "wasm-host-runtime-unidentified",
        }
    }
}

/// Selected backend and posture for one operation family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationBackendReport {
    /// Operation family.
    pub operation: OperationKind,
    /// Opaque stable backend identifier.
    pub backend: BackendIdentifier,
    /// Security classification for this operation only.
    pub security_posture: OperationSecurityPosture,
    /// Honest current backend-health posture.
    pub health_posture: BackendHealthPosture,
    /// Context-independent admission generation.
    ///
    /// This generation is not an [`crate::assurance::AssuranceContext`]
    /// generation. Allocation-gated reports carry those values separately.
    pub health_generation: usize,
}

impl OperationBackendReport {
    pub(crate) const fn ordinary(operation: OperationKind, backend: Backend) -> Self {
        let security_posture = if matches!(backend, Backend::Scalar) {
            OperationSecurityPosture::OrdinaryScalar
        } else {
            OperationSecurityPosture::OrdinaryAccelerated
        };
        Self {
            operation,
            backend: BackendIdentifier::ordinary(backend),
            security_posture,
            health_posture: BackendHealthPosture::AdmissionPolicyOnly,
            health_generation: 1,
        }
    }

    pub(crate) const fn secret_decode() -> Self {
        Self {
            operation: OperationKind::SecretDecode,
            backend: BackendIdentifier::SCALAR_CT_ORIENTED,
            security_posture: OperationSecurityPosture::ScalarConstantTimeOriented,
            health_posture: BackendHealthPosture::SecretPolicyFixed,
            health_generation: 1,
        }
    }

    /// Returns a stable structured snapshot.
    #[must_use]
    pub const fn snapshot(self) -> OperationBackendSnapshot {
        OperationBackendSnapshot {
            operation: self.operation.as_str(),
            backend: self.backend.as_str(),
            security_posture: self.security_posture.as_str(),
            health_posture: self.health_posture.as_str(),
            health_generation: self.health_generation,
        }
    }
}

/// Stable logging snapshot for one operation backend.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationBackendSnapshot {
    /// Stable operation identifier.
    pub operation: &'static str,
    /// Stable opaque backend identifier.
    pub backend: &'static str,
    /// Stable operation-security identifier.
    pub security_posture: &'static str,
    /// Stable backend-health identifier.
    pub health_posture: &'static str,
    /// Backend-health generation.
    pub health_generation: usize,
}

pub(crate) const fn wasm_artifact_posture() -> WasmArtifactPosture {
    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    if crate::simd::wasm_simd128_artifact_selected() {
        return WasmArtifactPosture::Simd128Artifact;
    }
    if cfg!(target_arch = "wasm32") {
        WasmArtifactPosture::ScalarArtifact
    } else {
        WasmArtifactPosture::NotWasm
    }
}

pub(crate) const fn wasm_runtime_posture() -> WasmRuntimePosture {
    if cfg!(target_arch = "wasm32") {
        WasmRuntimePosture::HostRuntimeUnidentified
    } else {
        WasmRuntimePosture::NotWasm
    }
}
