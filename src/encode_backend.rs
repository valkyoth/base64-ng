//! Encode backend dispatch boundary.
//!
//! This module is the single integration point between public encode APIs and
//! the implementation that performs encoding. AVX-512 VBMI, AVX2,
//! SSSE3/SSE4.1, and little-endian `AArch64` NEON encode dispatch is admitted
//! only for Standard/URL-safe alphabet families. `std` uses runtime CPU
//! detection; `no_std` requires complete compile-time target-feature evidence
//! and the atomic health latch. Unsupported alphabets and targets fall back to
//! scalar. In-place encode uses
//! stack staging before entering admitted encode backends so output writes do
//! not overwrite unread input bytes.

#[cfg(test)]
extern crate std;

use crate::{Alphabet, EncodeError, scalar};
#[cfg(feature = "checked-backend")]
mod checked;
mod in_place;
mod policy;
pub(crate) use in_place::encode_in_place;

/// Encode backend currently allowed to execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EncodeBackend {
    /// The audited scalar implementation.
    Scalar,
    /// `x86`/`x86_64` AVX-512 VBMI fixed-block encode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx512Vbmi,
    /// `x86`/`x86_64` AVX2 fixed-block encode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Avx2,
    /// `x86`/`x86_64` SSSE3/SSE4.1 fixed-block encode.
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    Ssse3Sse41,
    /// Little-endian `aarch64` NEON fixed-block encode.
    #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
    Neon,
    /// wasm32 `simd128` fixed-block encode.
    #[cfg(all(feature = "simd", target_arch = "wasm32"))]
    WasmSimd128,
    /// Linux `SpacemiT` X60 RVV 1.0 fixed-block encode.
    #[cfg(all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    ))]
    Rvv,
}

#[cfg(test)]
std::thread_local! {
    static LAST_TEST_EXECUTION: core::cell::Cell<EncodeBackend> = const {
        core::cell::Cell::new(EncodeBackend::Scalar)
    };
}

#[cfg(test)]
pub(crate) fn last_test_execution() -> EncodeBackend {
    LAST_TEST_EXECUTION.with(core::cell::Cell::get)
}

#[cfg(test)]
fn record_test_execution(backend: EncodeBackend) {
    LAST_TEST_EXECUTION.with(|observed| observed.set(backend));
}

#[cfg(not(test))]
const fn record_test_execution(_backend: EncodeBackend) {}

/// Returns the encode backend selected for this build and target.
#[must_use]
#[cfg(any(feature = "simd", test))]
pub(crate) fn active_encode_backend() -> EncodeBackend {
    active_encode_backend_for_input(usize::MAX)
}

pub(crate) fn active_encode_backend_for_input(input_len: usize) -> EncodeBackend {
    let candidate = candidate_encode_backend();
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        policy::select_x86(candidate, input_len, |backend| match backend {
            EncodeBackend::Avx512Vbmi => crate::v2::backend_health::admit(
                crate::runtime::OperationKind::Encode,
                crate::runtime::Backend::Avx512Vbmi,
            ),
            EncodeBackend::Avx2 => {
                crate::simd::avx2_encode_available()
                    && crate::v2::backend_health::admit(
                        crate::runtime::OperationKind::Encode,
                        crate::runtime::Backend::Avx2,
                    )
            }
            EncodeBackend::Ssse3Sse41 => {
                crate::simd::ssse3_sse41_encode_available()
                    && crate::v2::backend_health::admit(
                        crate::runtime::OperationKind::Encode,
                        crate::runtime::Backend::Ssse3Sse41,
                    )
            }
            EncodeBackend::Scalar => false,
        })
    }

    #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
    {
        policy::select_neon(candidate, input_len, |_| {
            crate::v2::backend_health::admit(
                crate::runtime::OperationKind::Encode,
                crate::runtime::Backend::Neon,
            )
        })
    }

    #[cfg(all(
        feature = "std",
        feature = "simd",
        target_arch = "riscv64",
        target_os = "linux"
    ))]
    {
        policy::select_rvv(candidate, input_len, |_| {
            crate::v2::backend_health::admit(
                crate::runtime::OperationKind::Encode,
                crate::runtime::Backend::Rvv,
            )
        })
    }

    #[cfg(not(any(
        all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")),
        all(feature = "simd", target_arch = "aarch64", target_endian = "little"),
        all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        )
    )))]
    {
        let _ = input_len;
        if crate::v2::backend_health::admit(
            crate::runtime::OperationKind::Encode,
            candidate.reported(),
        ) {
            candidate
        } else {
            EncodeBackend::Scalar
        }
    }
}

#[cfg(all(
    test,
    feature = "std",
    feature = "simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
pub(crate) const fn avx512_auto_preferred(input_len: usize) -> bool {
    input_len >= policy::X86_AVX512_MIN_INPUT
}

