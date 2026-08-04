//! Generation-bound assurance context and tokens.

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::runtime::{CtGatePosture, WipePosture};

use super::provider::ProtectedMemoryProvider;

/// Snapshot of the four independent context generations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssuranceGenerations {
    /// Ordinary backend-health generation.
    pub ordinary_backend: usize,
    /// Secret scalar-algorithm policy generation.
    pub secret_algorithm: usize,
    /// Wipe primitive and barrier generation.
    pub wipe_barrier: usize,
    /// Speculation-posture generation.
    pub speculation: usize,
}

/// Marker for dependency-free best-effort secret policy.
#[derive(Debug)]
pub struct BestEffort {
    _private: (),
}

/// Marker for deployment-attested high-assurance policy.
#[derive(Debug)]
pub struct Attested {
    _private: (),
}

mod sealed {
    pub trait Level {
        const ATTESTED: bool;
    }
}

impl sealed::Level for BestEffort {
    const ATTESTED: bool = false;
}

impl sealed::Level for Attested {
    const ATTESTED: bool = true;
}

/// Assurance-level behavior used by protected owners.
pub trait AssuranceLevel: sealed::Level {}

impl AssuranceLevel for BestEffort {}
impl AssuranceLevel for Attested {}

/// Current target identity bound by platform evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TargetAttestation {
    /// x86 with the crate's reviewed native barriers.
    X86,
    /// x86-64 with the crate's reviewed native barriers.
    X86_64,
    /// `AArch64` with deployment-attested CSDB effectiveness.
    Aarch64Csdb,
    /// A reviewed external embedded target/provider combination.
    ReviewedEmbedded,
}

impl TargetAttestation {
    const fn matches_current_target(self) -> bool {
        match self {
            Self::X86 => cfg!(target_arch = "x86"),
            Self::X86_64 => cfg!(target_arch = "x86_64"),
            Self::Aarch64Csdb => {
                cfg!(all(
                    target_arch = "aarch64",
                    base64_ng_aarch64_csdb_attested
                ))
            }
            Self::ReviewedEmbedded => true,
        }
    }
}

/// Exact wipe procedure named by platform evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum WipeAttestation {
    /// Byte-wise volatile overwrite plus the crate's selected barrier.
    VolatileBytesAndSelectedBarrier,
}

/// Unsafe-provider evidence used to mint an attested token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttestationEvidence {
    target: TargetAttestation,
    wipe: WipeAttestation,
    wipe_posture: WipePosture,
    speculation_posture: CtGatePosture,
    provider_identity: usize,
    provider_generation: usize,
}

impl AttestationEvidence {
    /// Constructs reviewed platform evidence.
    ///
    /// # Safety
    ///
    /// The caller must have independently established that every field is
    /// true for the exact deployed target, provider instance, wipe primitive,
    /// barrier sequence, and generation. Evidence must not outlive or be
    /// replayed across a provider-instance reset.
    #[must_use]
    #[allow(unsafe_code)]
    pub const unsafe fn new(
        target: TargetAttestation,
        wipe: WipeAttestation,
        wipe_posture: WipePosture,
        speculation_posture: CtGatePosture,
        provider_identity: usize,
        provider_generation: usize,
    ) -> Self {
        Self {
            target,
            wipe,
            wipe_posture,
            speculation_posture,
            provider_identity,
            provider_generation,
        }
    }

    pub(crate) const fn provider_identity(self) -> usize {
        self.provider_identity
    }

    pub(crate) const fn provider_generation(self) -> usize {
        self.provider_generation
    }

    pub(crate) const fn wipe_posture(self) -> WipePosture {
        self.wipe_posture
    }

    pub(crate) const fn speculation_posture(self) -> CtGatePosture {
        self.speculation_posture
    }
}

/// Reviewed platform evidence source.
///
/// # Safety
///
/// Implementors must return evidence obtained from the actual deployment and
/// exact provider instance. The implementation must not unwind and must not
/// treat build flags, target names, or requested policy as hardware evidence.
#[allow(unsafe_code)]
pub unsafe trait PlatformAttestation {
    /// Returns current evidence or a redacted failure.
    fn attest(&self) -> Result<AttestationEvidence, AssuranceError>;
}

