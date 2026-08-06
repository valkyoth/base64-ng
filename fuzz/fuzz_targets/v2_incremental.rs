#![no_main]

use base64_ng::{
    Base64, Codec, CountedSink, DecoderState, EncoderState, OperationError, STRICT_STANDARD_PADDED,
    STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, Status, legacy,
    web,
};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let mode = data.first().copied().unwrap_or(0) % 4;
    let controls = &data[..data.len().min(32)];
    let input_start = data.len().min(32);
    let input = &data[input_start..data.len().min(input_start + MAX_INPUT)];
    match mode {
        0 => exercise_codec(&STRICT_STANDARD_PADDED, input, controls),
        1 => exercise_codec(&STRICT_STANDARD_UNPADDED, input, controls),
        2 => exercise_codec(&STRICT_URL_SAFE_PADDED, input, controls),
        _ => exercise_codec(&STRICT_URL_SAFE_UNPADDED, input, controls),
    }
    exercise_web(input, controls);
});

fn exercise_codec<S: Codec>(codec: &Base64<S>, input: &[u8], controls: &[u8]) {
    let encoded = codec.encode_to_string(input).unwrap();
    assert_eq!(
        drive_encoder(codec.encoder(), input, controls),
        encoded.as_bytes()
    );

    let (decoded, error) = drive_decoder(codec.decoder(), encoded.as_bytes(), controls);
    assert!(error.is_none());
    assert_eq!(decoded, input);

    let expected = codec.decode_to_vec(input);
    let (incremental, incremental_error) = drive_decoder(codec.decoder(), input, controls);
    match expected {
        Ok(expected) => {
            assert!(incremental_error.is_none());
            assert_eq!(incremental, expected);
        }
        Err(_) => assert!(incremental_error.is_some()),
    }

    exercise_legacy(codec, encoded.as_bytes(), input, controls);
    exercise_counted_and_formatter(codec, input, controls);
}

fn drive_encoder(mut state: EncoderState, input: &[u8], controls: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut offset = 0;
    let mut turn = 0;
    while offset < input.len() {
        let chunk_len = controlled(controls, turn, 17).min(input.len() - offset);
        let chunk_end = offset + chunk_len;
        while offset < chunk_end {
            let mut scratch = [0u8; 8];
            let output_len = controlled(controls, turn + 1, scratch.len());
            let step = state
                .update(&input[offset..chunk_end], &mut scratch[..output_len])
                .unwrap();
            let progress = step.progress();
            assert!(progress.input_consumed() != 0 || progress.output_produced() != 0);
            assert!(progress.input_consumed() <= chunk_end - offset);
            assert!(progress.output_produced() <= output_len);
            offset += progress.input_consumed();
            output.extend_from_slice(&scratch[..progress.output_produced()]);
            turn += 2;
        }
    }
    loop {
        let mut scratch = [0u8; 8];
        let output_len = controlled(controls, turn, scratch.len());
        let step = state.finish(&mut scratch[..output_len]).unwrap();
        output.extend_from_slice(&scratch[..step.progress().output_produced()]);
        turn += 1;
        if step.status() == Status::Complete {
            break;
        }
        assert!(step.progress().output_produced() != 0);
    }
    assert_eq!(state.finish(&mut []).unwrap().status(), Status::Complete);
    state.reset();
    assert_eq!(state.source_position(), 0);
    assert_eq!(state.buffered_input_len(), 0);
    output
}

fn drive_decoder(
    mut state: DecoderState,
    input: &[u8],
    controls: &[u8],
) -> (Vec<u8>, Option<OperationError>) {
    let mut output = Vec::new();
    let mut offset = 0;
    let mut turn = 0;
    while offset < input.len() {
        let chunk_len = controlled(controls, turn, 19).min(input.len() - offset);
        let chunk_end = offset + chunk_len;
        while offset < chunk_end {
            let mut scratch = [0u8; 7];
            let output_len = controlled(controls, turn + 1, scratch.len());
            match state.update(&input[offset..chunk_end], &mut scratch[..output_len]) {
                Ok(step) => {
                    let progress = step.progress();
                    assert!(progress.input_consumed() != 0 || progress.output_produced() != 0);
                    assert!(progress.input_consumed() <= chunk_end - offset);
                    assert!(progress.output_produced() <= output_len);
                    offset += progress.input_consumed();
                    output.extend_from_slice(&scratch[..progress.output_produced()]);
                }
                Err(error) => {
                    assert_eq!(state.update(&[], &mut []).unwrap_err(), error);
                    state.clear();
                    return (output, Some(error));
                }
            }
            turn += 2;
        }
    }
    loop {
        let mut scratch = [0u8; 7];
        let output_len = controlled(controls, turn, scratch.len());
        match state.finish(&mut scratch[..output_len]) {
            Ok(step) => {
                output.extend_from_slice(&scratch[..step.progress().output_produced()]);
                turn += 1;
                if step.status() == Status::Complete {
                    assert_eq!(state.finish(&mut []).unwrap().status(), Status::Complete);
                    return (output, None);
                }
                assert!(step.progress().output_produced() != 0);
            }
            Err(error) => {
                assert_eq!(state.finish(&mut []).unwrap_err(), error);
                state.clear();
                return (output, Some(error));
            }
        }
    }
}

