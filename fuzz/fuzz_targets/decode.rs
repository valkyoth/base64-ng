#![no_main]

use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_decode(data, STANDARD);
    exercise_decode(data, STANDARD_NO_PAD);
    exercise_decode(data, URL_SAFE);
    exercise_decode(data, URL_SAFE_NO_PAD);

    exercise_legacy_decode(data, STANDARD);
    exercise_legacy_decode(data, STANDARD_NO_PAD);
    exercise_legacy_decode(data, URL_SAFE);
    exercise_legacy_decode(data, URL_SAFE_NO_PAD);
});

fn exercise_decode<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let mut output = vec![0u8; base64_ng::decoded_capacity(input.len())];
    let slice_result = engine.decode_slice(input, &mut output);
    let vec_result = engine.decode_vec(input);

    match (slice_result, vec_result) {
        (Ok(written), Ok(decoded)) => assert_eq!(&output[..written], decoded),
        (Err(slice_err), Err(vec_err)) => assert_eq!(slice_err, vec_err),
        (slice_result, vec_result) => {
            panic!("decode_slice and decode_vec disagreed: {slice_result:?} vs {vec_result:?}")
        }
    }
}

fn exercise_legacy_decode<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let required = engine.decoded_len_legacy(input);
    let mut output = vec![0u8; required.unwrap_or(0)];
    let slice_result = engine.decode_slice_legacy(input, &mut output);
    let vec_result = engine.decode_vec_legacy(input);

    match (slice_result, vec_result) {
        (Ok(written), Ok(decoded)) => assert_eq!(&output[..written], decoded),
        (Err(slice_err), Err(vec_err)) => assert_eq!(slice_err, vec_err),
        (slice_result, vec_result) => panic!(
            "decode_slice_legacy and decode_vec_legacy disagreed: {slice_result:?} vs {vec_result:?}"
        ),
    }
}
