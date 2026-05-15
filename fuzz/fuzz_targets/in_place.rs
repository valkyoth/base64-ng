#![no_main]

use base64_ng::{LineEnding, LineWrap, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_encode_in_place(data, STANDARD);
    exercise_encode_in_place(data, STANDARD_NO_PAD);
    exercise_encode_in_place(data, URL_SAFE);
    exercise_encode_in_place(data, URL_SAFE_NO_PAD);

    exercise_in_place(data, STANDARD);
    exercise_in_place(data, STANDARD_NO_PAD);
    exercise_in_place(data, URL_SAFE);
    exercise_in_place(data, URL_SAFE_NO_PAD);

    exercise_legacy_in_place(data, STANDARD);
    exercise_legacy_in_place(data, STANDARD_NO_PAD);
    exercise_legacy_in_place(data, URL_SAFE);
    exercise_legacy_in_place(data, URL_SAFE_NO_PAD);

    exercise_wrapped_in_place(data, STANDARD, LineWrap::new(4, LineEnding::Lf));
    exercise_wrapped_in_place(data, STANDARD, LineWrap::new(8, LineEnding::CrLf));
    exercise_wrapped_in_place(data, URL_SAFE, LineWrap::new(4, LineEnding::Lf));
});

fn exercise_encode_in_place<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let encoded = engine.encode_vec(input).expect("encoded length fits");
    let extra = input.len() % 7 + 1;
    let mut buffer = vec![0xa5; encoded.len() + extra];
    buffer[..input.len()].copy_from_slice(input);

    let clear_tail_result = engine
        .encode_in_place_clear_tail(&mut buffer, input.len())
        .map(|encoded| encoded.to_vec());

    match clear_tail_result {
        Ok(in_place) => {
            assert_eq!(in_place, encoded);
            assert!(buffer[encoded.len()..].iter().all(|byte| *byte == 0));
        }
        Err(err) => panic!("encode_in_place_clear_tail rejected valid input: {err:?}"),
    }

    if !input.is_empty() {
        let mut input_too_large = vec![0xa5; input.len() - 1];
        let err = engine
            .encode_in_place_clear_tail(&mut input_too_large, input.len())
            .expect_err("input length exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::EncodeError::InputTooLarge {
                input_len: input.len(),
                buffer_len: input.len() - 1,
            }
        );
        assert!(input_too_large.iter().all(|byte| *byte == 0));
    }

    if encoded.len() > 0 {
        let mut too_small = vec![0xa5; encoded.len() - 1];
        too_small[..input.len()].copy_from_slice(input);
        let err = engine
            .encode_in_place_clear_tail(&mut too_small, input.len())
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

fn exercise_in_place<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let vec_result = engine.decode_vec(input);
    let mut buffer = input.to_vec();
    let in_place_result = engine
        .decode_in_place(&mut buffer)
        .map(|decoded| decoded.to_vec());

    match (&in_place_result, &vec_result) {
        (Ok(in_place), Ok(decoded)) => assert_eq!(in_place, decoded),
        (Err(in_place_err), Err(vec_err)) => assert_eq!(in_place_err, vec_err),
        (in_place_result, vec_result) => {
            panic!(
                "decode_in_place and decode_vec disagreed: {in_place_result:?} vs {vec_result:?}"
            )
        }
    }

    let mut clear_tail_buffer = input.to_vec();
    let clear_tail_result = engine
        .decode_in_place_clear_tail(&mut clear_tail_buffer)
        .map(|decoded| decoded.to_vec());

    match (clear_tail_result, vec_result) {
        (Ok(clear_tail), Ok(decoded)) => {
            assert_eq!(clear_tail, decoded);
            assert!(
                clear_tail_buffer[decoded.len()..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }
        (Err(clear_tail_err), Err(vec_err)) => {
            assert_eq!(clear_tail_err, vec_err);
            assert!(clear_tail_buffer.iter().all(|byte| *byte == 0));
        }
        (clear_tail_result, vec_result) => {
            panic!(
                "decode_in_place_clear_tail and decode_vec disagreed: {clear_tail_result:?} vs {vec_result:?}"
            )
        }
    }
}

fn exercise_legacy_in_place<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let vec_result = engine.decode_vec_legacy(input);
    let mut buffer = input.to_vec();
    let in_place_result = engine
        .decode_in_place_legacy(&mut buffer)
        .map(|decoded| decoded.to_vec());

    match (&in_place_result, &vec_result) {
        (Ok(in_place), Ok(decoded)) => assert_eq!(in_place, decoded),
        (Err(in_place_err), Err(vec_err)) => assert_eq!(in_place_err, vec_err),
        (in_place_result, vec_result) => panic!(
            "decode_in_place_legacy and decode_vec_legacy disagreed: {in_place_result:?} vs {vec_result:?}"
        ),
    }

    let mut clear_tail_buffer = input.to_vec();
    let clear_tail_result = engine
        .decode_in_place_legacy_clear_tail(&mut clear_tail_buffer)
        .map(|decoded| decoded.to_vec());

    match (clear_tail_result, vec_result) {
        (Ok(clear_tail), Ok(decoded)) => {
            assert_eq!(clear_tail, decoded);
            assert!(
                clear_tail_buffer[decoded.len()..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }
        (Err(clear_tail_err), Err(vec_err)) => {
            assert_eq!(clear_tail_err, vec_err);
            assert!(clear_tail_buffer.iter().all(|byte| *byte == 0));
        }
        (clear_tail_result, vec_result) => panic!(
            "decode_in_place_legacy_clear_tail and decode_vec_legacy disagreed: {clear_tail_result:?} vs {vec_result:?}"
        ),
    }
}

fn exercise_wrapped_in_place<A, const PAD: bool>(
    input: &[u8],
    engine: base64_ng::Engine<A, PAD>,
    wrap: LineWrap,
) where
    A: base64_ng::Alphabet,
{
    let vec_result = engine.decode_wrapped_vec(input, wrap);
    let mut buffer = input.to_vec();
    let in_place_result = engine
        .decode_in_place_wrapped(&mut buffer, wrap)
        .map(|decoded| decoded.to_vec());

    match (&in_place_result, &vec_result) {
        (Ok(in_place), Ok(decoded)) => assert_eq!(in_place, decoded),
        (Err(in_place_err), Err(vec_err)) => assert_eq!(in_place_err, vec_err),
        (in_place_result, vec_result) => panic!(
            "decode_in_place_wrapped and decode_wrapped_vec disagreed: {in_place_result:?} vs {vec_result:?}"
        ),
    }

    let mut clear_tail_buffer = input.to_vec();
    let clear_tail_result = engine
        .decode_in_place_wrapped_clear_tail(&mut clear_tail_buffer, wrap)
        .map(|decoded| decoded.to_vec());

    match (clear_tail_result, vec_result) {
        (Ok(clear_tail), Ok(decoded)) => {
            assert_eq!(clear_tail, decoded);
            assert!(
                clear_tail_buffer[decoded.len()..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }
        (Err(clear_tail_err), Err(vec_err)) => {
            assert_eq!(clear_tail_err, vec_err);
            assert!(clear_tail_buffer.iter().all(|byte| *byte == 0));
        }
        (clear_tail_result, vec_result) => panic!(
            "decode_in_place_wrapped_clear_tail and decode_wrapped_vec disagreed: {clear_tail_result:?} vs {vec_result:?}"
        ),
    }
}