fn exercise_legacy<S: Codec>(codec: &Base64<S>, encoded: &[u8], expected: &[u8], controls: &[u8]) {
    let mut spaced = Vec::with_capacity(encoded.len() * 2);
    for (index, byte) in encoded.iter().copied().enumerate() {
        spaced.push(byte);
        if controls
            .get(index % controls.len().max(1))
            .copied()
            .unwrap_or(0)
            & 1
            != 0
        {
            spaced.push([b' ', b'\t', b'\r', b'\n'][index % 4]);
        }
    }
    let required = legacy::ASCII_WHITESPACE
        .decoded_len(codec, &spaced)
        .unwrap();
    let mut output = vec![0u8; required];
    let written = legacy::ASCII_WHITESPACE
        .decode_into(codec, &spaced, &mut output)
        .unwrap();
    assert_eq!(&output[..written], expected);
}

fn exercise_web(input: &[u8], controls: &[u8]) {
    let Ok(text) = core::str::from_utf8(input) else {
        return;
    };
    let expected = web::FORGIVING.decode_to_vec(text);
    let mut state = web::FORGIVING.decoder();
    let mut output = Vec::new();
    let mut offset = 0;
    let mut turn = 0;
    let mut failed = false;
    while offset < text.len() {
        let mut end = (offset + controlled(controls, turn, 13)).min(text.len());
        while end > offset && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == offset {
            end = text[offset..]
                .char_indices()
                .nth(1)
                .map_or(text.len(), |(relative, _)| offset + relative);
        }
        let mut scratch = [0u8; 5];
        match state.update(
            &text[offset..end],
            &mut scratch[..controlled(controls, turn + 1, 5)],
        ) {
            Ok(step) => {
                offset += step.progress().input_consumed();
                output.extend_from_slice(&scratch[..step.progress().output_produced()]);
            }
            Err(_) => {
                failed = true;
                break;
            }
        }
        turn += 2;
    }
    if !failed {
        loop {
            let mut scratch = [0u8; 5];
            match state.finish(&mut scratch[..controlled(controls, turn, 5)]) {
                Ok(step) => {
                    output.extend_from_slice(&scratch[..step.progress().output_produced()]);
                    if step.status() == Status::Complete {
                        break;
                    }
                }
                Err(_) => {
                    failed = true;
                    break;
                }
            }
            turn += 1;
        }
    }
    match expected {
        Ok(expected) => {
            assert!(!failed);
            assert_eq!(output, expected);
        }
        Err(_) => assert!(failed),
    }
}

fn exercise_counted_and_formatter<S: Codec>(codec: &Base64<S>, input: &[u8], controls: &[u8]) {
    let expected = codec.encode_to_string(input).unwrap();
    let mut sink = ShortSink {
        output: Vec::new(),
        maximum: controlled(controls, 0, 7),
    };
    let written = codec.encode_to_counted(input, &mut sink).unwrap();
    assert_eq!(written, expected.len());
    assert_eq!(sink.output, expected.as_bytes());

    let fail_after = controls.first().map_or(0, |byte| usize::from(*byte) % 4);
    let mut formatter = FailingFormatter {
        output: Vec::new(),
        calls: 0,
        fail_after,
    };
    let result = codec.encode_to_fmt(input, &mut formatter);
    if input.len() > fail_after * 3 {
        let error = result.unwrap_err();
        assert_eq!(error.confirmed(), formatter.output.len());
        assert_eq!(formatter.output, expected.as_bytes()[..formatter.output.len()]);
    } else {
        assert_eq!(result.unwrap(), expected.len());
        assert_eq!(formatter.output, expected.as_bytes());
    }
}

struct ShortSink {
    output: Vec<u8>,
    maximum: usize,
}

impl CountedSink for ShortSink {
    type Error = ();

    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let accepted = bytes.len().min(self.maximum.max(1));
        self.output.extend_from_slice(&bytes[..accepted]);
        Ok(accepted)
    }
}

struct FailingFormatter {
    output: Vec<u8>,
    calls: usize,
    fail_after: usize,
}

impl core::fmt::Write for FailingFormatter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let call = self.calls;
        self.calls += 1;
        if call == self.fail_after {
            return Err(core::fmt::Error);
        }
        self.output.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

fn controlled(controls: &[u8], turn: usize, maximum: usize) -> usize {
    usize::from(
        controls
            .get(turn % controls.len().max(1))
            .copied()
            .unwrap_or(0),
    ) % maximum
        + 1
}
