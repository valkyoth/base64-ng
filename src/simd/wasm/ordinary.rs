//! Closed-family WASM boundary; health admission belongs to the caller.
use super::{Alphabet, Standard, direct};
use crate::runtime::Backend;

#[cfg(all(test, feature = "std"))]
mod tests;

pub(crate) fn width(backend: Backend) -> Option<usize> {
    (backend == Backend::WasmSimd128 && super::wasm_simd128_decode_available()).then_some(16)
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
    for block in input.as_chunks::<16>().0 {
        // SAFETY: Entry proves simd128 and closes the alphabet family. The
        // fixed array proves the complete load. No output is supplied.
        if !unsafe { direct::validate_16_bytes_wasm::<A>(block) } {
            return false;
        }
    }
    true
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
    for (input, output) in input
        .as_chunks::<16>()
        .0
        .iter()
        .zip(output.as_chunks_mut::<12>().0)
    {
        // SAFETY: Entry proves simd128, closed alphabet and exact 16:12 geometry.
        // The kernel rejects before stores within each block. The caller owns
        // whole-input preflight and recovery after any partial multi-block write.
        if !unsafe { direct::decode_16_bytes::<A>(input, output) } {
            return false;
        }
    }
    true
}
