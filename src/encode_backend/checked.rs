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
    crate::v2::backend_health::quarantine(OperationKind::Encode, backend, fault);
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
    use crate::{BackendFault, EncodeError};

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