/// Runtime assurance failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AssuranceError {
    /// A relevant context generation changed.
    StaleGeneration,
    /// The explicit high-assurance build policy is absent.
    HighAssuranceBuildRequired,
    /// Evidence names another target or provider instance.
    MismatchedAttestation,
    /// Wipe or speculation posture is insufficient.
    InsufficientPosture,
    /// Provider health or generation is stale.
    ProviderUnavailable,
}

impl core::fmt::Display for AssuranceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::StaleGeneration => "assurance generation is stale",
            Self::HighAssuranceBuildRequired => "high-assurance build policy is required",
            Self::MismatchedAttestation => "platform attestation does not match",
            Self::InsufficientPosture => "platform posture is insufficient",
            Self::ProviderUnavailable => "protected-memory provider is unavailable",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AssuranceError {}

/// Mutable runtime assurance authority.
///
/// Generation invalidation is monotonic. Allocation protection is deliberately
/// absent from this context and must be proven separately for each protected
/// allocation.
pub struct AssuranceContext {
    ordinary_backend: AtomicUsize,
    secret_algorithm: AtomicUsize,
    wipe_barrier: AtomicUsize,
    speculation: AtomicUsize,
}

impl AssuranceContext {
    /// Creates a fresh context with no platform attestation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ordinary_backend: AtomicUsize::new(1),
            secret_algorithm: AtomicUsize::new(1),
            wipe_barrier: AtomicUsize::new(crate::cleanup::WIPE_PRIMITIVE_REVISION),
            speculation: AtomicUsize::new(1),
        }
    }

    /// Returns all current generation counters.
    #[must_use]
    pub fn generations(&self) -> AssuranceGenerations {
        AssuranceGenerations {
            ordinary_backend: self.ordinary_backend.load(Ordering::Acquire),
            secret_algorithm: self.secret_algorithm.load(Ordering::Acquire),
            wipe_barrier: self.wipe_barrier.load(Ordering::Acquire),
            speculation: self.speculation.load(Ordering::Acquire),
        }
    }

    /// Mints a context-borrowing best-effort token.
    #[must_use]
    pub fn best_effort_token(&self) -> AssuranceToken<'_, BestEffort> {
        AssuranceToken::new(self, self.generations(), None)
    }

    /// Mints an attested token after checking exact provider and target evidence.
    pub fn attested_token<P>(
        &self,
        provider: &P,
    ) -> Result<AssuranceToken<'_, Attested>, AssuranceError>
    where
        P: PlatformAttestation + ProtectedMemoryProvider,
    {
        if !cfg!(base64_ng_require_high_assurance) {
            return Err(AssuranceError::HighAssuranceBuildRequired);
        }
        let evidence = provider.attest()?;
        if !evidence.target.matches_current_target()
            || evidence.provider_identity != provider.provider_identity()
            || evidence.provider_generation != provider.provider_generation()
        {
            return Err(AssuranceError::MismatchedAttestation);
        }
        if evidence.wipe != WipeAttestation::VolatileBytesAndSelectedBarrier
            || !wipe_posture_is_attestable(evidence.wipe_posture)
            || !speculation_posture_is_attestable(evidence.speculation_posture)
        {
            return Err(AssuranceError::InsufficientPosture);
        }
        let generations = self.generations();
        Ok(AssuranceToken::new(self, generations, Some(evidence)))
    }

    /// Invalidates only ordinary backend-health evidence.
    pub fn invalidate_ordinary_backend(&self) {
        advance(&self.ordinary_backend);
    }

    /// Invalidates secret scalar-algorithm evidence.
    pub fn invalidate_secret_algorithm(&self) {
        advance(&self.secret_algorithm);
    }

    /// Invalidates wipe primitive and barrier evidence.
    pub fn invalidate_wipe_barrier(&self) {
        advance(&self.wipe_barrier);
    }

    /// Invalidates speculation-posture evidence.
    pub fn invalidate_speculation(&self) {
        advance(&self.speculation);
    }
}

