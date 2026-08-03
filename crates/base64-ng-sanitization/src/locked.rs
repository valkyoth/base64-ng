use sanitization::{ForkProtectionRequest, ProtectionRequest, Requirement};

#[must_use]
pub(crate) const fn required_secret_protection() -> ProtectionRequest {
    ProtectionRequest {
        memory_lock: Requirement::Required,
        dump_exclusion: Requirement::Required,
        fork: ForkProtectionRequest::exclude(Requirement::Required),
        guard_pages: Requirement::NotRequested,
        canary: required_canary_protection(),
        cache_policy: Requirement::NotRequested,
    }
}

#[cfg(feature = "canary-check")]
const fn required_canary_protection() -> Requirement {
    Requirement::Required
}

#[cfg(not(feature = "canary-check"))]
const fn required_canary_protection() -> Requirement {
    Requirement::NotRequested
}
