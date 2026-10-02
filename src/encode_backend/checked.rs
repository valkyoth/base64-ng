//! Bounded redundant encode verification before caller-visible commit.

use crate::runtime::{Backend, OperationKind};
use crate::{Alphabet, BackendFault, EncodeError, checked_encoded_len, scalar, wipe_bytes};

const INPUT_CHUNK: usize = 768;
const OUTPUT_CHUNK: usize = 1024;

pub(super) fn encode<A: Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let remaining = input.len() - read;
        let chunk_len = if remaining > INPUT_CHUNK {
            INPUT_CHUNK
        } else {
            remaining
        };
        let chunk = &input[read..read + chunk_len];
        let chunk_required =
            checked_encoded_len(chunk_len, PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut accelerated = [0u8; OUTPUT_CHUNK];
        let mut reference = [0u8; OUTPUT_CHUNK];
        let accelerated_len = crate::v2::backend_health::direct_encode::<A, PAD>(
            backend,
            chunk,
            &mut accelerated[..chunk_required],
        );
        #[cfg(test)]
        let accelerated_len = tests::inject_result(accelerated_len, &mut accelerated);
        let reference_len = scalar::encode_slice::<A, PAD>(chunk, &mut reference[..chunk_required]);

        let written =
            match compare_results(accelerated_len, reference_len, &accelerated, &reference) {
                Ok(written) => written,
                Err(fault) => {
                    wipe_bytes(&mut accelerated);
                    wipe_bytes(&mut reference);
                    return scalar_retry::<A, PAD>(backend, fault, input, output);
                }
            };

        output[write..write + written].copy_from_slice(&accelerated[..written]);
        wipe_bytes(&mut accelerated);
        wipe_bytes(&mut reference);
        read += chunk_len;
        write += written;
    }
    Ok(write)
}

fn compare_results(
    accelerated_len: Option<usize>,
    reference_len: Result<usize, EncodeError>,
    accelerated: &[u8],
    reference: &[u8],
) -> Result<usize, BackendFault> {
    match (accelerated_len, reference_len) {
        (Some(actual), Ok(expected))
            if actual <= accelerated.len()
                && expected <= reference.len()
                && actual == expected
                && accelerated[..actual] == reference[..expected] =>
        {
            Ok(actual)
        }
        (Some(actual), Ok(expected))
            if actual <= accelerated.len() && expected <= reference.len() =>
        {
            Err(BackendFault::OutputMismatch)
        }
        _ => Err(BackendFault::ImpossibleState),
    }
}

fn scalar_retry<A: Alphabet, const PAD: bool>(
    backend: Backend,
    fault: BackendFault,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    #[cfg(test)]
    let injected = tests::record_quarantine(backend, fault);
    #[cfg(not(test))]
    let injected = false;
    if !injected {
        crate::v2::backend_health::quarantine(OperationKind::Encode, backend, fault);
    }
    match scalar::encode_slice::<A, PAD>(input, output) {
        Ok(written) => Ok(written),
        Err(error) => {
            crate::v2::backend_health::quarantine(
                OperationKind::Encode,
                backend,
                BackendFault::ScalarRetryFailed,
            );
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::{BackendFault, EncodeError};
    use std::cell::Cell;
    type Injection = (u8, usize, Option<(crate::runtime::Backend, BackendFault)>);

    std::thread_local! {
        static FAULT: Cell<Injection> = const {
            Cell::new((0, 0, None))
        };
    }

    pub(super) fn inject_result(result: Option<usize>, output: &mut [u8]) -> Option<usize> {
        FAULT.with(|state| {
            let (fault, calls, quarantine) = state.get();
            state.set((fault, calls + 1, quarantine));
            if calls != 1 {
                return result;
            }
            match fault {
                1 => {
                    output.fill(0xff);
                    None
                }
                2 => {
                    output.fill(0xff);
                    Some(usize::MAX)
                }
                3 => {
                    output[0] ^= 1;
                    result
                }
                _ => result,
            }
        })
    }

    pub(super) fn record_quarantine(backend: crate::runtime::Backend, fault: BackendFault) -> bool {
        FAULT.with(|state| {
            let (injection, calls, _) = state.get();
            if injection == 0 {
                return false;
            }
            state.set((injection, calls, Some((backend, fault))));
            true
        })
    }

    #[test]
    fn canonical_checked_faults_rewrite_previous_chunks_and_quarantine() {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                FAULT.with(|state| state.set((0, 0, None)));
            }
        }
        let _reset = Reset;
        let _ = crate::initialize_backends();
        if crate::encode_backend::candidate_encode_backend()
            == crate::encode_backend::EncodeBackend::Scalar
        {
            return;
        }
        let start = std::time::Instant::now();
        let backend = loop {
            let backend = crate::encode_backend::active_encode_backend_for_input(2307);
            if backend != crate::encode_backend::EncodeBackend::Scalar {
                break backend;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
            std::thread::yield_now();
        };
        let input = [0xa5; 2307];
        let mut reference = [0x5a; 3080];
        let required =
            crate::scalar::encode_slice::<crate::UrlSafe, false>(&input, &mut reference).unwrap();
        for injection in [1, 2, 3] {
            FAULT.with(|state| state.set((injection, 0, None)));
            let mut actual = [0x5a; 3080];
            assert_eq!(
                crate::STRICT_URL_SAFE_UNPADDED.encode_into(&input, &mut actual),
                Ok(required)
            );
            assert_eq!(actual, reference);
            let fault = if injection == 3 {
                BackendFault::OutputMismatch
            } else {
                BackendFault::ImpossibleState
            };
            assert_eq!(
                FAULT.with(Cell::get),
                (injection, 2, Some((backend.reported(), fault)))
            );
        }
    }

    #[test]
    fn checked_encode_matches_scalar_for_multiple_chunks() {
        let backend = crate::encode_backend::active_encode_backend();
        if backend == crate::encode_backend::EncodeBackend::Scalar {
            return;
        }
        let input = [0xa5; 1539];
        let mut checked = [0u8; 2052];
        let mut scalar = [0u8; 2052];
        let checked_len =
            super::encode::<crate::Standard, true>(backend.reported(), &input, &mut checked)
                .unwrap();
        let scalar_len =
            crate::scalar::encode_slice::<crate::Standard, true>(&input, &mut scalar).unwrap();
        assert_eq!(checked_len, scalar_len);
        assert_eq!(checked, scalar);
    }

    #[test]
    fn comparison_faults_are_classified_without_trusting_backend_lengths() {
        let reference = *b"QUJD";
        let mut mismatch = reference;
        mismatch[2] ^= 1;
        assert_eq!(
            super::compare_results(Some(4), Ok(4), &mismatch, &reference),
            Err(BackendFault::OutputMismatch)
        );
        assert_eq!(
            super::compare_results(Some(5), Ok(4), &mismatch, &reference),
            Err(BackendFault::ImpossibleState)
        );
        assert_eq!(
            super::compare_results(
                None,
                Err(EncodeError::LengthOverflow),
                &mismatch,
                &reference
            ),
            Err(BackendFault::ImpossibleState)
        );
    }
}
