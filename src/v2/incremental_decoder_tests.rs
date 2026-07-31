extern crate std;

use std::{vec, vec::Vec};

use super::{
    contracts::{Failure, InputError, OperationError, Status, TerminalError},
    incremental_decoder::DecoderState,
    rfc4648_oracle::{self as oracle, ErrorClass, Profile},
    specifications::{CodecBuilder, STRICT_STANDARD_PADDED, STRICT_URL_SAFE_PADDED},
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
    for &(expected, encoded) in RFC_VECTORS {
        for profile in profiles() {
            assert_every_valid_partition(profile, encoded, expected);
        }
    }
}

#[test]
fn bounded_binary_inputs_match_independent_oracle_for_every_partition() {
    let mut plain = [0_u8; 6];
    for len in 0..=plain.len() {
        for (index, byte) in plain[..len].iter_mut().enumerate() {
            *byte = u8::try_from((index * 67 + len * 37) % 256).unwrap();
        }
        for profile in profiles() {
            let encoded = oracle::encode(profile, &plain[..len]);
            assert_every_valid_partition(profile, &encoded, &plain[..len]);
        }
    }
}

#[test]
fn malformed_inputs_keep_exact_indexes_across_every_chunk_boundary() {
    let cases = [
        malformed(Profile::StandardPadded, b"=AAA", invalid_padding(0)),
        malformed(Profile::StandardPadded, b"A=AA", invalid_padding(1)),
        malformed(Profile::StandardPadded, b"AA=A", invalid_padding(2)),
        malformed(Profile::StandardPadded, b"Zm!v", invalid_byte(2, b'!')),
        malformed(Profile::StandardPadded, b"AAAAZm!v", invalid_byte(6, b'!')),
        malformed(Profile::StandardPadded, b"AA-A", invalid_byte(2, b'-')),
        malformed(Profile::UrlSafePadded, b"AA+A", invalid_byte(2, b'+')),
        malformed(Profile::StandardPadded, b"AB==", noncanonical(1)),
        malformed(Profile::StandardPadded, b"AAB=", noncanonical(2)),
        malformed(Profile::StandardPadded, b"AA==A", trailing(4)),
        malformed(Profile::StandardPadded, b"AAA=A", trailing(4)),
    ];

    for case in cases {
        for mask in partition_masks(case.input.len()) {
            assert_malformed_partition(case, mask);
        }
    }
}

#[test]
fn every_non_alphabet_byte_is_rejected_at_every_quantum_position() {
    for profile in profiles() {
        for value in 0_u16..=u16::from(u8::MAX) {
            let byte = u8::try_from(value).unwrap();
            if byte == b'=' || profile_alphabet(profile).contains(&byte) {
                continue;
            }
            for position in 0..4 {
                let mut input = *b"AAAA";
                input[position] = byte;
                let case = malformed(profile, &input, invalid_byte(position, byte));
                for mask in partition_masks(input.len()) {
                    assert_malformed_partition(case, mask);
                }
            }
        }
    }
}

#[test]
fn every_ascii_whitespace_byte_is_rejected_at_every_quantum_position() {
    for profile in profiles() {
        for byte in *b" \t\r\n" {
            for position in 0..4 {
                let mut input = *b"AAAA";
                input[position] = byte;
                let case = malformed(profile, &input, invalid_byte(position, byte));
                for mask in partition_masks(input.len()) {
                    assert_malformed_partition(case, mask);
                }
            }
        }
    }
}

#[test]
fn every_noncanonical_terminal_value_is_rejected_across_every_boundary() {
    for profile in profiles() {
        let alphabet = profile_alphabet(profile);
        for (value, byte) in alphabet.iter().copied().enumerate() {
            if value & 0x0f != 0 {
                let input = [b'A', byte, b'=', b'='];
                let case = malformed(profile, &input, noncanonical(1));
                for mask in partition_masks(input.len()) {
                    assert_malformed_partition(case, mask);
                }
            }
            if value & 0x03 != 0 {
                let input = [b'A', b'A', byte, b'='];
                let case = malformed(profile, &input, noncanonical(2));
                for mask in partition_masks(input.len()) {
                    assert_malformed_partition(case, mask);
                }
            }
        }
    }
}

