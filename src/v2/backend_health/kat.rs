//! Direct backend known-answer tests. These calls never enter dispatch.

use crate::runtime::{Backend, OperationKind};
use crate::{Standard, UrlSafe};

const INPUT: [u8; 48] = [
    0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00,
    0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80,
    0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33,
];
const STANDARD_ENCODED: &[u8; 64] =
    b"+//vABCDf4BAVaoz+//vABCDf4BAVaoz+//vABCDf4BAVaoz+//vABCDf4BAVaoz";
const URL_SAFE_ENCODED: &[u8; 64] =
    b"-__vABCDf4BAVaoz-__vABCDf4BAVaoz-__vABCDf4BAVaoz-__vABCDf4BAVaoz";

pub(super) fn available(backend: Backend) -> bool {
    match backend {
        Backend::Scalar => true,
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx512Vbmi => crate::simd::avx512_decode_available(),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx2 => crate::simd::avx2_decode_available(),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Ssse3Sse41 => crate::simd::ssse3_sse41_decode_available(),
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        Backend::Neon => crate::simd::neon_available(),
        #[cfg(target_arch = "wasm32")]
        Backend::WasmSimd128 => crate::simd::wasm_simd128_decode_available(),
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

pub(super) fn run(operation: OperationKind, backend: Backend) -> bool {
    match operation {
        OperationKind::Encode => encode(backend),
        OperationKind::StrictDecode => decode(backend),
        OperationKind::SecretDecode => false,
    }
}

fn encode(backend: Backend) -> bool {
    let mut standard = [0u8; 64];
    let mut url_safe = [0u8; 64];
    direct_encode::<Standard, false>(backend, &INPUT, &mut standard) == Some(64)
        && direct_encode::<UrlSafe, false>(backend, &INPUT, &mut url_safe) == Some(64)
        && standard == *STANDARD_ENCODED
        && url_safe == *URL_SAFE_ENCODED
}

fn decode(backend: Backend) -> bool {
    let mut standard = [0u8; 48];
    let mut url_safe = [0u8; 48];
    direct_decode::<Standard, false>(backend, STANDARD_ENCODED, &mut standard) == Some(48)
        && direct_decode::<UrlSafe, false>(backend, URL_SAFE_ENCODED, &mut url_safe) == Some(48)
        && standard == INPUT
        && url_safe == INPUT
}

pub(crate) fn direct_encode<A: crate::Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Option<usize> {
    let result = match backend {
        Backend::Scalar => crate::scalar::encode_slice::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx512Vbmi => crate::simd::encode_slice_avx512::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx2 => crate::simd::encode_slice_avx2::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Ssse3Sse41 => crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output),
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        Backend::Neon => crate::simd::encode_slice_neon::<A, PAD>(input, output),
        #[cfg(target_arch = "wasm32")]
        Backend::WasmSimd128 => crate::simd::encode_slice_wasm_simd128::<A, PAD>(input, output),
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    result.ok()
}

pub(crate) fn direct_decode<A: crate::Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Option<usize> {
    let result = match backend {
        Backend::Scalar => crate::scalar::decode_slice::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx512Vbmi => crate::simd::decode_slice_avx512::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx2 => crate::simd::decode_slice_avx2::<A, PAD>(input, output),
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Ssse3Sse41 => crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output),
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        Backend::Neon => crate::simd::decode_slice_neon::<A, PAD>(input, output),
        #[cfg(target_arch = "wasm32")]
        Backend::WasmSimd128 => crate::simd::decode_slice_wasm_simd128::<A, PAD>(input, output),
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    result.ok()
}
