#![no_main]

use base64_ng_mime::{
    MimeBodyDecodePolicy, MimeBodyDecoder, MimeBodyEncoder, MimeBodyLimits, MimeBodyStatus,
    MimeBodyTerminalLineEnding, decode_mime_content_transfer_body_to_vec,
    encode_mime_content_transfer_body_to_string,
};
use libfuzzer_sys::fuzz_target;

const LIMITS: MimeBodyLimits = MimeBodyLimits::new(8192, 16384, 8192, 1024, 4096, 4096);

fuzz_target!(|data: &[u8]| {
    let split_seed = data.first().copied().unwrap_or(1);
    let plain = data.get(1..).unwrap_or_default();
    let expected = encode_mime_content_transfer_body_to_string(
        plain,
        LIMITS,
        MimeBodyTerminalLineEnding::IncludeCrLf,
    )
    .unwrap();
    let encoded = encode_chunks(plain, split_seed);
    assert_eq!(encoded, expected.as_bytes());

    let decoded = decode_chunks(&encoded, split_seed.wrapping_add(1)).unwrap();
    assert_eq!(decoded, plain);

    let one_shot = decode_mime_content_transfer_body_to_vec(
        data,
        MimeBodyDecodePolicy::Rfc2045Compatible,
        LIMITS,
    );
    let chunked = decode_chunks(data, split_seed.wrapping_add(2));
    match (one_shot, chunked) {
        (Ok((expected, _)), Ok(actual)) => assert_eq!(actual, expected),
        (Err(_), Err(_)) => {}
        (left, right) => panic!("one-shot/chunk result mismatch: {left:?} {right:?}"),
    }
});

fn encode_chunks(input: &[u8], seed: u8) -> Vec<u8> {
    let mut state = MimeBodyEncoder::new(LIMITS, MimeBodyTerminalLineEnding::IncludeCrLf);
    let mut output = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let chunk = chunk_len(seed, offset, input.len() - offset);
        let end = offset + chunk;
        while offset < end {
            let mut destination = [0u8; 7];
            let step = state.update(&input[offset..end], &mut destination).unwrap();
            offset += step.progress().input_consumed();
            output.extend_from_slice(&destination[..step.progress().output_produced()]);
        }
    }
    loop {
        let mut destination = [0u8; 7];
        let step = state.finish(&mut destination).unwrap();
        output.extend_from_slice(&destination[..step.progress().output_produced()]);
        if step.status() == MimeBodyStatus::Complete {
            return output;
        }
    }
}

fn decode_chunks(input: &[u8], seed: u8) -> Result<Vec<u8>, ()> {
    let mut state = MimeBodyDecoder::new(MimeBodyDecodePolicy::Rfc2045Compatible, LIMITS);
    let mut output = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let chunk = chunk_len(seed, offset, input.len() - offset);
        let end = offset + chunk;
        while offset < end {
            let mut destination = [0u8; 5];
            let step = state
                .update(&input[offset..end], &mut destination)
                .map_err(|_| ())?;
            offset += step.progress().input_consumed();
            output.extend_from_slice(&destination[..step.progress().output_produced()]);
        }
    }
    loop {
        let mut destination = [0u8; 5];
        let step = state.finish(&mut destination).map_err(|_| ())?;
        output.extend_from_slice(&destination[..step.progress().output_produced()]);
        if step.status() == MimeBodyStatus::Complete {
            return Ok(output);
        }
    }
}

fn chunk_len(seed: u8, offset: usize, remaining: usize) -> usize {
    (usize::from(seed).wrapping_add(offset) % 17 + 1).min(remaining)
}