#[cfg(all(
    any(test, feature = "checked-backend"),
    feature = "simd",
    target_arch = "aarch64",
    target_endian = "little"
))]
pub(crate) const fn neon_auto_preferred(input_len: usize) -> bool {
    input_len >= policy::NEON_MIN_INPUT
}

/// Returns the backend selected by CPU/build policy before health admission.
#[must_use]
pub(crate) fn candidate_encode_backend() -> EncodeBackend {
    #[cfg(feature = "simd")]
    match crate::simd::active_backend() {
        crate::simd::ActiveBackend::Scalar => {}
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx512Vbmi => return EncodeBackend::Avx512Vbmi,
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Avx2 => return EncodeBackend::Avx2,
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        crate::simd::ActiveBackend::Ssse3Sse41 => return EncodeBackend::Ssse3Sse41,
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        crate::simd::ActiveBackend::Neon => return EncodeBackend::Neon,
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        crate::simd::ActiveBackend::WasmSimd128 => return EncodeBackend::WasmSimd128,
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        crate::simd::ActiveBackend::Rvv => return EncodeBackend::Rvv,
    }

    EncodeBackend::Scalar
}

impl EncodeBackend {
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
            #[cfg(all(
                feature = "std",
                feature = "simd",
                target_arch = "riscv64",
                target_os = "linux"
            ))]
            Self::Rvv => crate::runtime::Backend::Rvv,
        }
    }
}

/// Encodes `input` into `output` through the admitted encode backend.
pub(crate) fn encode_slice<A, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError>
where
    A: Alphabet,
{
    if input.len() < policy::MIN_SIMD_INPUT {
        record_test_execution(EncodeBackend::Scalar);
        return scalar::encode_slice::<A, PAD>(input, output);
    }

    let backend = active_encode_backend_for_input(input.len());
    #[cfg(feature = "checked-backend")]
    if backend != EncodeBackend::Scalar && backend_supports::<A>(backend, input.len()) {
        record_test_execution(backend);
        return checked::encode::<A, PAD>(backend.reported(), input, output);
    }

    match backend {
        EncodeBackend::Scalar => {
            record_test_execution(EncodeBackend::Scalar);
            scalar::encode_slice::<A, PAD>(input, output)
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx512Vbmi => {
            if input.len() >= 48 && crate::simd::avx512_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_avx512::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx2 => {
            if input.len() >= 24 && crate::simd::avx2_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_avx2::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Ssse3Sse41 => {
            if input.len() >= 12 && crate::simd::ssse3_sse41_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        EncodeBackend::Neon => {
            if input.len() >= 12 && crate::simd::neon_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_neon::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EncodeBackend::WasmSimd128 => {
            if input.len() >= 12 && crate::simd::wasm_simd128_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_wasm_simd128::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        EncodeBackend::Rvv => {
            if input.len() >= 12 && crate::simd::rvv_supports_alphabet::<A>() {
                record_test_execution(backend);
                crate::simd::encode_slice_rvv::<A, PAD>(input, output)
            } else {
                record_test_execution(EncodeBackend::Scalar);
                scalar::encode_slice::<A, PAD>(input, output)
            }
        }
    }
}

#[cfg(all(
    feature = "checked-backend",
    any(
        target_arch = "x86",
        target_arch = "x86_64",
        all(target_arch = "aarch64", target_endian = "little")
    )
))]
pub(crate) fn encode_checked<A: Alphabet, const PAD: bool>(
    backend: crate::runtime::Backend,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    checked::encode::<A, PAD>(backend, input, output)
}

#[cfg(feature = "checked-backend")]
fn backend_supports<A: Alphabet>(backend: EncodeBackend, input_len: usize) -> bool {
    let _ = core::marker::PhantomData::<A>;
    let _ = input_len;
    match backend {
        EncodeBackend::Scalar => false,
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx512Vbmi => {
            input_len >= 48 && crate::simd::avx512_supports_alphabet::<A>()
        }
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Avx2 => input_len >= 24 && crate::simd::avx2_supports_alphabet::<A>(),
        #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
        EncodeBackend::Ssse3Sse41 => {
            input_len >= 12 && crate::simd::ssse3_sse41_supports_alphabet::<A>()
        }
        #[cfg(all(feature = "simd", target_arch = "aarch64", target_endian = "little"))]
        EncodeBackend::Neon => {
            neon_auto_preferred(input_len) && crate::simd::neon_supports_alphabet::<A>()
        }
        #[cfg(all(feature = "simd", target_arch = "wasm32"))]
        EncodeBackend::WasmSimd128 => {
            input_len >= 12 && crate::simd::wasm_simd128_supports_alphabet::<A>()
        }
        #[cfg(all(
            feature = "std",
            feature = "simd",
            target_arch = "riscv64",
            target_os = "linux"
        ))]
        EncodeBackend::Rvv => {
            input_len >= policy::RVV_MIN_INPUT && crate::simd::rvv_supports_alphabet::<A>()
        }
    }
}
