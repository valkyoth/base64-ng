//! Decode backend dispatch boundary.
//!
//! This module mirrors the encode backend boundary. Decode acceleration is
//! admitted only for the backends and surfaces named here; future SIMD decode
//! admission must update this boundary together with canonicality, error-shape,
//! fallback, retention, timing, and release evidence.

#[cfg(test)]
extern crate std;

use crate::{Alphabet, DecodeError, scalar};
#[cfg(feature = "checked-backend")]
mod checked;

/// Decode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// `x86`/`x86_64` AVX-512 VBMI fixed-block strict decode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx512Vbmi,
    /// `x86`/`x86_64` AVX2 fixed-block strict decode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx2,
    /// `x86`/`x86_64` SSSE3/SSE4.1 fixed-block strict decode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Ssse3Sse41,
    /// Little-endian `aarch64` NEON fixed-block strict decode.
    #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
    Neon,
    /// wasm32 `simd128` fixed-block strict decode.
    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    WasmSimd128,
}

#[cfg(test)]
std::thread_local! {
    static LAST_TEST_EXECUTION: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn last_test_execution() -> DecodeBackend {
    if LAST_TEST_EXECUTION.with(core::cell::Cell::get) {
        active_decode_backend()
    } else {
        DecodeBackend::Scalar
    }
}

#[cfg(test)]
fn record_test_execution(accelerated: bool) {
    LAST_TEST_EXECUTION.with(|observed| observed.set(accelerated));
}

#[cfg(not(test))]
const fn record_test_execution(_accelerated: bool) {}

/// Returns the decode backend selected for this build and target.
#[must_use]
pub(crate) fn active_decode_backend() -> DecodeBackend {
    let backend = candidate_decode_backend();
    if crate::v2::backend_health::admit(
        crate::runtime::OperationKind::StrictDecode,
        backend.reported(),
    ) {
        backend
    } else {
        DecodeBackend::Scalar
    }
}

/// Returns the backend selected by CPU/build policy before health admission.
#[must_use]
pub(crate) fn candidate_decode_backend() -> DecodeBackend {
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        if crate::simd::avx512_decode_available() {
            return DecodeBackend::Avx512Vbmi;
        }

        if crate::simd::avx2_decode_available() {
            return DecodeBackend::Avx2;
        }

        if crate::simd::ssse3_sse41_decode_available() {
            return DecodeBackend::Ssse3Sse41;
        }
    }

    #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
    {
        if crate::simd::neon_available() {
            return DecodeBackend::Neon;
        }
    }

    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    {
        if crate::simd::wasm_simd128_decode_available() {
            return DecodeBackend::WasmSimd128;
        }
    }

    DecodeBackend::Scalar
}

impl DecodeBackend {
    pub(crate) const fn reported(self) -> crate::runtime::Backend {
        match self {
            Self::Scalar => crate::runtime::Backend::Scalar,
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Avx512Vbmi => crate::runtime::Backend::Avx512Vbmi,
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Avx2 => crate::runtime::Backend::Avx2,
            #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
            Self::Ssse3Sse41 => crate::runtime::Backend::Ssse3Sse41,
            #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
            Self::Neon => crate::runtime::Backend::Neon,
            #[cfg(all(feature = "simd", target_arch = "wasm32"))]
            Self::WasmSimd128 => crate::runtime::Backend::WasmSimd128,
        }
    }
}

/// Decodes `input` into `output` through the admitted strict decode backend.
///
/// Normal strict `Engine::decode_slice` enters this boundary directly.
/// `Engine::decode_slice_wrapped` also enters after strict line-profile
/// validation and scalar line-ending compaction. `Engine::decode_slice_legacy`
/// enters after strict legacy validation and scalar whitespace compaction.
/// In-place decode enters after strict validation and stack staging. `ct`
/// secret decode remains a separate scalar surface unless a future admission
/// package explicitly adds and tests it.
pub(crate) fn decode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError>
where
    A: Alphabet,
{
    let backend = active_decode_backend();
    #[cfg(feature = "checked-backend")]
    if backend != DecodeBackend::Scalar && backend_supports::<A>(backend) {
        record_test_execution(true);
        return checked::decode::<A, PAD>(backend.reported(), input, output);
    }

    match backend {
        DecodeBackend::Scalar => {
            record_test_execution(false);
            scalar::decode_slice::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Avx512Vbmi => {
            record_test_execution(true);
            crate::simd::decode_slice_avx512::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Avx2 => {
            record_test_execution(true);
            crate::simd::decode_slice_avx2::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Ssse3Sse41 => {
            record_test_execution(true);
            crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        DecodeBackend::Neon => {
            record_test_execution(true);
            crate::simd::decode_slice_neon::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        DecodeBackend::WasmSimd128 => {
            record_test_execution(true);
            crate::simd::decode_slice_wasm_simd128::<A, PAD>(input, output)
        }
    }
}

#[cfg(feature = "checked-backend")]
fn backend_supports<A: Alphabet>(backend: DecodeBackend) -> bool {
    match backend {
        DecodeBackend::Scalar => false,
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Avx512Vbmi => crate::simd::avx512_supports_alphabet::<A>(),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Avx2 => crate::simd::avx2_supports_alphabet::<A>(),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        DecodeBackend::Ssse3Sse41 => crate::simd::ssse3_sse41_supports_alphabet::<A>(),
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        DecodeBackend::Neon => crate::simd::neon_supports_alphabet::<A>(),
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        DecodeBackend::WasmSimd128 => crate::simd::wasm_simd128_supports_alphabet::<A>(),
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeBackend, active_decode_backend};

    #[test]
    fn boundary_uses_only_admitted_backends() {
        let backend = active_decode_backend();
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        if backend == DecodeBackend::Avx512Vbmi {
            assert!(crate::simd::avx512_decode_available());
            return;
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        if backend == DecodeBackend::Avx2 {
            assert!(crate::simd::avx2_decode_available());
            return;
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        if backend == DecodeBackend::Ssse3Sse41 {
            assert!(crate::simd::ssse3_sse41_decode_available());
            return;
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        if backend == DecodeBackend::Neon {
            assert!(crate::simd::neon_available());
            return;
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        if backend == DecodeBackend::WasmSimd128 {
            assert!(crate::simd::wasm_simd128_decode_available());
            return;
        }
        assert_eq!(backend, DecodeBackend::Scalar);
    }
}
