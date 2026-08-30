#![cfg(feature = "alloc")]

use alloc::{string::String, vec::Vec};

use super::{
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, Status, compat, web, web::ForgivingError,
};

const FIXTURES: &str = include_str!("../../tests/fixtures/whatwg-forgiving-base64.txt");

#[test]
fn forgiving_one_shot_matches_locked_whatwg_and_browser_fixtures() {
    for (input, expected) in fixtures() {
        let input = String::from_utf8(input).unwrap();
        let mut output = [0xa5; 16];
        if let Some(expected) = expected {
            assert_eq!(web::FORGIVING.validate(&input), Ok(()));
            assert_eq!(web::FORGIVING.decoded_len(&input), Ok(expected.len()));
            let written = web::FORGIVING.decode_into(&input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
            assert!(output[written..].iter().all(|byte| *byte == 0xa5));
            assert_eq!(web::FORGIVING.decode_to_vec(&input).unwrap(), expected);
        } else {
            assert_eq!(
                web::FORGIVING.validate(&input),
                Err(ForgivingError::InvalidInput)
            );
            assert_eq!(
                web::FORGIVING.decode_into(&input, &mut output),
                Err(ForgivingError::InvalidInput)
            );
            assert_eq!(output, [0xa5; 16]);
        }
    }
}

#[test]
fn forgiving_incremental_matches_every_fixture_at_every_split() {
    for (input, expected) in fixtures() {
        let input = String::from_utf8(input).unwrap();
        for split in 0..=input.len() {
            if !input.is_char_boundary(split) {
                continue;
            }
            let result = decode_incrementally([&input[..split], &input[split..]]);
            assert_eq!(
                result.ok().as_deref(),
                expected.as_deref(),
                "{input:?} at {split}"
            );
        }
    }
}

#[test]
fn forgiving_incremental_handles_one_byte_chunks_and_absorbs_failure() {
    let input = " Z\tg\n=\r= ";
    let chunks: Vec<&str> = input
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(index, _)| input.get(index..=index).unwrap())
        .collect();
    assert_eq!(decode_incrementally(chunks).unwrap(), b"f");

    let mut decoder = web::FORGIVING.decoder();
    assert_eq!(
        decoder.update("!", &mut [0; 1]),
        Err(ForgivingError::InvalidInput)
    );
    assert_eq!(
        decoder.update("Zg==", &mut [0; 1]),
        Err(ForgivingError::InvalidInput)
    );
    assert_eq!(
        decoder.finish(&mut [0; 1]),
        Err(ForgivingError::InvalidInput)
    );
    decoder.reset();
    assert_eq!(decode_existing(&mut decoder, ["Zg=="]).unwrap(), b"f");
}

#[test]
fn forgiving_incremental_error_does_not_hide_same_call_output() {
    let mut decoder = web::FORGIVING.decoder();
    let mut output = [0xa5; 8];
    assert_eq!(
        decoder.update("Zm9v!", &mut output),
        Err(ForgivingError::InvalidInput)
    );
    assert_eq!(output, [0xa5; 8]);
    assert_eq!(decoder.source_position(), 0);
}

#[test]
fn forgiving_one_shot_is_transactional_and_allocation_bounded() {
    let mut short = [0xa5; 1];
    assert_eq!(
        web::FORGIVING.decode_into("Zm9v", &mut short),
        Err(ForgivingError::OutputTooSmall {
            required: 3,
            available: 1,
        })
    );
    assert_eq!(short, [0xa5]);
    assert_eq!(
        web::FORGIVING.decode_to_vec_with_limit("Zm9v", 2),
        Err(ForgivingError::AllocationLimitExceeded {
            required: 3,
            limit: 2,
        })
    );
}

#[test]
fn strict_presets_reject_web_only_acceptance() {
    for input in [b" Zg== ".as_slice(), b"Zh==", b"Zg\n==", b"Zg==\r\n"] {
        assert!(
            STRICT_STANDARD_PADDED
                .decode_into(input, &mut [0; 8])
                .is_err()
        );
    }
    for input in [b" Zh ".as_slice(), b"Zh", b"Zg\n"] {
        assert!(
            STRICT_STANDARD_UNPADDED
                .decode_into(input, &mut [0; 8])
                .is_err()
        );
    }
    assert_eq!(web::FORGIVING.decode_to_vec(" Zh== ").unwrap(), b"f");
    assert_eq!(web::FORGIVING.decode_to_vec("Zh").unwrap(), b"f");
}

