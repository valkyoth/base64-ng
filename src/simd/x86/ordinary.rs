//! Checked, closed-family entry points; health admission belongs to the caller.
use crate::runtime::Backend;

pub(crate) fn width(backend: Backend) -> Option<usize> {
    match backend {
        Backend::Avx2 if super::avx2_decode_available() => Some(32),
        Backend::Ssse3Sse41 if super::ssse3_sse41_decode_available() => Some(16),
        _ => None,
    }
}

pub(crate) fn validate(backend: Backend, input: &[u8], url: bool) -> bool {
    let Some(width) = width(backend) else {
        return false;
    };
    if !input.len().is_multiple_of(width) {
        return false;
    }
    // SAFETY: width checked CPU/OS support. Only fixed arrays and the closed
    // Standard/URL-safe families enter the reviewed classifiers. No stores.
    let valid = unsafe {
        if width == 32 {
            if url {
                super::decode_direct::validate_blocks_avx2::<crate::UrlSafe>(
                    input.as_chunks::<32>().0,
                )
            } else {
                super::decode_direct::validate_blocks_avx2::<crate::Standard>(
                    input.as_chunks::<32>().0,
                )
            }
        } else {
            input.as_chunks::<16>().0.iter().all(|block| {
                if url {
                    super::decode_direct::validate_16_bytes_ssse3_sse41::<crate::UrlSafe>(block)
                } else {
                    super::decode_direct::validate_16_bytes_ssse3_sse41::<crate::Standard>(block)
                }
            })
        }
    };
    // SAFETY: The same CPU check covers cleanup, including early rejection.
    unsafe {
        if width == 32 {
            super::cleanup::clear_ymm_registers_after_encode_block();
        } else {
            super::cleanup::clear_xmm_registers_after_encode_block();
        }
    }
    valid
}

pub(crate) fn decode(backend: Backend, input: &[u8], output: &mut [u8], url: bool) -> bool {
    let Some(width) = width(backend) else {
        return false;
    };
    if !input.len().is_multiple_of(width) || output.len() != input.len() / 4 * 3 {
        return false;
    }
    // SAFETY: CPU/OS support and exact input/output geometry were checked.
    // Loops derive arrays from slices; kernels classify before each store.
    unsafe {
        if width == 32 {
            if url {
                decode_avx2::<crate::UrlSafe>(input, output)
            } else {
                decode_avx2::<crate::Standard>(input, output)
            }
        } else if url {
            decode_ssse3::<crate::UrlSafe>(input, output)
        } else {
            decode_ssse3::<crate::Standard>(input, output)
        }
    }
}

#[target_feature(enable = "avx2")]
unsafe fn decode_avx2<A: crate::Alphabet>(input: &[u8], output: &mut [u8]) -> bool {
    let valid = input
        .as_chunks::<32>()
        .0
        .iter()
        .zip(output.as_chunks_mut::<24>().0.iter_mut())
        .all(|(input, output)| {
            // SAFETY: Target features and exact arrays prove kernel preconditions.
            unsafe { super::decode_direct::decode_32_bytes_avx2::<A>(input, output) }
        });
    // SAFETY: All vector results are dead, even on first-block failure.
    unsafe { super::cleanup::clear_ymm_registers_after_encode_block() };
    valid
}

#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn decode_ssse3<A: crate::Alphabet>(input: &[u8], output: &mut [u8]) -> bool {
    let valid = input
        .as_chunks::<16>()
        .0
        .iter()
        .zip(output.as_chunks_mut::<12>().0.iter_mut())
        .all(|(input, output)| {
            // SAFETY: Target features and exact arrays prove kernel preconditions.
            unsafe { super::decode_direct::decode_16_bytes_ssse3_sse41::<A>(input, output) }
        });
    // SAFETY: All vector results are dead, even on first-block failure.
    unsafe { super::cleanup::clear_xmm_registers_after_encode_block() };
    valid
}
