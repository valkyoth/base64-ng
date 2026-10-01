//! Closed-family RVV boundary. Public routing always uses the exact X60 probe.
use crate::{Standard, UrlSafe, runtime::Backend};

#[cfg(all(test, feature = "std", target_os = "linux"))]
mod tests;

pub(crate) fn width(backend: Backend) -> Option<usize> {
    (backend == Backend::Rvv && super::available()).then_some(16)
}

pub(crate) fn validate(backend: Backend, input: &[u8], url: bool) -> bool {
    width(backend).is_some() && input.len().is_multiple_of(16) && classify(input, url)
}

pub(crate) fn decode(backend: Backend, input: &[u8], output: &mut [u8], url: bool) -> bool {
    if width(backend).is_none()
        || !input.len().is_multiple_of(16)
        || output.len() != input.len() / 4 * 3
    {
        return false;
    }
    decode_available(input, output, url)
}

fn classify(input: &[u8], url: bool) -> bool {
    // SAFETY: Callers prove enabled RVV on this thread. The assembly consumes
    // at most the supplied slice length, including a partial final VL, makes
    // no stores, returns 0/1, and clears its vector registers on both exits.
    unsafe {
        if url {
            super::base64_ng_rvv_validate_url_safe(input.as_ptr(), input.len()) == 1
        } else {
            super::base64_ng_rvv_validate_standard(input.as_ptr(), input.len()) == 1
        }
    }
}

fn decode_available(input: &[u8], output: &mut [u8], url: bool) -> bool {
    // The old packing leaf assumes valid sextets. Keep this safe boundary
    // independently defensive even when its caller already owns a preflight.
    if !classify(input, url) {
        return false;
    }
    if !input.is_empty() {
        // SAFETY: Callers prove RVV, disjoint exact 4:3 spans and full quanta.
        // Classification above proves the closed alphabet before any store.
        unsafe {
            if url {
                super::decode_quanta::<UrlSafe>(
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    input.len() / 4,
                );
            } else {
                super::decode_quanta::<Standard>(
                    input.as_ptr(),
                    output.as_mut_ptr(),
                    input.len() / 4,
                );
            }
        }
    }
    true
}
