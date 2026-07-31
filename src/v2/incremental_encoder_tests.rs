extern crate std;

use std::{vec, vec::Vec};

use super::{
    contracts::{OperationError, Status, TerminalError},
    incremental::EncoderState,
    rfc4648_oracle::{self as oracle, Profile},
    specifications::{
        CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
        STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
    },
};

const RFC_VECTORS: &[(&[u8], &[u8])] = &[
    (b"", b""),
    (b"f", b"Zg=="),
    (b"fo", b"Zm8="),
    (b"foo", b"Zm9v"),
    (b"foob", b"Zm9vYg=="),
    (b"fooba", b"Zm9vYmE="),
    (b"foobar", b"Zm9vYmFy"),
];

#[test]
fn rfc4648_vectors_pass_every_input_and_output_partition() {
    for &(input, padded) in RFC_VECTORS {
        for profile in profiles() {
            let expected = if is_padded(profile) {
                padded.to_vec()
            } else {
                oracle::encode(profile, input)
            };
            assert_every_partition(profile, input, &expected);
        }
    }
}

#[test]
fn bounded_inputs_match_independent_oracle_for_every_partition() {
    let mut input = [0_u8; 6];
    for len in 0..=input.len() {
        for (index, byte) in input[..len].iter_mut().enumerate() {
            *byte = u8::try_from((index * 73 + len * 41) % 256).unwrap();
        }
        for profile in profiles() {
            let expected = oracle::encode(profile, &input[..len]);
            assert_every_partition(profile, &input[..len], &expected);
        }
    }
}

