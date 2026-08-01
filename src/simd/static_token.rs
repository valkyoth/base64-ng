//! Static SIMD capability for deployments without runtime CPU probing.

use core::marker::PhantomData;

use crate::runtime::{Backend, OperationKind};
use crate::{Alphabet, DecodeError, EncodeError, Standard, UrlSafe};

/// Non-forgeable, thread-bound proof that a static SIMD backend passed its KAT.
///
/// The token bypasses runtime CPU probing only. It does not bypass bounds,
/// canonicality, backend health, quarantine, or reporting. Kernel operations
/// are added to this contract by the architecture admission commits that
/// follow Commit 24.
///
/// The raw-pointer marker deliberately makes this value neither `Send` nor
/// `Sync`; deployment evidence applies to the thread that constructed it.
///
/// ```compile_fail
/// let token = base64_ng::StaticBackendToken::for_compiled_target().unwrap();
/// std::thread::spawn(move || drop(token));
/// ```
#[derive(Debug)]
pub struct StaticBackendToken {
    backend: Backend,
    generation: usize,
    _thread_bound: PhantomData<*mut ()>,
}

impl StaticBackendToken {
    /// Selects the strongest backend proven by compile-time target features.
    ///
    /// Returns `None` when the build lacks a complete feature bundle,
    /// pointer-width atomics, or a passing known-answer test.
    #[must_use]
    pub fn for_compiled_target() -> Option<Self> {
        let backend = compiled_backend()?;
        Self::admit(backend, false)
    }

    /// Constructs a token from deployment-supplied backend evidence.
    ///
    /// # Safety
    ///
    /// Before calling, the deployment must prove that this thread's CPU and OS
    /// vector state support every feature returned by
    /// [`Backend::required_cpu_features`], that the ABI preserves the required
    /// vector state, and that migration cannot move this thread to an
    /// incompatible CPU. A false attestation can execute an unsupported
    /// instruction during the mandatory KAT and terminate the process.
    #[must_use]
    pub unsafe fn assume_supported(backend: Backend) -> Option<Self> {
        if backend_matches_target(backend) {
            Self::admit(backend, true)
        } else {
            None
        }
    }

    /// Returns the exact backend represented by this token.
    #[must_use]
    pub const fn backend(&self) -> Backend {
        self.backend
    }

    /// Returns the health generation borrowed by this token.
    #[must_use]
    pub const fn health_generation(&self) -> usize {
        self.generation
    }

