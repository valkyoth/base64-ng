#![no_main]

use base64_ng::{
    Base64, CodecBuilder, DecodePadding, EncodePadding, RuntimeSpec, TrailingBits,
    ValidatedAlphabet,
};
use libfuzzer_sys::fuzz_target;

const MAX_PAYLOAD: usize = 2048;
const STANDARD: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fuzz_target!(|data: &[u8]| {
    exercise_alphabet_rejection(data);
    let Some((codec, payload)) = runtime_codec(data) else {
        return;
    };
    exercise_codec(&codec, payload);
    exercise_arbitrary_decode(&codec, &data[..data.len().min(MAX_PAYLOAD)]);
});

fn exercise_alphabet_rejection(data: &[u8]) {
    let slice_len = data.first().map_or(0, |byte| usize::from(*byte) % 66);
    let remainder = data.get(1..).unwrap_or_default();
    let available = remainder.len().min(slice_len);
    let _ = ValidatedAlphabet::try_from_slice(&remainder[..available]);

    let mut table = STANDARD;
    for (destination, source) in table.iter_mut().zip(data.iter().copied()) {
        *destination = source;
    }
    let result = ValidatedAlphabet::new(table);
    if let Ok(alphabet) = result {
        for (index, byte) in alphabet.as_array().iter().copied().enumerate() {
            assert_eq!(alphabet.encode_value(index as u8), Some(byte));
            assert_eq!(alphabet.decode_byte(byte), Some(index as u8));
        }
        assert_eq!(alphabet.encode_value(64), None);
    }
}

fn runtime_codec(data: &[u8]) -> Option<(Base64<RuntimeSpec>, &[u8])> {
    let mut table = STANDARD;
    if data.len() >= 64 && data.first().is_some_and(|byte| byte & 1 != 0) {
        table.copy_from_slice(&data[..64]);
    } else if let Some(rotation) = data.first() {
        table.rotate_left(usize::from(*rotation) % 64);
    }
    let alphabet = ValidatedAlphabet::new(table).ok()?;
    let policy = data.get(64).copied().unwrap_or(0);
    let encode_padding = if policy & 1 == 0 {
        EncodePadding::Padded
    } else {
        EncodePadding::Unpadded
    };
    let decode_padding = match (policy >> 1) % 3 {
        0 => DecodePadding::RequireCanonical,
        1 => DecodePadding::Forbid,
        _ => DecodePadding::Indifferent,
    };
    let trailing_bits = if policy & 8 == 0 {
        TrailingBits::RequireCanonical
    } else {
        TrailingBits::AllowNonCanonical
    };
    let codec = CodecBuilder::new(alphabet)
        .encode_padding(encode_padding)
        .decode_padding(decode_padding)
        .trailing_bits(trailing_bits)
        .build()
        .ok()?;
    let payload_start = data.len().min(65);
    let payload_end = data.len().min(payload_start + MAX_PAYLOAD);
    Some((codec, &data[payload_start..payload_end]))
}

fn exercise_codec(codec: &Base64<RuntimeSpec>, input: &[u8]) {
    let expected = independent_encode(
        codec.settings().alphabet().as_array(),
        codec.settings().encode_padding() == EncodePadding::Padded,
        input,
    );
    let encoded = codec.encode_to_string(input).unwrap();
    assert_eq!(encoded.as_bytes(), expected);
    assert_eq!(codec.encoded_len(input.len()).unwrap(), expected.len());

    let mut output = vec![0xa5; expected.len() + 3];
    let written = codec.encode_into(input, &mut output).unwrap();
    assert_eq!(&output[..written], expected);
    assert!(output[written..].iter().all(|byte| *byte == 0xa5));

    if !expected.is_empty() {
        let mut too_small = vec![0x5a; expected.len() - 1];
        assert!(codec.encode_into(input, &mut too_small).is_err());
        assert!(too_small.iter().all(|byte| *byte == 0x5a));
        assert!(
            codec
                .encode_to_string_with_limit(input, expected.len() - 1)
                .is_err()
        );
    }

    let decoded = codec.decode_to_vec(&expected).unwrap();
    assert_eq!(decoded, input);
    assert_eq!(codec.decoded_len(&expected).unwrap(), input.len());

    let mut decoded_output = vec![0x3c; input.len() + 3];
    let decoded_len = codec.decode_into(&expected, &mut decoded_output).unwrap();
    assert_eq!(&decoded_output[..decoded_len], input);
    assert!(
        decoded_output[decoded_len..]
            .iter()
            .all(|byte| *byte == 0x3c)
    );

    let mut encode_in_place = vec![0x77; expected.len() + 2];
    encode_in_place[..input.len()].copy_from_slice(input);
    let in_place_len = codec
        .encode_in_place(&mut encode_in_place, input.len())
        .unwrap();
    assert_eq!(&encode_in_place[..in_place_len], expected);

    let mut decode_in_place = expected.clone();
    decode_in_place.extend_from_slice(&[0x66; 3]);
    let in_place_len = codec
        .decode_in_place(&mut decode_in_place, expected.len())
        .unwrap();
    assert_eq!(&decode_in_place[..in_place_len], input);

    let mut appended = String::from("prefix:");
    let prefix_len = appended.len();
    assert_eq!(
        codec.encode_append(input, &mut appended).unwrap(),
        expected.len()
    );
    assert_eq!(&appended.as_bytes()[prefix_len..], expected);

    let mut decoded_append = b"prefix:".to_vec();
    let prefix_len = decoded_append.len();
    assert_eq!(
        codec.decode_append(&expected, &mut decoded_append).unwrap(),
        input.len()
    );
    assert_eq!(&decoded_append[prefix_len..], input);
}

fn exercise_arbitrary_decode(codec: &Base64<RuntimeSpec>, input: &[u8]) {
    let mut output = vec![0x91; input.len().saturating_add(3)];
    let original = output.clone();
    let result = codec.decode_into(input, &mut output);
    match result {
        Ok(written) => {
            assert_eq!(codec.decoded_len(input), Ok(written));
            assert_eq!(codec.decode_to_vec(input).unwrap(), &output[..written]);
            assert!(output[written..].iter().all(|byte| *byte == 0x91));
        }
        Err(_) => assert_eq!(output, original),
    }
}

fn independent_encode(alphabet: &[u8; 64], padded: bool, input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity((input.len() + 2) / 3 * 4);
    let (chunks, remainder) = input.as_chunks::<3>();
    for chunk in chunks {
        output.push(alphabet[usize::from(chunk[0] >> 2)]);
        output.push(alphabet[usize::from(((chunk[0] & 3) << 4) | (chunk[1] >> 4))]);
        output.push(alphabet[usize::from(((chunk[1] & 15) << 2) | (chunk[2] >> 6))]);
        output.push(alphabet[usize::from(chunk[2] & 63)]);
    }
    match remainder {
        [] => {}
        [first] => {
            output.push(alphabet[usize::from(first >> 2)]);
            output.push(alphabet[usize::from((first & 3) << 4)]);
            if padded {
                output.extend_from_slice(b"==");
            }
        }
        [first, second] => {
            output.push(alphabet[usize::from(first >> 2)]);
            output.push(alphabet[usize::from(((first & 3) << 4) | (second >> 4))]);
            output.push(alphabet[usize::from((second & 15) << 2)]);
            if padded {
                output.push(b'=');
            }
        }
        _ => unreachable!(),
    }
    output
}