#[test]
fn one_byte_chunks_and_repeated_output_full_never_reconsume_input() {
    let mut state = STRICT_STANDARD_PADDED.encoder();
    let mut no_output = [];
    let first = state.update(b"foo", &mut no_output).unwrap();
    assert_eq!(first.progress().input_consumed(), 3);
    assert_eq!(first.progress().output_produced(), 0);
    assert_output_full(first.status());
    assert_eq!(state.source_position(), 3);

    for _ in 0..3 {
        let stalled = state.update(b"", &mut no_output).unwrap();
        assert_eq!(stalled.progress().input_consumed(), 0);
        assert_eq!(stalled.progress().output_produced(), 0);
        assert_output_full(stalled.status());
        assert_eq!(state.source_position(), 3);
    }

    let mut encoded = [0_u8; 4];
    for byte in &mut encoded {
        let step = state.update(b"", core::slice::from_mut(byte)).unwrap();
        assert_eq!(step.progress().input_consumed(), 0);
        assert_eq!(step.progress().output_produced(), 1);
    }
    assert_eq!(&encoded, b"Zm9v");
    assert_eq!(
        state.update(b"", &mut no_output).unwrap().status(),
        Status::NeedInput
    );
    assert_eq!(
        state.finish(&mut no_output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(
        state.finish(&mut no_output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(
        state.update(b"x", &mut no_output),
        Err(OperationError::Terminal(TerminalError::InputAfterComplete))
    );
}

#[test]
fn final_tails_retry_one_byte_at_a_time_for_padded_and_unpadded_codecs() {
    for profile in profiles() {
        for input in [&b"f"[..], &b"fo"[..]] {
            let expected = oracle::encode(profile, input);
            let mut state = encoder(profile);
            let mut no_output = [];
            let update = state.update(input, &mut no_output).unwrap();
            assert_eq!(update.status(), Status::NeedInput);
            assert_eq!(update.progress().input_consumed(), input.len());

            let mut actual = Vec::new();
            loop {
                let mut byte = [0_u8; 1];
                let step = state.finish(&mut byte).unwrap();
                actual.extend_from_slice(&byte[..step.progress().output_produced()]);
                match step.status() {
                    Status::OutputFull(requirement) => {
                        assert_eq!(requirement.minimum_output().get(), 1);
                    }
                    Status::Complete => break,
                    Status::NeedInput => panic!("finish requested more input"),
                }
            }
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn finish_closes_encoder_input_before_pending_output_drains() {
    let mut state = STRICT_STANDARD_PADDED.encoder();
    state.update(b"f", &mut []).unwrap();
    let finishing = state.finish(&mut []).unwrap();
    assert_output_full(finishing.status());
    assert_eq!(
        state.update(b"x", &mut []),
        Err(OperationError::Terminal(TerminalError::InputAfterFinish))
    );
    let mut output = [0_u8; 4];
    assert_eq!(
        state.finish(&mut output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(&output, b"Zg==");
}

#[test]
fn runtime_custom_alphabet_uses_the_validated_owned_table() {
    const CUSTOM: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let codec = CodecBuilder::from_table(CUSTOM)
        .unwrap()
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .build()
        .unwrap();
    let mut state = codec.encoder();
    let mut output = [0_u8; 16];
    let update = state.update(b"custom", &mut output).unwrap();
    let written = update.progress().output_produced();
    let finish = state.finish(&mut output[written..]).unwrap();
    let total = written + finish.progress().output_produced();

    assert_eq!(&output[..total], b"W1TxbE7r");
}

#[test]
fn reset_discards_ordinary_pending_state_without_drop_overhead() {
    assert!(!core::mem::needs_drop::<EncoderState>());
    let mut state = STRICT_STANDARD_PADDED.encoder();
    let mut no_output = [];
    state.update(b"foobar", &mut no_output).unwrap();
    state.reset();
    assert_eq!(state.source_position(), 0);

    let actual = drive(&mut state, b"f", &[1], &[1]);
    assert_eq!(actual, b"Zg==");
}

fn assert_every_partition(profile: Profile, input: &[u8], expected: &[u8]) {
    let input_masks = partition_masks(input.len());
    let output_masks = partition_masks(expected.len());
    for input_mask in input_masks {
        let input_chunks = partition_lengths(input.len(), input_mask);
        for &output_mask in &output_masks {
            let output_chunks = partition_lengths(expected.len(), output_mask);
            let mut state = encoder(profile);
            let actual = drive(&mut state, input, &input_chunks, &output_chunks);
            assert_eq!(actual, expected);
        }
    }
}

fn drive(
    state: &mut EncoderState,
    input: &[u8],
    input_chunks: &[usize],
    output_chunks: &[usize],
) -> Vec<u8> {
    let mut output = Vec::new();
    let mut input_start = 0;
    let mut output_chunk = 0;

    for &input_len in input_chunks {
        let input_end = input_start + input_len;
        let mut accepted = input_start;
        loop {
            let capacity = output_chunks.get(output_chunk).copied().unwrap_or(1);
            output_chunk += 1;
            let mut destination = [0_u8; 16];
            let step = state
                .update(&input[accepted..input_end], &mut destination[..capacity])
                .unwrap();
            accepted += step.progress().input_consumed();
            output.extend_from_slice(&destination[..step.progress().output_produced()]);
            match step.status() {
                Status::NeedInput => {
                    assert_eq!(accepted, input_end);
                    break;
                }
                Status::OutputFull(requirement) => {
                    assert_eq!(requirement.minimum_output().get(), 1);
                }
                Status::Complete => panic!("update completed before finish"),
            }
        }
        input_start = input_end;
    }

    loop {
        let capacity = output_chunks.get(output_chunk).copied().unwrap_or(1);
        output_chunk += 1;
        let mut destination = [0_u8; 16];
        let step = state.finish(&mut destination[..capacity]).unwrap();
        output.extend_from_slice(&destination[..step.progress().output_produced()]);
        match step.status() {
            Status::Complete => break,
            Status::OutputFull(requirement) => {
                assert_eq!(requirement.minimum_output().get(), 1);
            }
            Status::NeedInput => panic!("finish requested more input"),
        }
    }
    output
}

fn encoder(profile: Profile) -> EncoderState {
    EncoderState::new(match profile {
        Profile::StandardPadded => STRICT_STANDARD_PADDED.settings(),
        Profile::StandardUnpadded => STRICT_STANDARD_UNPADDED.settings(),
        Profile::UrlSafePadded => STRICT_URL_SAFE_PADDED.settings(),
        Profile::UrlSafeUnpadded => STRICT_URL_SAFE_UNPADDED.settings(),
    })
}

const fn profiles() -> [Profile; 4] {
    [
        Profile::StandardPadded,
        Profile::StandardUnpadded,
        Profile::UrlSafePadded,
        Profile::UrlSafeUnpadded,
    ]
}

const fn is_padded(profile: Profile) -> bool {
    matches!(profile, Profile::StandardPadded | Profile::UrlSafePadded)
}

fn assert_output_full(status: Status) {
    let Status::OutputFull(requirement) = status else {
        panic!("expected output-full status");
    };
    assert_eq!(requirement.minimum_output().get(), 1);
}

fn partition_masks(len: usize) -> Vec<usize> {
    if len <= 1 {
        return vec![0];
    }
    (0..(1_usize << (len - 1))).collect()
}

fn partition_lengths(len: usize, mask: usize) -> Vec<usize> {
    if len == 0 {
        return vec![0];
    }
    let mut lengths = Vec::new();
    let mut start = 0;
    for boundary in 1..len {
        if mask & (1 << (boundary - 1)) != 0 {
            lengths.push(boundary - start);
            start = boundary;
        }
    }
    lengths.push(len - start);
    lengths
}
