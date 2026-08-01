//! Bounded redundant strict-decode verification before caller-visible commit.

use crate::runtime::{Backend, OperationKind};
use crate::{Alphabet, BackendFault, DecodeError, scalar, wipe_bytes};

const INPUT_CHUNK: usize = 1024;
const OUTPUT_CHUNK: usize = 768;

pub(super) fn decode<A: Alphabet, const PAD: bool>(
    backend: Backend,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
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
        let mut accelerated = [0u8; OUTPUT_CHUNK];
        let mut reference = [0u8; OUTPUT_CHUNK];
        let accelerated_len =
            crate::v2::backend_health::direct_decode::<A, PAD>(backend, chunk, &mut accelerated);
        let reference_len = scalar::decode_slice::<A, PAD>(chunk, &mut reference);
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
    reference_len: Result<usize, DecodeError>,
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
) -> Result<usize, DecodeError> {
    crate::v2::backend_health::quarantine(OperationKind::StrictDecode, backend, fault);
    match scalar::decode_slice::<A, PAD>(input, output) {
        Ok(written) => Ok(written),
        Err(error) => {
            crate::v2::backend_health::quarantine(
                OperationKind::StrictDecode,
                backend,
                BackendFault::ScalarRetryFailed,
            );
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{BackendFault, DecodeError};

    #[test]
    fn checked_decode_matches_scalar_for_multiple_chunks() {
        let backend = crate::decode_backend::active_decode_backend();
        if backend == crate::decode_backend::DecodeBackend::Scalar {
            return;
        }
        let input = [0xa5; 1539];
        let mut encoded = [0u8; 2052];
        crate::scalar::encode_slice::<crate::Standard, true>(&input, &mut encoded).unwrap();
        let mut checked = [0u8; 1539];
        let mut scalar = [0u8; 1539];
        let checked_len =
            super::decode::<crate::Standard, true>(backend.reported(), &encoded, &mut checked)
                .unwrap();
        let scalar_len =
            crate::scalar::decode_slice::<crate::Standard, true>(&encoded, &mut scalar).unwrap();
        assert_eq!(checked_len, scalar_len);
        assert_eq!(checked, scalar);
    }

    #[test]
    fn malformed_input_is_rejected_before_output_changes() {
        let backend = crate::decode_backend::active_decode_backend();
        if backend == crate::decode_backend::DecodeBackend::Scalar {
            return;
        }
        let mut output = [0x55; 12];
        let error = super::decode::<crate::Standard, true>(
            backend.reported(),
            b"QUJDREVGR0hJ!!!!",
            &mut output,
        )
        .unwrap_err();
        assert!(matches!(error, crate::DecodeError::InvalidByte { .. }));
        assert_eq!(output, [0x55; 12]);
    }

    #[test]
    fn comparison_faults_are_classified_without_trusting_backend_lengths() {
        let reference = *b"ABC";
        let mut mismatch = reference;
        mismatch[1] ^= 1;
        assert_eq!(
            super::compare_results(Some(3), Ok(3), &mismatch, &reference),
            Err(BackendFault::OutputMismatch)
        );
        assert_eq!(
            super::compare_results(Some(4), Ok(3), &mismatch, &reference),
            Err(BackendFault::ImpossibleState)
        );
        assert_eq!(
            super::compare_results(None, Err(DecodeError::InvalidLength), &mismatch, &reference,),
            Err(BackendFault::ImpossibleState)
        );
    }
}