    /// Returns whether quarantine and generation state still validate it.
    ///
    /// This is an admission snapshot, not synchronous cancellation. An
    /// invocation that already observed a healthy generation may finish while
    /// another thread quarantines that backend.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let encode = crate::v2::backend_health::snapshot(OperationKind::Encode, self.backend);
        let decode = crate::v2::backend_health::snapshot(OperationKind::StrictDecode, self.backend);
        encode.state == crate::BackendHealthState::Healthy
            && decode.state == crate::BackendHealthState::Healthy
            && encode.generation == self.generation
            && decode.generation == self.generation
    }

    /// Encodes with the statically admitted Standard-alphabet backend.
    ///
    /// Commits 25 and 26 enable direct SSSE3/SSE4.1, AVX2, and AVX-512 VBMI
    /// execution. Other token backends, or a token invalidated by quarantine,
    /// use the scalar encoder.
    /// The `checked-backend` feature applies the same per-call redundant
    /// scalar comparison and quarantine policy as automatic dispatch.
    pub fn encode_standard<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        self.encode::<Standard, PAD>(input, output)
    }

    /// Encodes with the statically admitted URL-safe-alphabet backend.
    ///
    /// Commits 25 and 26 enable direct SSSE3/SSE4.1, AVX2, and AVX-512 VBMI
    /// execution. Other token backends, or a token invalidated by quarantine,
    /// use the scalar encoder.
    /// The `checked-backend` feature applies the same per-call redundant
    /// scalar comparison and quarantine policy as automatic dispatch.
    pub fn encode_url_safe<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        self.encode::<UrlSafe, PAD>(input, output)
    }

    /// Decodes strict Standard Base64 with the statically admitted backend.
    ///
    /// Commit 27 enables direct SSSE3/SSE4.1 and AVX2 execution. AVX-512 and
    /// non-x86 token backends remain scalar here until their own rewrite
    /// commits admit the static decode operation. Invalid input retains the
    /// ordinary strict decoder's exact diagnostics and does not modify output.
    pub fn decode_standard<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        self.decode::<Standard, PAD>(input, output)
    }

    /// Decodes strict URL-safe Base64 with the statically admitted backend.
    ///
    /// Commit 27 enables direct SSSE3/SSE4.1 and AVX2 execution. AVX-512 and
    /// non-x86 token backends remain scalar here until their own rewrite
    /// commits admit the static decode operation. Invalid input retains the
    /// ordinary strict decoder's exact diagnostics and does not modify output.
    pub fn decode_url_safe<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        self.decode::<UrlSafe, PAD>(input, output)
    }

    fn encode<A: Alphabet, const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        if !self.is_valid() {
            return crate::scalar::encode_slice::<A, PAD>(input, output);
        }
        #[cfg(all(
            feature = "checked-backend",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        match self.backend {
            Backend::Avx512Vbmi | Backend::Avx2 | Backend::Ssse3Sse41 => {
                return crate::encode_backend::encode_checked::<A, PAD>(
                    self.backend,
                    input,
                    output,
                );
            }
            _ => {}
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        match self.backend {
            Backend::Avx512Vbmi => {
                return crate::simd::encode_slice_avx512::<A, PAD>(input, output);
            }
            Backend::Avx2 => return crate::simd::encode_slice_avx2::<A, PAD>(input, output),
            Backend::Ssse3Sse41 => {
                return crate::simd::encode_slice_ssse3_sse41::<A, PAD>(input, output);
            }
            _ => {}
        }
        crate::scalar::encode_slice::<A, PAD>(input, output)
    }

    fn decode<A: Alphabet, const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        if !self.is_valid() {
            return crate::scalar::decode_slice::<A, PAD>(input, output);
        }
        #[cfg(all(
            feature = "checked-backend",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        match self.backend {
            Backend::Avx2 | Backend::Ssse3Sse41 => {
                return crate::decode_backend::decode_checked::<A, PAD>(
                    self.backend,
                    input,
                    output,
                );
            }
            _ => {}
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        match self.backend {
            Backend::Avx2 => return crate::simd::decode_slice_avx2::<A, PAD>(input, output),
            Backend::Ssse3Sse41 => {
                return crate::simd::decode_slice_ssse3_sse41::<A, PAD>(input, output);
            }
            _ => {}
        }
        crate::scalar::decode_slice::<A, PAD>(input, output)
    }

    fn admit(backend: Backend, deployment_attested: bool) -> Option<Self> {
        let admit_operation = |operation| {
            if deployment_attested {
                crate::v2::backend_health::admit_deployment_attested(operation, backend)
            } else {
                crate::v2::backend_health::admit(operation, backend)
            }
        };
        if !cfg!(target_has_atomic = "ptr")
            || !admit_operation(OperationKind::Encode)
            || !admit_operation(OperationKind::StrictDecode)
        {
            return None;
        }
        let encode = crate::v2::backend_health::snapshot(OperationKind::Encode, backend);
        let decode = crate::v2::backend_health::snapshot(OperationKind::StrictDecode, backend);
        if encode.generation != decode.generation {
            return None;
        }
        Some(Self {
            backend,
            generation: encode.generation,
            _thread_bound: PhantomData,
        })
    }
}

fn compiled_backend() -> Option<Backend> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if cfg!(all(
            target_feature = "avx512f",
            target_feature = "avx512bw",
            target_feature = "avx512vl",
            target_feature = "avx512vbmi"
        )) {
            return Some(Backend::Avx512Vbmi);
        }
        if cfg!(target_feature = "avx2") {
            return Some(Backend::Avx2);
        }
        if cfg!(all(target_feature = "ssse3", target_feature = "sse4.1")) {
            return Some(Backend::Ssse3Sse41);
        }
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    if cfg!(target_feature = "neon") {
        return Some(Backend::Neon);
    }
    #[cfg(target_arch = "wasm32")]
    if cfg!(target_feature = "simd128") {
        return Some(Backend::WasmSimd128);
    }
    None
}

const fn backend_matches_target(backend: Backend) -> bool {
    match backend {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        Backend::Avx512Vbmi | Backend::Avx2 | Backend::Ssse3Sse41 => true,
        #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
        Backend::Neon => true,
        #[cfg(target_arch = "wasm32")]
        Backend::WasmSimd128 => true,
        _ => false,
    }
}
