extern crate std;

use std::{vec, vec::Vec};

use super::{
    contracts::{Failure, InputError, OperationError, Status, TerminalError},
    incremental_decoder::DecoderState,
    rfc4648_oracle::{self as oracle, Profile},
    specifications::{
        CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
        STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_UNPADDED,
    },
};

const RFC_VECTORS: &[(&[u8], &[u8])] = &[
    (b"", b""),
    (b"f", b"Zg"),
    (b"fo", b"Zm8"),
    (b"foo", b"Zm9v"),
    (b"foob", b"Zm9vYg"),
    (b"fooba", b"Zm9vYmE"),
    (b"foobar", b"Zm9vYmFy"),
];

#[test]
fn rfc4648_unpadded_vectors_pass_every_input_and_output_partition() {
    for &(expected, encoded) in RFC_VECTORS {
        for profile in profiles() {
            assert_every_valid_partition(profile, encoded, expected);
        }
    }
}

#[test]
fn bounded_unpadded_inputs_match_independent_oracle_for_every_partition() {
    let mut plain = [0_u8; 6];
    for len in 0..=plain.len() {
        for (index, byte) in plain[..len].iter_mut().enumerate() {
            *byte = u8::try_from((index * 71 + len * 43) % 256).unwrap();
        }
        for profile in profiles() {
            let encoded = oracle::encode(profile, &plain[..len]);
            assert_every_valid_partition(profile, &encoded, &plain[..len]);
        }
    }
}

