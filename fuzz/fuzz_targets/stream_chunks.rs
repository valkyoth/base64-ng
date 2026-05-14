#![no_main]

use std::io::{Cursor, Read, Write};

use base64_ng::{
    STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    stream::{Decoder, DecoderReader, Encoder},
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

    exercise_decoder_reader_chunks(payload, split_seed, STANDARD);
    exercise_decoder_reader_chunks(payload, split_seed, STANDARD_NO_PAD);
    exercise_decoder_reader_chunks(payload, split_seed, URL_SAFE);
    exercise_decoder_reader_chunks(payload, split_seed, URL_SAFE_NO_PAD);

    exercise_decoder_reader_adjacent_payload(payload, split_seed, STANDARD);
    exercise_decoder_reader_adjacent_payload(payload, split_seed, URL_SAFE);
});

struct ChunkedReader<'a> {
    input: &'a [u8],
    max_chunk: usize,
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.input.len().min(self.max_chunk).min(output.len());
        if len == 0 {
            return Ok(0);
        }

        output[..len].copy_from_slice(&self.input[..len]);
        self.input = &self.input[len..];
        Ok(len)
    }
}

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

fn exercise_decoder_reader_chunks<A, const PAD: bool>(
    input: &[u8],
    split_seed: u8,
    engine: base64_ng::Engine<A, PAD>,
) where
    A: base64_ng::Alphabet,
{
    if PAD && input.contains(&b'=') {
        return;
    }

    let expected = engine.decode_vec(input);
    let source = ChunkedReader {
        input,
        max_chunk: chunk_size(split_seed),
    };
    let mut reader = DecoderReader::new(source, engine);
    let mut decoded = Vec::new();
    let streamed = reader.read_to_end(&mut decoded);

    match (streamed, expected) {
        (Ok(_), Ok(expected)) => assert_eq!(decoded, expected),
        (Err(_), Err(_)) => {}
        (streamed, expected) => {
            panic!("decoder reader and slice decoder disagreed: {streamed:?} vs {expected:?}")
        }
    }
}

fn exercise_decoder_reader_adjacent_payload<A>(
    input: &[u8],
    split_seed: u8,
    engine: base64_ng::Engine<A, true>,
) where
    A: base64_ng::Alphabet,
{
    let payload_len = input
        .len()
        .min(usize::from(split_seed % 31) + 1)
        .max(1);
    let mut payload = Vec::with_capacity(payload_len);
    if input.is_empty() {
        payload.push(split_seed);
    } else {
        payload.extend_from_slice(&input[..payload_len]);
    }
    if payload.len() % 3 == 0 {
        payload.push(split_seed.wrapping_add(1));
    }

    let suffix = if input.len() > payload_len {
        &input[payload_len..]
    } else {
        b"NEXT"
    };

    let encoded = engine.encode_vec(&payload).unwrap();
    assert!(encoded.contains(&b'='));

    let mut stream = Vec::with_capacity(encoded.len() + suffix.len());
    stream.extend_from_slice(&encoded);
    stream.extend_from_slice(suffix);

    let cursor = Cursor::new(stream.as_slice());
    let mut reader = DecoderReader::new(cursor, engine);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();

    assert_eq!(decoded, payload);
    assert_eq!(reader.get_ref().position(), encoded.len() as u64);
    let remaining = &reader.get_ref().get_ref()[encoded.len()..];
    assert_eq!(remaining, suffix);
}

fn chunk_size(seed: u8) -> usize {
    usize::from(seed % 17) + 1
}
