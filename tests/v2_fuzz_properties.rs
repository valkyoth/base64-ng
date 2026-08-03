#![cfg(feature = "alloc")]

use base64_ng::{
    Base64, Codec, CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED, Status,
    ValidatedAlphabet,
};

const STANDARD: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[test]
fn runtime_alphabet_and_padding_properties_are_deterministic() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for rotation in 0..64 {
        let mut table = STANDARD;
        table.rotate_left(rotation);
        let alphabet = ValidatedAlphabet::new(table).unwrap();
        for padded in [false, true] {
            let codec = CodecBuilder::new(alphabet)
                .encode_padding(if padded {
                    EncodePadding::Padded
                } else {
                    EncodePadding::Unpadded
                })
                .decode_padding(if padded {
                    DecodePadding::RequireCanonical
                } else {
                    DecodePadding::Forbid
                })
                .build()
                .unwrap();
            for len in 0..96 {
                let input = generated_bytes(&mut seed, len);
                let expected = independent_encode(&table, padded, &input);
                let encoded = codec.encode_to_string(&input).unwrap();
                assert_eq!(encoded.as_bytes(), expected);
                assert_eq!(codec.decode_to_vec(&expected).unwrap(), input);
            }
        }
    }
}

#[test]
fn every_small_incremental_partition_matches_one_shot() {
    let mut seed = 0xd1b5_4a32_d192_ed03u64;
    for len in 0..160 {
        let input = generated_bytes(&mut seed, len);
        let expected = STRICT_STANDARD_PADDED.encode_to_string(&input).unwrap();
        for input_chunk in 1..=9 {
            for output_chunk in 1..=5 {
                let encoded =
                    incremental_encode(&STRICT_STANDARD_PADDED, &input, input_chunk, output_chunk);
                assert_eq!(encoded, expected.as_bytes());
                let decoded = incremental_decode(
                    &STRICT_STANDARD_PADDED,
                    &encoded,
                    input_chunk,
                    output_chunk,
                );
                assert_eq!(decoded, input);
            }
        }
    }
}

fn generated_bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(len);
    for _ in 0..len {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        output.push(*seed as u8);
    }
    output
}

fn incremental_encode<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    input_chunk: usize,
    output_chunk: usize,
) -> Vec<u8> {
    let mut state = codec.encoder();
    let mut output = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let end = (offset + input_chunk).min(input.len());
        while offset < end {
            let mut scratch = [0u8; 5];
            let step = state
                .update(&input[offset..end], &mut scratch[..output_chunk])
                .unwrap();
            offset += step.progress().input_consumed();
            output.extend_from_slice(&scratch[..step.progress().output_produced()]);
        }
    }
    loop {
        let mut scratch = [0u8; 5];
        let step = state.finish(&mut scratch[..output_chunk]).unwrap();
        output.extend_from_slice(&scratch[..step.progress().output_produced()]);
        if step.status() == Status::Complete {
            return output;
        }
    }
}

fn incremental_decode<S: Codec>(
    codec: &Base64<S>,
    input: &[u8],
    input_chunk: usize,
    output_chunk: usize,
) -> Vec<u8> {
    let mut state = codec.decoder();
    let mut output = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let end = (offset + input_chunk).min(input.len());
        while offset < end {
            let mut scratch = [0u8; 5];
            let step = state
                .update(&input[offset..end], &mut scratch[..output_chunk])
                .unwrap();
            offset += step.progress().input_consumed();
            output.extend_from_slice(&scratch[..step.progress().output_produced()]);
        }
    }
    loop {
        let mut scratch = [0u8; 5];
        let step = state.finish(&mut scratch[..output_chunk]).unwrap();
        output.extend_from_slice(&scratch[..step.progress().output_produced()]);
        if step.status() == Status::Complete {
            return output;
        }
    }
}

fn independent_encode(alphabet: &[u8; 64], padded: bool, input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        output.extend_from_slice(&[
            alphabet[usize::from(chunk[0] >> 2)],
            alphabet[usize::from(((chunk[0] & 3) << 4) | (chunk[1] >> 4))],
            alphabet[usize::from(((chunk[1] & 15) << 2) | (chunk[2] >> 6))],
            alphabet[usize::from(chunk[2] & 63)],
        ]);
    }
    match chunks.remainder() {
        [] => {}
        [first] => {
            output.extend_from_slice(&[
                alphabet[usize::from(first >> 2)],
                alphabet[usize::from((first & 3) << 4)],
            ]);
            if padded {
                output.extend_from_slice(b"==");
            }
        }
        [first, second] => {
            output.extend_from_slice(&[
                alphabet[usize::from(first >> 2)],
                alphabet[usize::from(((first & 3) << 4) | (second >> 4))],
                alphabet[usize::from((second & 15) << 2)],
            ]);
            if padded {
                output.push(b'=');
            }
        }
        _ => unreachable!(),
    }
    output
}