impl Default for AssuranceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Non-forgeable, context-borrowing assurance token.
///
/// Tokens are deliberately neither `Copy` nor `Clone`. The marker also makes
/// them `!Sync`, `!UnwindSafe`, and `!RefUnwindSafe`; these auto traits are API
/// friction only, never a cleanup security boundary.
pub struct AssuranceToken<'context, Level: AssuranceLevel> {
    context: &'context AssuranceContext,
    generations: AssuranceGenerations,
    evidence: Option<AttestationEvidence>,
    _level: PhantomData<Level>,
    _not_sync_or_unwind_safe: PhantomData<(UnsafeCell<()>, &'context mut dyn FnMut())>,
}

impl<'context, Level: AssuranceLevel> AssuranceToken<'context, Level> {
    fn new(
        context: &'context AssuranceContext,
        generations: AssuranceGenerations,
        evidence: Option<AttestationEvidence>,
    ) -> Self {
        Self {
            context,
            generations,
            evidence,
            _level: PhantomData,
            _not_sync_or_unwind_safe: PhantomData,
        }
    }

    /// Returns the captured generation snapshot.
    #[must_use]
    pub const fn generations(&self) -> AssuranceGenerations {
        self.generations
    }

    /// Revalidates only generations relevant to secret computation.
    pub fn revalidate(&self) -> Result<(), AssuranceError> {
        let current = self.context.generations();
        if current.secret_algorithm == 0
            || current.wipe_barrier == 0
            || (Level::ATTESTED && current.speculation == 0)
            || current.secret_algorithm != self.generations.secret_algorithm
            || current.wipe_barrier != self.generations.wipe_barrier
            || (Level::ATTESTED && current.speculation != self.generations.speculation)
        {
            return Err(AssuranceError::StaleGeneration);
        }
        Ok(())
    }

    pub(crate) const fn context(&self) -> &'context AssuranceContext {
        self.context
    }

    pub(crate) const fn evidence(&self) -> Option<AttestationEvidence> {
        self.evidence
    }

    pub(crate) const fn requires_attestation() -> bool {
        Level::ATTESTED
    }
}

impl AssuranceContext {
    pub(crate) fn revalidate_snapshot<Level: AssuranceLevel>(
        &self,
        generations: AssuranceGenerations,
    ) -> Result<(), AssuranceError> {
        let current = self.generations();
        if current.secret_algorithm == 0
            || current.wipe_barrier == 0
            || (Level::ATTESTED && current.speculation == 0)
            || current.secret_algorithm != generations.secret_algorithm
            || current.wipe_barrier != generations.wipe_barrier
            || (Level::ATTESTED && current.speculation != generations.speculation)
        {
            Err(AssuranceError::StaleGeneration)
        } else {
            Ok(())
        }
    }

    pub(crate) fn revalidate_wipe_snapshot<Level: AssuranceLevel>(
        &self,
        generations: AssuranceGenerations,
    ) -> Result<(), AssuranceError> {
        let current = self.generations();
        if current.wipe_barrier == 0
            || (Level::ATTESTED && current.speculation == 0)
            || current.wipe_barrier != generations.wipe_barrier
            || (Level::ATTESTED && current.speculation != generations.speculation)
        {
            Err(AssuranceError::StaleGeneration)
        } else {
            Ok(())
        }
    }
}

impl<Level: AssuranceLevel> core::fmt::Debug for AssuranceToken<'_, Level> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("AssuranceToken")
            .field(
                "level",
                &if Level::ATTESTED {
                    "attested"
                } else {
                    "best-effort"
                },
            )
            .field("generations", &self.generations)
            .finish_non_exhaustive()
    }
}

fn advance(generation: &AtomicUsize) {
    let _ = generation.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
        Some(if value == 0 {
            0
        } else {
            value.checked_add(1).unwrap_or(0)
        })
    });
}

const fn wipe_posture_is_attestable(posture: WipePosture) -> bool {
    matches!(posture, WipePosture::HardwareFence)
}

const fn speculation_posture_is_attestable(posture: CtGatePosture) -> bool {
    matches!(
        posture,
        CtGatePosture::HardwareSpeculationBarrier
            | CtGatePosture::HardwareSpeculationBarrierBuildAsserted
    )
}
