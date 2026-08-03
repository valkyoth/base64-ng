#![no_main]

#[path = "support/assurance_provider.rs"]
mod assurance_provider;

use assurance_provider::{Fault, ScheduledProvider};
use base64_ng::{
    STRICT_STANDARD_PADDED,
    assurance::{
        BestEffort, BestEffortProvider, CleanupOutcome, ProtectedMemoryProvider, ProtectedSecret,
        ProviderHealth, ProviderLimits, WipeEvidence,
    },
    secret::SecretInput,
};
use libfuzzer_sys::fuzz_target;

const MAX_OPERATIONS: usize = 256;

fuzz_target!(|data: &[u8]| {
    exercise_default_provider(&data[..data.len().min(MAX_OPERATIONS)]);
    for fault in Fault::ALL {
        exercise_teardown_fault(fault, data);
    }
});

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

fn exercise_default_provider(operations: &[u8]) {
    let context = base64_ng::assurance::AssuranceContext::new();
    let provider = BestEffortProvider::<4>::new(limits()).unwrap();
    let mut live: Vec<ProtectedSecret<'_, _, _, BestEffort>> = Vec::new();
    for operation in operations {
        match operation % 9 {
            0 => {
                let token = context.best_effort_token();
                let length = usize::from(*operation >> 4).saturating_add(1);
                if let Ok(allocation) = ProtectedSecret::try_new(&provider, &token, length) {
                    live.push(allocation);
                }
            }
            1 => {
                if let Some(allocation) = live.pop() {
                    let _ = allocation.try_close();
                }
            }
            2 => {
                drop(live.pop());
            }
            3 => context.invalidate_secret_algorithm(),
            4 => context.invalidate_wipe_barrier(),
            5 => {
                let _ = provider.maintain();
            }
            6 => {
                let _ = provider.restore_health_after_self_check();
            }
            7 => exercise_assured_encode(&context, &provider, operations),
            _ => exercise_assured_decode(&context, &provider),
        }
        assert_provider_bounds(&provider);
    }
    drop(live);
    for _ in 0..4 {
        let _ = provider.maintain();
    }
    assert_provider_bounds(&provider);
}

fn exercise_assured_encode(
    context: &base64_ng::assurance::AssuranceContext,
    provider: &BestEffortProvider<4>,
    input: &[u8],
) {
    let input = &input[..input.len().min(24)];
    let token = context.best_effort_token();
    let required = STRICT_STANDARD_PADDED.encoded_len(input.len()).unwrap();
    let Ok(allocation) = ProtectedSecret::try_new(provider, &token, required) else {
        return;
    };
    if let Ok(secret) =
        STRICT_STANDARD_PADDED.encode_assured(&token, allocation, &SecretInput::new(input))
    {
        let expected = STRICT_STANDARD_PADDED.encode_to_string(input).unwrap();
        assert_eq!(secret.expose_secret().as_bytes(), expected.as_bytes());
        let _ = secret.try_close();
    }
}

fn exercise_assured_decode(
    context: &base64_ng::assurance::AssuranceContext,
    provider: &BestEffortProvider<4>,
) {
    let token = context.best_effort_token();
    let Ok(allocation) = ProtectedSecret::try_new(provider, &token, 6) else {
        return;
    };
    if let Ok(secret) =
        STRICT_STANDARD_PADDED.decode_assured(&token, allocation, &SecretInput::new(b"c2VjcmV0"))
    {
        assert_eq!(secret.expose_secret().as_bytes(), b"secret");
        let _ = secret.try_close();
    }
}

fn assert_provider_bounds(provider: &BestEffortProvider<4>) {
    let report = provider.report();
    let identities = report
        .active_and_reserved
        .checked_add(report.quarantined)
        .and_then(|count| count.checked_add(report.permanently_quarantined))
        .and_then(|count| count.checked_add(report.tombstoned))
        .unwrap();
    assert!(identities <= 4);
    assert!(report.charged_logical_bytes <= 256);
    assert!(report.charged_effective_pages <= 16);
    if report.health == ProviderHealth::Healthy {
        assert_eq!(report.permanently_quarantined, 0);
    }
}

fn exercise_teardown_fault(fault: Fault, data: &[u8]) {
    let context = base64_ng::assurance::AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = ScheduledProvider::new(fault);
    let allocation: ProtectedSecret<'_, _, _, BestEffort> =
        ProtectedSecret::try_new(&provider, &token, 16).unwrap();
    let input = &data[..data.len().min(12)];
    let secret = STRICT_STANDARD_PADDED
        .encode_assured(&token, allocation, &SecretInput::new(input))
        .unwrap();
    let result = secret.try_close();
    match result {
        Ok(report) => {
            assert_eq!(report.outcome, CleanupOutcome::Closed);
            assert_eq!(report.wipe, WipeEvidence::WipedBestEffort);
            provider.assert_terminal_claims(None);
        }
        Err(error) => provider.assert_terminal_claims(Some(&error)),
    }
}
