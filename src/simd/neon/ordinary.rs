//! Closed-family NEON boundary; health admission belongs to the caller.
use super::{Alphabet, Standard, direct, neon_available};
use crate::runtime::Backend;

#[cfg(all(test, feature = "std", target_os = "linux"))]
mod tests;

pub(crate) fn width(backend: Backend) -> Option<usize> {
    (backend == Backend::Neon && neon_available()).then_some(16)
}

pub(crate) fn validate(backend: Backend, input: &[u8], url_safe: bool) -> bool {
    if width(backend).is_none() || !input.len().is_multiple_of(16) {
        return false;
    }
    if url_safe {
        validate_family::<crate::UrlSafe>(input)
    } else {
        validate_family::<Standard>(input)
    }
}

fn validate_family<A: Alphabet>(input: &[u8]) -> bool {
    let mut valid = true;
    for block in input.as_chunks::<16>().0 {
        // SAFETY: Entry checks NEON and closes the alphabet family. The fixed
        // array proves the exact load. No output is supplied or written.
        if !unsafe { direct::validate_16_bytes_neon::<A>(block) } {
            valid = false;
            break;
        }
    }
    // SAFETY: Entry proves NEON. Clear after both acceptance and rejection.
    unsafe { clear_neon_registers_after_vector_block!() };
    valid
}

pub(crate) fn decode(backend: Backend, input: &[u8], output: &mut [u8], url_safe: bool) -> bool {
    if width(backend).is_none()
        || !input.len().is_multiple_of(16)
        || output.len() != input.len() / 4 * 3
    {
        return false;
    }
    if url_safe {
        decode_family::<crate::UrlSafe>(input, output)
    } else {
        decode_family::<Standard>(input, output)
    }
}

fn decode_family<A: Alphabet>(input: &[u8], output: &mut [u8]) -> bool {
    let mut valid = true;
    for (input, output) in input
        .as_chunks::<16>()
        .0
        .iter()
        .zip(output.as_chunks_mut::<12>().0)
    {
        // SAFETY: Entry proves NEON, closed alphabet, and exact geometry.
        if !unsafe { direct::decode_16_bytes::<A>(input, output) } {
            valid = false;
            break;
        }
    }
    // SAFETY: Entry proves NEON; no computed vectors remain live.
    unsafe { clear_neon_registers_after_vector_block!() };
    valid
}