#[test]
fn every_short_unpadded_alphabet_tail_is_classified_and_decoded() {
    for profile in profiles() {
        let alphabet = alphabet(profile);
        for first in 0..64 {
            let one = [alphabet[first]];
            assert_eq!(finish_tail(profile, &one), Err(InputError::InvalidLength));

            for second in 0..64 {
                let two = [alphabet[first], alphabet[second]];
                if second.is_multiple_of(16) {
                    let expected = [(u8_value(first) << 2) | (u8_value(second) >> 4), 0];
                    assert_eq!(finish_tail(profile, &two), Ok((1, expected)));
                } else {
                    assert_eq!(
                        finish_tail(profile, &two),
                        Err(InputError::NonCanonicalTrailingBits { index: 1 })
                    );
                }

                for third in 0..64 {
                    let three = [alphabet[first], alphabet[second], alphabet[third]];
                    if third.is_multiple_of(4) {
                        let expected = [
                            (u8_value(first) << 2) | (u8_value(second) >> 4),
                            (u8_value(second) << 4) | (u8_value(third) >> 2),
                        ];
                        assert_eq!(finish_tail(profile, &three), Ok((2, expected)));
                    } else {
                        assert_eq!(
                            finish_tail(profile, &three),
                            Err(InputError::NonCanonicalTrailingBits { index: 2 })
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn unpadded_mode_rejects_every_padding_position_and_optional_padding() {
    for profile in profiles() {
        for position in 0..4 {
            let mut input = *b"AAAA";
            input[position] = b'=';
            assert_every_malformed_partition(
                profile,
                &input,
                InputError::InvalidPadding { index: position },
            );
        }
        assert_every_malformed_partition(
            profile,
            b"AAAA=",
            InputError::InvalidPadding { index: 4 },
        );
        assert_every_malformed_partition(profile, b"Zg==", InputError::InvalidPadding { index: 2 });
    }
}

#[test]
fn impossible_and_noncanonical_unpadded_tails_fail_absorbingly() {
    for profile in profiles() {
        assert_every_malformed_partition(profile, b"A", InputError::InvalidLength);
        for (value, byte) in alphabet(profile).iter().copied().enumerate() {
            if value & 0x0f != 0 {
                assert_every_malformed_partition(
                    profile,
                    &[b'A', byte],
                    InputError::NonCanonicalTrailingBits { index: 1 },
                );
            }
            if value & 0x03 != 0 {
                assert_every_malformed_partition(
                    profile,
                    &[b'A', b'A', byte],
                    InputError::NonCanonicalTrailingBits { index: 2 },
                );
            }
        }
    }
}

#[test]
fn unpadded_finish_retries_one_byte_at_a_time_without_input_replay() {
    let mut decoder = STRICT_STANDARD_UNPADDED.decoder();
    let update = decoder.update(b"Zm8", &mut []).unwrap();
    assert_eq!(update.progress().input_consumed(), 3);
    assert!(matches!(update.status(), Status::NeedInput));

    let stalled = decoder.finish(&mut []).unwrap();
    assert_eq!(stalled.progress().output_produced(), 0);
    assert_output_full(stalled.status());
    let repeated = decoder.finish(&mut []).unwrap();
    assert_eq!(repeated.progress().output_produced(), 0);
    assert_output_full(repeated.status());
    assert_eq!(
        decoder.update(b"A", &mut []),
        Err(OperationError::Terminal(TerminalError::InputAfterFinish))
    );

    let mut output = [0_u8; 2];
    let first = decoder.finish(&mut output[..1]).unwrap();
    assert_eq!(first.progress().input_consumed(), 0);
    assert_eq!(first.progress().output_produced(), 1);
    assert_output_full(first.status());
    let second = decoder.finish(&mut output[1..]).unwrap();
    assert_eq!(second.progress().input_consumed(), 0);
    assert_eq!(second.progress().output_produced(), 1);
    assert!(matches!(second.status(), Status::Complete));
    assert_eq!(&output, b"fo");

    let complete = decoder.finish(&mut []).unwrap();
    assert_eq!(complete.progress().output_produced(), 0);
    assert!(matches!(complete.status(), Status::Complete));
}

#[test]
fn padded_and_unpadded_modes_share_full_quantum_results_but_not_tail_policy() {
    let encoded = b"Zm9v";
    let padded = drive_valid(
        STRICT_STANDARD_PADDED.decoder(),
        encoded,
        &[1, 1, 1, 1],
        &[1; 3],
    );
    let unpadded = drive_valid(
        STRICT_STANDARD_UNPADDED.decoder(),
        encoded,
        &[1, 1, 1, 1],
        &[1; 3],
    );
    assert_eq!(padded, b"foo");
    assert_eq!(unpadded, padded);

    let mut padded = STRICT_STANDARD_PADDED.decoder();
    padded.update(b"Zg", &mut []).unwrap();
    assert_eq!(
        padded.finish(&mut [0xa5]),
        Err(OperationError::Failed(Failure::Input(
            InputError::TruncatedInput { index: 2 }
        )))
    );
    assert_eq!(
        drive_valid(STRICT_STANDARD_UNPADDED.decoder(), b"Zg", &[1, 1], &[1]),
        b"f"
    );
}

#[test]
fn validated_runtime_alphabet_supports_strict_unpadded_finalization() {
    const CUSTOM: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let codec = CodecBuilder::from_table(CUSTOM)
        .unwrap()
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .build()
        .unwrap();
    let encoded = oracle::encode(Profile::StandardUnpadded, b"custom");
    let translated: Vec<u8> = encoded
        .into_iter()
        .map(|byte| {
            let value = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                .iter()
                .position(|candidate| *candidate == byte)
                .unwrap();
            CUSTOM[value]
        })
        .collect();
    let actual = drive_valid(
        DecoderState::new_unpadded(codec.settings()),
        &translated,
        &[1; 8],
        &[1; 6],
    );
    assert_eq!(actual, b"custom");
}

fn assert_every_valid_partition(profile: Profile, input: &[u8], expected: &[u8]) {
    for input_mask in partition_masks(input.len()) {
        let input_chunks = partition_lengths(input.len(), input_mask);
        for output_mask in partition_masks(expected.len()) {
            let output_chunks = partition_lengths(expected.len(), output_mask);
            let actual = drive_valid(decoder(profile), input, &input_chunks, &output_chunks);
            assert_eq!(actual, expected);
        }
    }
}

fn drive_valid(
    mut state: DecoderState,
    input: &[u8],
    input_chunks: &[usize],
    output_chunks: &[usize],
) -> Vec<u8> {
    let mut output = Vec::new();
    let mut accepted = 0;
    let mut output_chunk = 0;
    for &input_len in input_chunks {
        let end = accepted + input_len;
        while accepted < end || input_len == 0 {
            let capacity = output_chunks.get(output_chunk).copied().unwrap_or(1);
            output_chunk += 1;
            let mut destination = [0_u8; 8];
            let step = state
                .update(&input[accepted..end], &mut destination[..capacity])
                .unwrap();
            accepted += step.progress().input_consumed();
            output.extend_from_slice(&destination[..step.progress().output_produced()]);
            if matches!(step.status(), Status::NeedInput) {
                assert_eq!(accepted, end);
                break;
            }
            assert_output_full(step.status());
        }
    }
    loop {
        let capacity = output_chunks.get(output_chunk).copied().unwrap_or(1);
        output_chunk += 1;
        let mut destination = [0_u8; 8];
        let step = state.finish(&mut destination[..capacity]).unwrap();
        output.extend_from_slice(&destination[..step.progress().output_produced()]);
        if matches!(step.status(), Status::Complete) {
            break;
        }
        assert_output_full(step.status());
    }
    output
}

fn finish_tail(profile: Profile, input: &[u8]) -> Result<(usize, [u8; 2]), InputError> {
    let mut state = decoder(profile);
    state.update(input, &mut []).unwrap();
    let mut output = [0_u8; 2];
    match state.finish(&mut output) {
        Ok(step) => Ok((step.progress().output_produced(), output)),
        Err(OperationError::Failed(Failure::Input(error))) => Err(error),
        Err(error) => panic!("unexpected operation error: {error:?}"),
    }
}

fn assert_every_malformed_partition(profile: Profile, input: &[u8], expected: InputError) {
    for mask in partition_masks(input.len()) {
        let mut state = decoder(profile);
        let chunks = partition_lengths(input.len(), mask);
        let mut start = 0;
        let mut found = None;
        for len in chunks {
            let end = start + len;
            let mut destination = [0xa5_u8; 3];
            let before = destination;
            match state.update(&input[start..end], &mut destination) {
                Ok(step) => {
                    assert_eq!(step.progress().input_consumed(), len);
                    start = end;
                }
                Err(error) => {
                    assert_eq!(destination, before);
                    found = Some(error);
                    break;
                }
            }
        }
        let error = found.unwrap_or_else(|| {
            let mut destination = [0xa5_u8; 3];
            let before = destination;
            let error = state.finish(&mut destination).unwrap_err();
            assert_eq!(destination, before);
            error
        });
        let expected = OperationError::Failed(Failure::Input(expected));
        assert_eq!(error, expected);
        assert_eq!(state.update(b"", &mut []), Err(expected));
        assert_eq!(state.finish(&mut []), Err(expected));
    }
}

fn decoder(profile: Profile) -> DecoderState {
    match profile {
        Profile::StandardUnpadded => STRICT_STANDARD_UNPADDED.decoder(),
        Profile::UrlSafeUnpadded => STRICT_URL_SAFE_UNPADDED.decoder(),
        Profile::StandardPadded | Profile::UrlSafePadded => {
            panic!("Commit 11 helper accepts unpadded profiles only")
        }
    }
}

const fn profiles() -> [Profile; 2] {
    [Profile::StandardUnpadded, Profile::UrlSafeUnpadded]
}

const fn alphabet(profile: Profile) -> &'static [u8; 64] {
    match profile {
        Profile::StandardUnpadded => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        }
        Profile::UrlSafeUnpadded => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        }
        Profile::StandardPadded | Profile::UrlSafePadded => {
            panic!("Commit 11 helper accepts unpadded profiles only")
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

fn u8_value(value: usize) -> u8 {
    u8::try_from(value).unwrap()
}

#[test]
fn failed_unpadded_state_rejects_terminal_and_new_input_until_reset() {
    let mut state = STRICT_STANDARD_UNPADDED.decoder();
    state.update(b"A", &mut []).unwrap();
    let expected = OperationError::Failed(Failure::Input(InputError::InvalidLength));
    assert_eq!(state.finish(&mut []), Err(expected));
    assert_eq!(state.update(b"AAAA", &mut [0; 3]), Err(expected));
    assert_eq!(state.finish(&mut []), Err(expected));
    state.reset();
    assert_eq!(
        drive_valid(state, b"Zg", &[2], &[1]),
        b"f",
        "reset starts an unrelated unpadded message"
    );
}

#[test]
fn completed_unpadded_decoder_rejects_new_input_until_reset() {
    let mut state = STRICT_STANDARD_UNPADDED.decoder();
    assert_eq!(drive_valid(state.clone(), b"", &[0], &[0]), b"");
    state.finish(&mut []).unwrap();
    assert_eq!(
        state.update(b"A", &mut []),
        Err(OperationError::Terminal(TerminalError::InputAfterComplete))
    );
}