#[test]
fn named_compatibility_presets_are_not_secret_eligible() {
    let presets = [
        compat::STANDARD_PADDED_PADDING_INDIFFERENT,
        compat::STANDARD_UNPADDED_PADDING_INDIFFERENT,
        compat::STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::STANDARD_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::STANDARD_PADDED_FULL_COMPATIBILITY,
        compat::STANDARD_UNPADDED_FULL_COMPATIBILITY,
        compat::URL_SAFE_PADDED_PADDING_INDIFFERENT,
        compat::URL_SAFE_UNPADDED_PADDING_INDIFFERENT,
        compat::URL_SAFE_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::URL_SAFE_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::URL_SAFE_PADDED_FULL_COMPATIBILITY,
        compat::URL_SAFE_UNPADDED_FULL_COMPATIBILITY,
    ];
    assert!(
        presets
            .iter()
            .all(|codec| !codec.settings().permits_secret_processing())
    );
}

#[test]
fn compatibility_trailing_bits_match_independent_bit_oracle() {
    let table = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for second in 0u8..64 {
        let input = [table[0], table[usize::from(second)], b'=', b'='];
        let decoded = compat::STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS
            .decode_to_vec(&input)
            .unwrap();
        assert_eq!(decoded, [(second >> 4)]);
    }
    for third in 0u8..64 {
        let input = [table[0], table[0], table[usize::from(third)], b'='];
        let decoded = compat::STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS
            .decode_to_vec(&input)
            .unwrap();
        assert_eq!(decoded, [0, third >> 2]);
    }
}

#[test]
fn forgiving_short_inputs_match_independent_bit_stream_oracle() {
    const SYMBOLS: &[u8] = b"AB+/= \n-!";
    for len in 0..=5 {
        let cases = SYMBOLS.len().pow(u32::try_from(len).unwrap());
        for mut case in 0..cases {
            let mut bytes = vec![0u8; len];
            for byte in &mut bytes {
                *byte = SYMBOLS[case % SYMBOLS.len()];
                case /= SYMBOLS.len();
            }
            let input = String::from_utf8(bytes).unwrap();
            assert_eq!(
                web::FORGIVING.decode_to_vec(&input).ok(),
                forgiving_oracle(&input),
                "{input:?}"
            );
        }
    }
}

fn decode_incrementally<I>(chunks: I) -> Result<Vec<u8>, ForgivingError>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    decode_existing(&mut web::FORGIVING.decoder(), chunks)
}

fn decode_existing<I>(
    decoder: &mut web::ForgivingDecoder,
    chunks: I,
) -> Result<Vec<u8>, ForgivingError>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut output_bytes = Vec::new();
    for chunk in chunks {
        let chunk = chunk.as_ref();
        let mut offset = 0;
        while offset < chunk.len() {
            let mut byte = [0u8; 1];
            let step = decoder.update(&chunk[offset..], &mut byte)?;
            let progress = step.progress();
            output_bytes.extend_from_slice(&byte[..progress.output_produced()]);
            offset += progress.input_consumed();
            assert!(progress.input_consumed() != 0 || progress.output_produced() != 0);
        }
    }
    loop {
        let mut byte = [0u8; 1];
        let step = decoder.finish(&mut byte)?;
        output_bytes.extend_from_slice(&byte[..step.progress().output_produced()]);
        if step.status() == Status::Complete {
            return Ok(output_bytes);
        }
    }
}

fn fixtures() -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    FIXTURES
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let (input, expected) = line.split_once('|').unwrap();
            let expected = expected.trim();
            (
                decode_hex(input.trim()),
                (expected != "ERROR").then(|| decode_hex(expected)),
            )
        })
        .collect()
}

fn decode_hex(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = core::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

fn forgiving_oracle(input: &str) -> Option<Vec<u8>> {
    let mut symbols: Vec<u8> = input
        .bytes()
        .filter(|byte| !matches!(*byte, b'\t' | b'\n' | 0x0c | b'\r' | b' '))
        .collect();
    if symbols.len().is_multiple_of(4) {
        for _ in 0..2 {
            if symbols.last() == Some(&b'=') {
                symbols.pop();
            }
        }
    }
    if symbols.len() % 4 == 1 {
        return None;
    }

    let mut output = Vec::new();
    let mut accumulator = 0u32;
    let mut bit_count = 0u8;
    for symbol in symbols {
        let value = match symbol {
            b'A'..=b'Z' => symbol - b'A',
            b'a'..=b'z' => symbol - b'a' + 26,
            b'0'..=b'9' => symbol - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bit_count += 6;
        if bit_count >= 8 {
            bit_count -= 8;
            output.push(u8::try_from(accumulator >> bit_count).unwrap());
            accumulator &= (1u32 << bit_count) - 1;
        }
    }
    Some(output)
}