#[test]
fn truncated_padded_inputs_fail_at_the_absolute_end_position() {
    for input in [&b"A"[..], &b"AA"[..], &b"AAA"[..], &b"Zg="[..]] {
        let case = malformed(
            Profile::StandardPadded,
            input,
            InputError::TruncatedInput { index: input.len() },
        );
        for mask in partition_masks(input.len()) {
            assert_malformed_partition(case, mask);
        }
    }
}

#[test]
fn one_byte_destinations_drain_pending_output_without_reconsuming_input() {
    let mut state = STRICT_STANDARD_PADDED.decoder();
    let mut no_output = [];
    let first = state.update(b"Zm9v", &mut no_output).unwrap();
    assert_eq!(first.progress().input_consumed(), 4);
    assert_eq!(first.progress().output_produced(), 0);
    assert_output_full(first.status());
    assert_eq!(state.source_position(), 4);

    for _ in 0..3 {
        let stalled = state.update(b"", &mut no_output).unwrap();
        assert_eq!(stalled.progress().input_consumed(), 0);
        assert_eq!(stalled.progress().output_produced(), 0);
        assert_output_full(stalled.status());
    }

    let mut decoded = [0_u8; 3];
    for byte in &mut decoded {
        let step = state.update(b"", core::slice::from_mut(byte)).unwrap();
        assert_eq!(step.progress().input_consumed(), 0);
        assert_eq!(step.progress().output_produced(), 1);
    }
    assert_eq!(&decoded, b"foo");
    assert_eq!(
        state.finish(&mut no_output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(
        state.finish(&mut no_output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(
        state.update(b"A", &mut no_output),
        Err(OperationError::Terminal(TerminalError::InputAfterComplete))
    );
}

#[test]
fn finish_closes_padded_decoder_input_before_pending_output_drains() {
    let mut state = STRICT_STANDARD_PADDED.decoder();
    state.update(b"Zm9v", &mut []).unwrap();
    let finishing = state.finish(&mut []).unwrap();
    assert_output_full(finishing.status());
    assert_eq!(
        state.update(b"A", &mut []),
        Err(OperationError::Terminal(TerminalError::InputAfterFinish))
    );
    let mut output = [0_u8; 3];
    assert_eq!(
        state.finish(&mut output).unwrap().status(),
        Status::Complete
    );
    assert_eq!(&output, b"foo");
}

#[test]
fn validated_runtime_alphabet_decodes_strict_padded_input() {
    const CUSTOM: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let codec = CodecBuilder::from_table(CUSTOM).unwrap().build().unwrap();
    let encoded = oracle::encode(Profile::StandardPadded, b"custom");
    let mut translated = Vec::with_capacity(encoded.len());
    for byte in encoded {
        translated.push(if byte == b'=' {
            b'='
        } else {
            let value = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                .iter()
                .position(|candidate| *candidate == byte)
                .unwrap();
            CUSTOM[value]
        });
    }

    let mut state = DecoderState::new_padded(codec.settings());
    let actual = drive_valid(&mut state, &translated, &[1; 8], &[1; 6]);
    assert_eq!(actual, b"custom");
}

#[test]
fn reset_discards_ordinary_decoder_state_without_drop_overhead() {
    assert!(!core::mem::needs_drop::<DecoderState>());
    let mut state = STRICT_STANDARD_PADDED.decoder();
    let mut no_output = [];
    state.update(b"Zm9v", &mut no_output).unwrap();
    state.reset();
    assert_eq!(state.source_position(), 0);
    assert_eq!(drive_valid(&mut state, b"Zg==", &[1; 4], &[1]), b"f");
}

fn assert_every_valid_partition(profile: Profile, input: &[u8], expected: &[u8]) {
    for input_mask in partition_masks(input.len()) {
        let input_chunks = partition_lengths(input.len(), input_mask);
        for output_mask in partition_masks(expected.len()) {
            let output_chunks = partition_lengths(expected.len(), output_mask);
            let mut state = decoder(profile);
            let actual = drive_valid(&mut state, input, &input_chunks, &output_chunks);
            assert_eq!(actual, expected);
        }
    }
}

fn drive_valid(
    state: &mut DecoderState,
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

fn assert_malformed_partition(case: MalformedCase<'_>, mask: usize) {
    let chunks = partition_lengths(case.input.len(), mask);
    let mut state = decoder(case.profile);
    let mut input_start = 0;
    let mut call = 0;

    for input_len in chunks {
        let input_end = input_start + input_len;
        let mut accepted = input_start;
        loop {
            let capacity = call % 4;
            call += 1;
            let mut destination = [0xa5_u8; 3];
            let before = destination;
            match state.update(
                &case.input[accepted..input_end],
                &mut destination[..capacity],
            ) {
                Ok(step) => {
                    accepted += step.progress().input_consumed();
                    match step.status() {
                        Status::NeedInput => break,
                        Status::OutputFull(_) => {}
                        Status::Complete => panic!("update completed before finish"),
                    }
                }
                Err(error) => {
                    assert_eq!(destination, before);
                    assert_absorbing(&mut state, error, case.expected);
                    assert_oracle_index_when_comparable(case);
                    return;
                }
            }
        }
        input_start = input_end;
    }

    let mut destination = [0xa5_u8; 3];
    let before = destination;
    let error = state.finish(&mut destination).unwrap_err();
    assert_eq!(destination, before);
    assert_absorbing(&mut state, error, case.expected);
}

fn assert_absorbing(state: &mut DecoderState, error: OperationError, expected: InputError) {
    let expected = OperationError::Failed(Failure::Input(expected));
    assert_eq!(error, expected);
    assert_eq!(state.update(b"", &mut []), Err(expected));
    assert_eq!(state.finish(&mut []), Err(expected));
}

fn assert_oracle_index_when_comparable(case: MalformedCase<'_>) {
    if !case.input.len().is_multiple_of(4) {
        return;
    }
    let oracle_error = oracle::decode(case.profile, case.input).unwrap_err();
    assert_eq!(oracle_error.offset, input_error_index(case.expected));
    match case.expected {
        InputError::InvalidByte { .. } => assert_eq!(oracle_error.class, ErrorClass::Byte),
        InputError::InvalidPadding { .. } | InputError::NonCanonicalTrailingBits { .. } => {
            assert_eq!(oracle_error.class, ErrorClass::Padding);
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
struct MalformedCase<'a> {
    profile: Profile,
    input: &'a [u8],
    expected: InputError,
}

const fn malformed(profile: Profile, input: &[u8], expected: InputError) -> MalformedCase<'_> {
    MalformedCase {
        profile,
        input,
        expected,
    }
}

const fn invalid_byte(index: usize, byte: u8) -> InputError {
    InputError::InvalidByte { index, byte }
}

const fn invalid_padding(index: usize) -> InputError {
    InputError::InvalidPadding { index }
}

const fn noncanonical(index: usize) -> InputError {
    InputError::NonCanonicalTrailingBits { index }
}

const fn trailing(index: usize) -> InputError {
    InputError::TrailingData { index }
}

const fn input_error_index(error: InputError) -> Option<usize> {
    match error {
        InputError::InvalidByte { index, .. }
        | InputError::InvalidPadding { index }
        | InputError::NonCanonicalTrailingBits { index }
        | InputError::TruncatedInput { index }
        | InputError::TrailingData { index }
        | InputError::InvalidLineWrap { index } => Some(index),
        InputError::InvalidLength => None,
    }
}

fn decoder(profile: Profile) -> DecoderState {
    match profile {
        Profile::StandardPadded => STRICT_STANDARD_PADDED.decoder(),
        Profile::UrlSafePadded => STRICT_URL_SAFE_PADDED.decoder(),
        Profile::StandardUnpadded | Profile::UrlSafeUnpadded => {
            panic!("Commit 10 covers padded decoding only")
        }
    }
}

const fn profiles() -> [Profile; 2] {
    [Profile::StandardPadded, Profile::UrlSafePadded]
}

const fn profile_alphabet(profile: Profile) -> &'static [u8; 64] {
    match profile {
        Profile::StandardPadded => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        }
        Profile::UrlSafePadded => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        }
        Profile::StandardUnpadded | Profile::UrlSafeUnpadded => {
            panic!("Commit 10 covers padded decoding only")
        }
    }
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
