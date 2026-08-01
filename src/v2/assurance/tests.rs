#![cfg(feature = "alloc")]

use super::*;
use crate::{STRICT_STANDARD_PADDED, secret::SecretInput};

fn limits() -> ProviderLimits {
    ProviderLimits {
        max_identities: 4,
        max_logical_bytes: 256,
        max_effective_pages: 16,
        max_registry_entries: 4,
        max_retry_attempts: 2,
        max_maintenance_work: 2,
        page_size: 64,
    }
}

#[test]
fn protected_direct_decode_and_encode_require_token_and_allocation() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<4>::new(limits()).unwrap();

    let allocation = ProtectedSecret::try_new(&provider, &token, 6).unwrap();
    let decoded = STRICT_STANDARD_PADDED
        .decode_assured(&token, allocation, &SecretInput::new(b"c2VjcmV0"))
        .unwrap();
    assert_eq!(decoded.expose_secret().as_bytes(), b"secret");
    assert_eq!(decoded.try_close().unwrap().outcome, CleanupOutcome::Closed);

    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let encoded = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");
    encoded.try_close().unwrap();
    let report = provider.report();
    assert_eq!(report.active_and_reserved, 0);
    assert_eq!(report.charged_logical_bytes, 0);
}

#[test]
fn ordinary_backend_generation_does_not_stale_secret_token() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    context.invalidate_ordinary_backend();
    assert!(token.revalidate().is_ok());

    context.invalidate_secret_algorithm();
    assert_eq!(token.revalidate(), Err(AssuranceError::StaleGeneration));
}

#[test]
fn token_report_keeps_all_context_generations_independent() {
    let context = AssuranceContext::new();
    let first = context.best_effort_token().report().snapshot();
    context.invalidate_ordinary_backend();
    let second = context.best_effort_token().report().snapshot();
    assert_ne!(
        first.ordinary_backend_generation,
        second.ordinary_backend_generation
    );
    assert_eq!(
        first.secret_algorithm_generation,
        second.secret_algorithm_generation
    );
    assert_eq!(
        first.wipe_barrier_generation,
        second.wipe_barrier_generation
    );
    assert_eq!(first.speculation_generation, second.speculation_generation);
    assert_eq!(
        second.secret_decode_backend.backend,
        "scalar-constant-time-oriented"
    );
    assert_eq!(second.attestation_posture, "not-attested");
    assert_eq!(second.secret_policy_posture, "best-effort");
}

#[test]
fn stale_secret_algorithm_does_not_invalidate_completed_wipe_evidence() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<4>::new(limits()).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();

    context.invalidate_secret_algorithm();
    let report = validated.try_close().unwrap();
    assert_eq!(report.wipe, WipeEvidence::WipedBestEffort);
    assert_eq!(report.outcome, CleanupOutcome::Closed);
}

#[test]
fn provider_budget_is_reserved_before_plaintext_storage() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let mut constrained = limits();
    constrained.max_identities = 1;
    constrained.max_registry_entries = 1;
    let provider = BestEffortProvider::<1>::new(constrained).unwrap();
    let first = ProtectedSecret::try_new(&provider, &token, 32).unwrap();
    assert_eq!(
        ProtectedSecret::try_new(&provider, &token, 32).unwrap_err(),
        ProtectionError::ProtectionResourceExhausted(ResourceKind::Identities),
    );
    first.try_close().unwrap();
}

#[test]
fn stale_generation_consumes_and_closes_unvalidated_storage() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let mut provider_limits = limits();
    provider_limits.max_identities = 2;
    provider_limits.max_registry_entries = 2;
    let provider = BestEffortProvider::<2>::new(provider_limits).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    context.invalidate_wipe_barrier();
    let error = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap_err();
    assert_eq!(
        error,
        AssuredEncodeError::Assurance(AssuranceError::StaleGeneration)
    );
    assert_eq!(provider.report().active_and_reserved, 0);
}

#[test]
fn protected_debug_and_cleanup_errors_are_redacted() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let mut provider_limits = limits();
    provider_limits.max_identities = 1;
    provider_limits.max_registry_entries = 1;
    let provider = BestEffortProvider::<1>::new(provider_limits).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let rendered = format!("{allocation:?}");
    assert!(rendered.contains("<redacted>"));
    assert!(!rendered.contains("0x"));
}

#[test]
fn bounded_recovery_exhaustion_retains_quarantine_and_shuts_down() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let mut provider_limits = limits();
    provider_limits.max_identities = 1;
    provider_limits.max_registry_entries = 1;
    provider_limits.max_retry_attempts = 1;
    let provider = BestEffortProvider::<1>::new(provider_limits).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    context.invalidate_wipe_barrier();
    drop(allocation);
    assert_eq!(provider.report().quarantined, 1);
    assert_eq!(provider.maintain(), 1);
    assert_eq!(provider.report().health, ProviderHealth::Shutdown);
    assert_eq!(provider.report().quarantined, 0);
    assert_eq!(provider.report().permanently_quarantined, 1);
    let snapshot = provider.report().snapshot();
    assert_eq!(snapshot.health, "shutdown");
    assert_eq!(snapshot.permanently_quarantined, 1);
}

#[test]
fn assert_unwind_safe_bypass_still_runs_cleanup_for_each_typestate() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<4>::new(limits()).unwrap();

    let uninitialized = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(uninitialized);
        panic!("reviewed uninitialized cleanup test");
    }));
    assert!(result.is_err());

    let uninitialized = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let unvalidated = uninitialized
        .begin_unvalidated(&token, super::SecretOperation::Decode)
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(unvalidated);
        panic!("reviewed unvalidated cleanup test");
    }));
    assert!(result.is_err());

    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    let validated = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(b"secret"))
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(validated);
        panic!("reviewed validated cleanup test");
    }));
    assert!(result.is_err());
    assert_eq!(provider.report().active_and_reserved, 0);
}

#[allow(dead_code)]
fn protected_send_is_conditional_on_the_sealed_provider_proof<P>()
where
    P: ThreadMovableProvider + Sync,
    P::Handle: Send,
{
    fn assert_send<T: Send>() {}
    assert_send::<ProtectedSecret<'_, P, Validated, BestEffort>>();
}
