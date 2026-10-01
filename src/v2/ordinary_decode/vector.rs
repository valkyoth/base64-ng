//! Ordinary x86 bulk operations. Tail grammar remains in the scalar validator.
use super::Family;
use crate::{
    BackendFault,
    runtime::{Backend, OperationKind},
};

#[inline]
pub(super) fn select(len: usize) -> Option<Backend> {
    // Complete-call measurements show setup dominates small inputs. This
    // conservative integration floor does not change existing ISA thresholds.
    if len < 512 {
        return None;
    }
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // Reserve the final quantum. No automatic AVX-512 admission.
        let backend =
            crate::decode_backend::active_decode_backend_for_input(len.saturating_sub(1) / 4 * 4)
                .reported();
        width(backend).map(|_| backend)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        None
    }
}

fn width(backend: Backend) -> Option<usize> {
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        crate::simd::ordinary::width(backend)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        let _ = backend;
        None
    }
}

pub(super) fn quarantine(backend: Backend, fault: BackendFault) {
    #[cfg(all(test, feature = "std"))]
    if tests::quarantine(backend, fault) {
        return;
    }
    crate::v2::backend_health::quarantine(OperationKind::StrictDecode, backend, fault);
}

pub(super) fn validated_len(
    backend: Backend,
    family: Family,
    input: &[u8],
    padded: bool,
) -> Option<usize> {
    let width = width(backend)?;
    let prefix = input.len().saturating_sub(1) / width * width;
    if !validate(backend, &input[..prefix], family == Family::UrlSafe) {
        return None;
    }
    let tail = family.validated_len(&input[prefix..], padded)?;
    (prefix / 4).checked_mul(3)?.checked_add(tail)
}

fn validate(backend: Backend, input: &[u8], url: bool) -> bool {
    #[cfg(all(test, feature = "std"))]
    if let Some(valid) = tests::validate(backend) {
        return valid;
    }
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        crate::simd::ordinary::validate(backend, input, url)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        let _ = (backend, input, url);
        false
    }
}

fn decode(backend: Backend, input: &[u8], output: &mut [u8], url: bool) -> bool {
    #[cfg(all(test, feature = "std"))]
    if let Some(valid) = tests::decode(backend, output) {
        return valid;
    }
    #[cfg(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        crate::simd::ordinary::decode(backend, input, output, url)
    }
    #[cfg(not(all(feature = "simd", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        let _ = (backend, input, output, url);
        false
    }
}

// Returning zero makes the caller overwrite the entire body with the table
// writer. This input already has a complete proof; retry cannot expose an error
// after partial output. The remainder and tail are always written by the caller.
pub(super) fn write(backend: Backend, family: Family, input: &[u8], output: &mut [u8]) -> usize {
    let Some(width) = width(backend) else {
        return 0;
    };
    if !crate::v2::backend_health::admit(OperationKind::StrictDecode, backend) {
        return 0;
    }
    #[cfg(all(test, feature = "std"))]
    if tests::unavailable() {
        return 0;
    }
    let prefix = input.len() / width * width;
    let input = &input[..prefix];
    let output = &mut output[..prefix / 4 * 3];
    #[cfg(not(feature = "checked-backend"))]
    let valid = decode(backend, input, output, family == Family::UrlSafe);
    #[cfg(feature = "checked-backend")]
    let valid = checked(backend, family, input, output);
    if valid {
        #[cfg(all(
            test,
            feature = "simd",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        crate::decode_backend::record_test_execution(match backend {
            Backend::Avx2 => crate::decode_backend::DecodeBackend::Avx2,
            Backend::Ssse3Sse41 => crate::decode_backend::DecodeBackend::Ssse3Sse41,
            _ => crate::decode_backend::DecodeBackend::Scalar,
        });
        prefix
    } else {
        quarantine(backend, BackendFault::OutputMismatch);
        0
    }
}

#[cfg(all(test, feature = "std"))]
mod tests;

#[cfg(feature = "checked-backend")]
fn checked(backend: Backend, family: Family, input: &[u8], output: &mut [u8]) -> bool {
    for (input, output) in input.chunks(1024).zip(output.chunks_mut(768)) {
        let mut accelerated = [0; 768];
        let mut reference = [0; 768];
        let len = output.len();
        let valid = decode(
            backend,
            input,
            &mut accelerated[..len],
            family == Family::UrlSafe,
        );
        let reference_len = match family {
            Family::Standard => {
                crate::scalar::decode_slice::<crate::Standard, false>(input, &mut reference)
            }
            Family::UrlSafe => {
                crate::scalar::decode_slice::<crate::UrlSafe, false>(input, &mut reference)
            }
        };
        let agrees = valid && reference_len == Ok(len) && accelerated[..len] == reference[..len];
        if agrees {
            output.copy_from_slice(&accelerated[..len]);
        }
        crate::wipe_bytes(&mut accelerated);
        crate::wipe_bytes(&mut reference);
        if !agrees {
            return false;
        }
    }
    true
}
