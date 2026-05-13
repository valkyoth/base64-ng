#![no_main]

use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_encode(data, STANDARD);
    exercise_encode(data, STANDARD_NO_PAD);
    exercise_encode(data, URL_SAFE);
    exercise_encode(data, URL_SAFE_NO_PAD);

    exercise_decode(data, STANDARD);
    exercise_decode(data, STANDARD_NO_PAD);
    exercise_decode(data, URL_SAFE);
    exercise_decode(data, URL_SAFE_NO_PAD);

    exercise_legacy_decode(data, STANDARD);
    exercise_legacy_decode(data, STANDARD_NO_PAD);
    exercise_legacy_decode(data, URL_SAFE);
    exercise_legacy_decode(data, URL_SAFE_NO_PAD);
});

fn exercise_encode<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let encoded = engine.encode_vec(input).expect("encoded length fits");
    let extra = input.len() % 7 + 1;
    let mut output = vec![0xa5; encoded.len() + extra];
    let written = engine
        .encode_slice_clear_tail(input, &mut output)
        .expect("encoding into large enough output succeeds");
    assert_eq!(&output[..written], encoded);
    assert!(output[written..].iter().all(|byte| *byte == 0));

    if !encoded.is_empty() {
        let mut too_small = vec![0xa5; encoded.len() - 1];
        let err = engine
            .encode_slice_clear_tail(input, &mut too_small)
            .expect_err("encoded output exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::EncodeError::OutputTooSmall {
                required: encoded.len(),
                available: encoded.len() - 1,
            }
        );
        assert!(too_small.iter().all(|byte| *byte == 0));
    }
}

fn exercise_decode<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let mut output = vec![0u8; base64_ng::decoded_capacity(input.len())];
    let slice_result = engine.decode_slice(input, &mut output);
    let vec_result = engine.decode_vec(input);

    match (&slice_result, &vec_result) {
        (Ok(written), Ok(decoded)) => assert_eq!(&output[..*written], decoded),
        (Err(slice_err), Err(vec_err)) => assert_eq!(slice_err, vec_err),
        (slice_result, vec_result) => {
            panic!("decode_slice and decode_vec disagreed: {slice_result:?} vs {vec_result:?}")
        }
    }

    let mut clear_tail_output = vec![0xa5; base64_ng::decoded_capacity(input.len()) + 3];
    let clear_tail_result = engine.decode_slice_clear_tail(input, &mut clear_tail_output);
    match (clear_tail_result, vec_result) {
        (Ok(written), Ok(decoded)) => {
            assert_eq!(&clear_tail_output[..written], decoded);
            assert!(
                clear_tail_output[written..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }
        (Err(clear_tail_err), Err(vec_err)) => {
            assert_eq!(clear_tail_err, vec_err);
            assert!(clear_tail_output.iter().all(|byte| *byte == 0));
        }
        (clear_tail_result, vec_result) => panic!(
            "decode_slice_clear_tail and decode_vec disagreed: {clear_tail_result:?} vs {vec_result:?}"
        ),
    }

    if let Ok(decoded) = engine.decode_vec(input)
        && !decoded.is_empty()
    {
        let mut too_small = vec![0xa5; decoded.len() - 1];
        let err = engine
            .decode_slice_clear_tail(input, &mut too_small)
            .expect_err("decoded output exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::DecodeError::OutputTooSmall {
                required: decoded.len(),
                available: decoded.len() - 1,
            }
        );
        assert!(too_small.iter().all(|byte| *byte == 0));
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

    match (&slice_result, &vec_result) {
        (Ok(written), Ok(decoded)) => assert_eq!(&output[..*written], decoded),
        (Err(slice_err), Err(vec_err)) => assert_eq!(slice_err, vec_err),
        (slice_result, vec_result) => panic!(
            "decode_slice_legacy and decode_vec_legacy disagreed: {slice_result:?} vs {vec_result:?}"
        ),
    }

    let mut clear_tail_output = vec![0xa5; required.unwrap_or(0) + 3];
    let clear_tail_result = engine.decode_slice_legacy_clear_tail(input, &mut clear_tail_output);
    match (clear_tail_result, vec_result) {
        (Ok(written), Ok(decoded)) => {
            assert_eq!(&clear_tail_output[..written], decoded);
            assert!(
                clear_tail_output[written..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }
        (Err(clear_tail_err), Err(vec_err)) => {
            assert_eq!(clear_tail_err, vec_err);
            assert!(clear_tail_output.iter().all(|byte| *byte == 0));
        }
        (clear_tail_result, vec_result) => panic!(
            "decode_slice_legacy_clear_tail and decode_vec_legacy disagreed: {clear_tail_result:?} vs {vec_result:?}"
        ),
    }

    if let Ok(decoded) = engine.decode_vec_legacy(input)
        && !decoded.is_empty()
    {
        let mut too_small = vec![0xa5; decoded.len() - 1];
        let err = engine
            .decode_slice_legacy_clear_tail(input, &mut too_small)
            .expect_err("decoded output exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::DecodeError::OutputTooSmall {
                required: decoded.len(),
                available: decoded.len() - 1,
            }
        );
        assert!(too_small.iter().all(|byte| *byte == 0));
    }
}
