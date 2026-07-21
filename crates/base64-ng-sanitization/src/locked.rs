use crate::LockedDecodeError;
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

pub(crate) fn admit_locked<T, E>(secret: T, degraded: bool) -> Result<T, LockedDecodeError<E>> {
    if degraded {
        Err(LockedDecodeError::DegradedProtection)
    } else {
        Ok(secret)
    }
}

#[cfg(test)]
mod tests {
    use super::admit_locked;
    use crate::LockedDecodeError;
    use core::cell::Cell;

    struct DropProbe<'a>(&'a Cell<bool>);

    impl Drop for DropProbe<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn degraded_admission_drops_rejected_secret() {
        let dropped = Cell::new(false);
        let result = admit_locked::<_, ()>(DropProbe(&dropped), true);

        assert!(matches!(result, Err(LockedDecodeError::DegradedProtection)));
        assert!(dropped.get());
    }

    #[test]
    fn healthy_admission_returns_secret() {
        let dropped = Cell::new(false);
        let result = admit_locked::<_, ()>(DropProbe(&dropped), false);

        assert!(result.is_ok());
        assert!(!dropped.get());
        drop(result);
        assert!(dropped.get());
    }
}
