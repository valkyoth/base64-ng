use base64_ng::{Base64, Codec, OperationError, Status};

use super::Case;

pub(super) fn exercise_success<S: Codec>(case: &Case<'_>, codec: &Base64<S>) {
    assert_eq!(
        codec.encode_to_string(&case.input).unwrap().as_bytes(),
        case.encoded,
        "{}: 2.0 one-shot encode",
        case.id
    );
    assert_eq!(
        codec.decode_to_vec(case.encoded).unwrap(),
        case.input,
        "{}: 2.0 one-shot decode",
        case.id
    );
    assert_eq!(
        incremental_encode(codec, &case.input, &case.partitions).unwrap(),
        case.encoded,
        "{}: 2.0 incremental encode",
        case.id
    );
    assert_eq!(
        incremental_decode(codec, case.encoded, &case.partitions).unwrap(),
        case.input,
        "{}: 2.0 incremental decode",
        case.id
    );
}

pub(super) fn exercise_failure<S: Codec>(case: &Case<'_>, codec: &Base64<S>) {
    let mut output = [0xa5; 64];
    assert!(codec.decode_into(case.encoded, &mut output).is_err());
    assert!(output.iter().all(|byte| *byte == 0xa5));

    let (decoded, rejected) = run_incremental_decode(codec, case.encoded, &case.partitions);
    assert!(
        rejected,
        "{}: 2.0 incremental decode accepted input",
        case.id
    );
    assert_eq!(
        decoded, case.committed_prefix,
        "{}: 2.0 incremental committed prefix",
        case.id
    );
}

fn incremental_encode<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    partitions: &[usize],
) -> Result<Vec<u8>, OperationError> {
    let mut state = codec.encoder();
    let mut output = Vec::new();
    for_each_partition(input, partitions, |chunk| {
        let mut pending = [0u8; 128];
        let step = state.update(chunk, &mut pending)?;
        assert_eq!(step.progress().input_consumed(), chunk.len());
        output.extend_from_slice(&pending[..step.progress().output_produced()]);
        Ok(())
    })?;
    let mut pending = [0u8; 8];
    let step = state.finish(&mut pending)?;
    output.extend_from_slice(&pending[..step.progress().output_produced()]);
    assert_eq!(step.status(), Status::Complete);
    Ok(output)
}

fn incremental_decode<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    partitions: &[usize],
) -> Result<Vec<u8>, ()> {
    let (output, rejected) = run_incremental_decode(codec, input, partitions);
    if rejected { Err(()) } else { Ok(output) }
}

fn run_incremental_decode<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    partitions: &[usize],
) -> (Vec<u8>, bool) {
    let mut state = codec.decoder();
    let mut output = Vec::new();
    let updates: Result<(), OperationError> = for_each_partition(input, partitions, |chunk| {
        let mut pending = [0u8; 128];
        let step = state.update(chunk, &mut pending)?;
        assert_eq!(step.progress().input_consumed(), chunk.len());
        output.extend_from_slice(&pending[..step.progress().output_produced()]);
        Ok(())
    });
    if updates.is_err() {
        return (output, true);
    }
    let mut pending = [0u8; 8];
    match state.finish(&mut pending) {
        Ok(step) => {
            output.extend_from_slice(&pending[..step.progress().output_produced()]);
            assert_eq!(step.status(), Status::Complete);
            (output, false)
        }
        Err(_) => (output, true),
    }
}

fn for_each_partition<E, F>(
    input: &[u8],
    partitions: &[usize],
    mut visit: F,
) -> Result<(), E>
where
    F: FnMut(&[u8]) -> Result<(), E>,
{
    let mut offset = 0usize;
    for &requested in partitions {
        let end = offset.saturating_add(requested).min(input.len());
        visit(&input[offset..end])?;
        offset = end;
    }
    if offset < input.len() {
        visit(&input[offset..])?;
    }
    Ok(())
}
