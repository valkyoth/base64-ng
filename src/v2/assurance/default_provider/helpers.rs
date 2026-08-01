//! Checked arithmetic helpers for the bounded default provider.

use super::{ProtectionError, ProviderReport, ProviderState, ResourceKind, SlotLifecycle};

pub(super) fn report_from_state<const SLOTS: usize>(
    state: &ProviderState<SLOTS>,
) -> ProviderReport {
    let mut active_and_reserved = 0;
    let mut quarantined = 0;
    let mut charged_logical_bytes = 0;
    let mut charged_effective_pages = 0;
    for slot in &state.slots {
        match slot.lifecycle {
            SlotLifecycle::Free => continue,
            SlotLifecycle::Reserved | SlotLifecycle::Active => active_and_reserved += 1,
            SlotLifecycle::Quarantined | SlotLifecycle::PermanentlyQuarantined => {
                quarantined += 1;
            }
        }
        charged_logical_bytes += slot.logical_bytes;
        charged_effective_pages += slot.effective_pages;
    }
    ProviderReport {
        health: state.health,
        health_generation: state.health_generation,
        protection_generation: state.protection_generation,
        active_and_reserved,
        quarantined,
        tombstoned: 0,
        charged_logical_bytes,
        charged_effective_pages,
    }
}

pub(super) fn check_limit(
    current: usize,
    additional: usize,
    maximum: usize,
    resource: ResourceKind,
) -> Result<(), ProtectionError> {
    if current
        .checked_add(additional)
        .is_none_or(|total| total > maximum)
    {
        Err(ProtectionError::ProtectionResourceExhausted(resource))
    } else {
        Ok(())
    }
}

pub(super) fn effective_pages(
    address: usize,
    len: usize,
    page_size: usize,
) -> Result<usize, ProtectionError> {
    if len == 0 {
        return Ok(0);
    }
    let offset = address % page_size;
    offset
        .checked_add(len)
        .and_then(|span| span.checked_add(page_size - 1))
        .map(|span| span / page_size)
        .ok_or(ProtectionError::LengthOverflow)
}

pub(super) fn next_generation(value: usize) -> usize {
    value.checked_add(1).unwrap_or(0)
}
