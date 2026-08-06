//! Direct backend known-answer tests. These calls never enter dispatch.

use crate::runtime::{Backend, OperationKind};
use crate::{Standard, UrlSafe};

const INPUT: [u8; 48] = [
    0x00, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8b, 0x30, 0xd3, 0x8f, 0x41, 0x14, 0x93, 0x51,
    0x55, 0x97, 0x61, 0x96, 0x9b, 0x71, 0xd7, 0x9f, 0x82, 0x18, 0xa3, 0x92, 0x59, 0xa7, 0xa2, 0x9a,
    0xab, 0xb2, 0xdb, 0xaf, 0xc3, 0x1c, 0xb3, 0xd3, 0x5d, 0xb7, 0xe3, 0x9e, 0xbb, 0xf3, 0xdf, 0xbf,
];
const STANDARD_ENCODED: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_ENCODED: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BOUNDARY_INPUT: [u8; 48] = [
    0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff,
    0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff,
    0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0xff,
];
const BOUNDARY_STANDARD: &[u8; 64] =
    b"AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/";
const BOUNDARY_URL_SAFE: &[u8; 64] =
    b"AAAA_wAAAP8AAAD_AAAA_wAAAP8AAAD_AAAA_wAAAP8AAAD_AAAA_wAAAP8AAAD_";

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
        #[cfg(all(feature = "std", target_arch = "riscv64", target_os = "linux"))]
        Backend::Rvv => crate::simd::rvv_available(),
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
    encode_matches::<Standard>(backend, &INPUT, STANDARD_ENCODED)
        && encode_matches::<UrlSafe>(backend, &INPUT, URL_SAFE_ENCODED)
        && encode_matches::<Standard>(backend, &BOUNDARY_INPUT, BOUNDARY_STANDARD)
        && encode_matches::<UrlSafe>(backend, &BOUNDARY_INPUT, BOUNDARY_URL_SAFE)
}

fn decode(backend: Backend) -> bool {
    decode_matches::<Standard>(backend, STANDARD_ENCODED, &INPUT)
        && decode_matches::<UrlSafe>(backend, URL_SAFE_ENCODED, &INPUT)
        && decode_matches::<Standard>(backend, BOUNDARY_STANDARD, &BOUNDARY_INPUT)
        && decode_matches::<UrlSafe>(backend, BOUNDARY_URL_SAFE, &BOUNDARY_INPUT)
}

fn encode_matches<A: crate::Alphabet>(
    backend: Backend,
    input: &[u8; 48],
    expected: &[u8; 64],
) -> bool {
    let mut output = [0u8; 64];
    direct_encode::<A, false>(backend, input, &mut output) == Some(64) && output == *expected
}

fn decode_matches<A: crate::Alphabet>(
    backend: Backend,
    input: &[u8; 64],
    expected: &[u8; 48],
) -> bool {
    let mut output = [0u8; 48];
    direct_decode::<A, false>(backend, input, &mut output) == Some(48) && output == *expected
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
        #[cfg(all(feature = "std", target_arch = "riscv64", target_os = "linux"))]
        Backend::Rvv => crate::simd::encode_slice_rvv::<A, PAD>(input, output),
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
        #[cfg(all(feature = "std", target_arch = "riscv64", target_os = "linux"))]
        Backend::Rvv => crate::simd::decode_slice_rvv::<A, PAD>(input, output),
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    result.ok()
}
