#![no_main]

use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_in_place(data, STANDARD);
    exercise_in_place(data, STANDARD_NO_PAD);
    exercise_in_place(data, URL_SAFE);
    exercise_in_place(data, URL_SAFE_NO_PAD);

    exercise_legacy_in_place(data, STANDARD);
    exercise_legacy_in_place(data, STANDARD_NO_PAD);
    exercise_legacy_in_place(data, URL_SAFE);
    exercise_legacy_in_place(data, URL_SAFE_NO_PAD);
});

fn exercise_in_place<A, const PAD: bool>(input: &[u8], engine: base64_ng::Engine<A, PAD>)
where
    A: base64_ng::Alphabet,
{
    let vec_result = engine.decode_vec(input);
    let mut buffer = input.to_vec();
    let in_place_result = engine
        .decode_in_place(&mut buffer)
        .map(|decoded| decoded.to_vec());

    match (in_place_result, vec_result) {
        (Ok(in_place), Ok(decoded)) => assert_eq!(in_place, decoded),
        (Err(in_place_err), Err(vec_err)) => assert_eq!(in_place_err, vec_err),
        (in_place_result, vec_result) => {
            panic!(
                "decode_in_place and decode_vec disagreed: {in_place_result:?} vs {vec_result:?}"
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

    match (in_place_result, vec_result) {
        (Ok(in_place), Ok(decoded)) => assert_eq!(in_place, decoded),
        (Err(in_place_err), Err(vec_err)) => assert_eq!(in_place_err, vec_err),
        (in_place_result, vec_result) => panic!(
            "decode_in_place_legacy and decode_vec_legacy disagreed: {in_place_result:?} vs {vec_result:?}"
        ),
    }
}
