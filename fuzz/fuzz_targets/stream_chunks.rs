#![no_main]

use std::io::Write;

use base64_ng::{
    STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    stream::{Decoder, Encoder},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (split_seed, payload) = data
        .split_first()
        .map_or((0, data), |(seed, rest)| (*seed, rest));

    exercise_decoder_chunks(payload, split_seed, STANDARD);
    exercise_decoder_chunks(payload, split_seed, STANDARD_NO_PAD);
    exercise_decoder_chunks(payload, split_seed, URL_SAFE);
    exercise_decoder_chunks(payload, split_seed, URL_SAFE_NO_PAD);

    exercise_encoder_chunks(payload, split_seed, STANDARD);
    exercise_encoder_chunks(payload, split_seed, STANDARD_NO_PAD);
    exercise_encoder_chunks(payload, split_seed, URL_SAFE);
    exercise_encoder_chunks(payload, split_seed, URL_SAFE_NO_PAD);
});

fn exercise_decoder_chunks<A, const PAD: bool>(
    input: &[u8],
    split_seed: u8,
    engine: base64_ng::Engine<A, PAD>,
) where
    A: base64_ng::Alphabet,
{
    let expected = engine.decode_vec(input);
    let mut decoder = Decoder::new(Vec::new(), engine);

    for chunk in input.chunks(chunk_size(split_seed)) {
        if decoder.write_all(chunk).is_err() {
            return;
        }
    }

    match (decoder.finish(), expected) {
        (Ok(streamed), Ok(decoded)) => assert_eq!(streamed, decoded),
        (Err(_), Err(_)) => {}
        (stream_result, expected) => {
            panic!("stream decoder and slice decoder disagreed: {stream_result:?} vs {expected:?}")
        }
    }
}

fn exercise_encoder_chunks<A, const PAD: bool>(
    input: &[u8],
    split_seed: u8,
    engine: base64_ng::Engine<A, PAD>,
) where
    A: base64_ng::Alphabet,
{
    let expected = engine.encode_vec(input).unwrap();
    let mut encoder = Encoder::new(Vec::new(), engine);

    for chunk in input.chunks(chunk_size(split_seed)) {
        encoder.write_all(chunk).unwrap();
    }

    let streamed = encoder.finish().unwrap();
    assert_eq!(streamed, expected);
}

fn chunk_size(seed: u8) -> usize {
    usize::from(seed % 17) + 1
}
