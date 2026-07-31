use super::{
    STANDARD, Standard, checked_encoded_len, ct, decode_backend, decode_byte, decode_chunk,
    decode_tail_unpadded, decoded_capacity, scalar,
    v2::{
        alphabet::{ValidatedAlphabetError, validate_position_for_proof},
        contracts::Status,
        specifications::{STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED},
    },
    validate_tail_unpadded,
};

#[cfg(base64_ng_kani_advanced)]
use super::{LineEnding, LineWrap, Profile, STANDARD_NO_PAD};

#[kani::proof]
fn checked_encoded_len_is_bounded_for_small_inputs() {
    let len = usize::from(kani::any::<u8>());
    let padded = kani::any::<bool>();
    let encoded = checked_encoded_len(len, padded).expect("u8 input length cannot overflow");

    assert!(encoded >= len);
    assert!(encoded <= len / 3 * 4 + 4);
}

#[kani::proof]
fn decoded_capacity_is_bounded_for_small_inputs() {
    let len = usize::from(kani::any::<u8>());
    let capacity = decoded_capacity(len);

    assert!(capacity <= len / 4 * 3 + 2);
}

#[kani::proof]
#[kani::unwind(66)]
fn validated_alphabet_constructor_indexing_is_bounded() {
    let table = kani::any::<[u8; 64]>();
    let index = usize::from(kani::any::<u8>() & 63);

    match validate_position_for_proof(&table, index) {
        Ok(()) => {}
        Err(ValidatedAlphabetError::InvalidByte {
            index: error_index, ..
        })
        | Err(ValidatedAlphabetError::PaddingByte { index: error_index }) => {
            assert!(error_index == index);
        }
        Err(ValidatedAlphabetError::DuplicateByte { first, second, .. }) => {
            assert!(first == index);
            assert!(second > first);
            assert!(second < 64);
        }
        Err(ValidatedAlphabetError::InvalidLength { .. }) => {
            unreachable!("fixed arrays cannot produce a length error");
        }
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_in_place_decode_returns_prefix_within_buffer() {
    let mut buffer = kani::any::<[u8; 8]>();
    let result = STANDARD.decode_in_place(&mut buffer);

    if let Ok(decoded) = result {
        assert!(decoded.len() <= 8);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_slice_returns_written_within_output() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = STANDARD.decode_slice(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_backend_matches_scalar_for_one_quantum() {
    let input = kani::any::<[u8; 4]>();
    let mut backend_output = kani::any::<[u8; 3]>();
    let mut scalar_output = backend_output;

    let backend = decode_backend::decode_slice::<Standard, true>(&input, &mut backend_output);
    let scalar = scalar::decode_slice::<Standard, true>(&input, &mut scalar_output);

    assert!(backend == scalar);
    if backend.is_ok() {
        assert!(backend_output == scalar_output);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_chunk_returns_written_within_output() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = decode_chunk::<Standard, true>(input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
        assert!(written <= 3);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_chunk_bit_packing_matches_decoded_values() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = decode_chunk::<Standard, true>(input, &mut output);

    if let Ok(written) = result {
        let v0 = decode_byte::<Standard>(input[0], 0).expect("successful chunk has v0");
        let v1 = decode_byte::<Standard>(input[1], 1).expect("successful chunk has v1");

        assert!(output[0] == ((v0 << 2) | (v1 >> 4)));

        if written >= 2 {
            let v2 = decode_byte::<Standard>(input[2], 2).expect("successful chunk has v2");
            assert!(output[1] == ((v1 << 4) | (v2 >> 2)));
        }

        if written == 3 {
            let v2 = decode_byte::<Standard>(input[2], 2).expect("successful chunk has v2");
            let v3 = decode_byte::<Standard>(input[3], 3).expect("successful chunk has v3");
            assert!(output[2] == ((v2 << 6) | v3));
        }
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_validate_tail_unpadded_accepts_or_rejects_without_panic() {
    let input = kani::any::<[u8; 3]>();
    let len = usize::from(kani::any::<u8>() % 4);
    let result = validate_tail_unpadded::<Standard>(&input[..len]);

    if result.is_ok() {
        assert!(len == 0 || len == 2 || len == 3);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_two_byte_tail_returns_written_within_output() {
    let input = kani::any::<[u8; 2]>();
    let mut output = kani::any::<[u8; 1]>();
    let result = decode_tail_unpadded::<Standard>(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
        assert!(written == 1);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_three_byte_tail_returns_written_within_output() {
    let input = kani::any::<[u8; 3]>();
    let mut output = kani::any::<[u8; 2]>();
    let result = decode_tail_unpadded::<Standard>(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
        assert!(written == 2);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_decode_slice_clear_tail_clears_output_on_error() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = STANDARD.decode_slice_clear_tail(&input, &mut output);

    if result.is_err() {
        assert!(output.iter().all(|byte| *byte == 0));
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_encode_slice_returns_written_within_output() {
    let input = kani::any::<[u8; 3]>();
    let mut output = kani::any::<[u8; 4]>();
    let result = STANDARD.encode_slice(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn one_shot_standard_encode_is_exact_and_bounded() {
    let input = kani::any::<[u8; 3]>();
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut output = kani::any::<[u8; 4]>();
    let result = STRICT_STANDARD_PADDED.encode_into(&input[..input_len], &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
        assert!(written == STRICT_STANDARD_PADDED.encoded_len(input_len).unwrap());
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn incremental_standard_encoder_progress_and_state_are_bounded() {
    let input = kani::any::<[u8; 4]>();
    let input_len = usize::from(kani::any::<u8>() % 5);
    let mut output = kani::any::<[u8; 4]>();
    let output_len = usize::from(kani::any::<u8>() % 5);
    let mut encoder = STRICT_STANDARD_PADDED.encoder();

    let step = encoder
        .update(&input[..input_len], &mut output[..output_len])
        .expect("bounded source positions cannot fail");

    assert!(step.progress().input_consumed() <= input_len);
    assert!(step.progress().output_produced() <= output_len);
    assert!(encoder.source_position() == step.progress().input_consumed());
    assert!(encoder.proof_invariants());
}

#[kani::proof]
#[kani::unwind(12)]
fn incremental_standard_encoder_finish_is_bounded() {
    let input = kani::any::<[u8; 2]>();
    let input_len = usize::from(kani::any::<u8>() % 3);
    let mut encoder = STRICT_STANDARD_PADDED.encoder();
    let mut update_output = [0_u8; 4];
    let update = encoder
        .update(&input[..input_len], &mut update_output)
        .expect("bounded source positions cannot fail");
    assert!(update.progress().input_consumed() == input_len);

    let mut output = kani::any::<[u8; 4]>();
    let output_len = usize::from(kani::any::<u8>() % 5);
    let finish = encoder
        .finish(&mut output[..output_len])
        .expect("ordinary encoding cannot reject byte input");

    assert!(finish.progress().input_consumed() == 0);
    assert!(finish.progress().output_produced() <= output_len);
    assert!(encoder.proof_invariants());
}

#[kani::proof]
#[kani::unwind(12)]
fn incremental_padded_decoder_progress_and_retry_are_bounded() {
    let input = *b"AAAA";
    let input_len = usize::from(kani::any::<u8>() % 4);
    let mut output = kani::any::<[u8; 3]>();
    let output_len = usize::from(kani::any::<u8>() % 4);
    let mut decoder = STRICT_STANDARD_PADDED.decoder();

    let step = decoder
        .update(&input[..input_len], &mut output[..output_len])
        .expect("a partial canonical quantum cannot fail");
    assert!(step.progress().input_consumed() == input_len);
    assert!(step.progress().output_produced() == 0);
    assert!(decoder.source_position() == input_len);
    assert!(decoder.proof_invariants());

    let mut retry_decoder = STRICT_STANDARD_PADDED.decoder();
    let full = retry_decoder
        .update(&input, &mut [])
        .expect("a canonical quantum cannot fail");
    assert!(matches!(full.status(), Status::OutputFull(_)));
    assert!(full.progress().input_consumed() == input.len());
    let mut retry_output = [0_u8; 1];
    let retry = retry_decoder
        .update(&[], &mut retry_output)
        .expect("one output byte must drain pending decoder state");
    assert!(retry.progress().input_consumed() == 0);
    assert!(retry.progress().output_produced() == 1);
    assert!(retry_decoder.proof_invariants());
}

#[kani::proof]
#[kani::unwind(12)]
fn incremental_unpadded_decoder_finish_and_retry_are_bounded() {
    let input = *b"AAA";
    let input_len = 2 + usize::from(kani::any::<u8>() % 2);
    let mut decoder = STRICT_STANDARD_UNPADDED.decoder();
    let update = decoder
        .update(&input[..input_len], &mut [])
        .expect("a partial canonical unpadded tail cannot fail before finish");
    assert!(update.progress().input_consumed() == input_len);
    assert!(update.progress().output_produced() == 0);

    let mut output = kani::any::<[u8; 2]>();
    let output_len = usize::from(kani::any::<u8>() % 3);
    let finish = decoder
        .finish(&mut output[..output_len])
        .expect("zero-valued tail symbols are canonical");
    assert!(finish.progress().input_consumed() == 0);
    assert!(finish.progress().output_produced() <= output_len);
    assert!(decoder.proof_invariants());

    if matches!(finish.status(), Status::OutputFull(_)) {
        let mut retry_output = [0_u8; 1];
        let retry = decoder
            .finish(&mut retry_output)
            .expect("one byte must drain pending final output");
        assert!(retry.progress().input_consumed() == 0);
        assert!(retry.progress().output_produced() == 1);
        assert!(decoder.proof_invariants());
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_encode_in_place_returns_prefix_within_buffer() {
    let mut buffer = kani::any::<[u8; 8]>();
    let input_len = usize::from(kani::any::<u8>() % 9);
    let result = STANDARD.encode_in_place(&mut buffer, input_len);

    if let Ok(encoded) = result {
        assert!(encoded.len() <= 8);
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn standard_clear_tail_decode_clears_buffer_on_error() {
    let mut buffer = kani::any::<[u8; 4]>();
    let result = STANDARD.decode_in_place_clear_tail(&mut buffer);

    if result.is_err() {
        assert!(buffer.iter().all(|byte| *byte == 0));
    }
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn advanced_wrapped_standard_decode_slice_returns_written_within_output() {
    let input = kani::any::<[u8; 8]>();
    let mut output = kani::any::<[u8; 6]>();
    let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    let result = profile.decode_slice(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
    }
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn advanced_wrapped_standard_decode_clear_tail_clears_output_on_error() {
    let input = kani::any::<[u8; 8]>();
    let mut output = kani::any::<[u8; 6]>();
    let profile = Profile::new(STANDARD, Some(LineWrap::new(4, LineEnding::Lf)));
    let result = profile.decode_slice_clear_tail(&input, &mut output);

    if result.is_err() {
        assert!(output.iter().all(|byte| *byte == 0));
    }
}

#[cfg(base64_ng_kani_advanced)]
#[kani::proof]
#[kani::unwind(70)]
fn advanced_public_strict_decode_surfaces_do_not_panic_for_bounded_inputs() {
    let input = kani::any::<[u8; 8]>();
    let mut output = kani::any::<[u8; 6]>();
    let mut in_place = input;

    let _ = STANDARD.decode_slice(&input, &mut output);
    let _ = STANDARD.decode_slice_clear_tail(&input, &mut output);
    let _ = STANDARD.validate_result(&input);
    let _ = STANDARD.decoded_len(&input);
    let _ = STANDARD.decode_in_place(&mut in_place);
    let _ = STANDARD_NO_PAD.decode_slice(&input, &mut output);
    let _ = STANDARD_NO_PAD.validate_result(&input);
    let _ = STANDARD_NO_PAD.decoded_len(&input);
}

#[kani::proof]
#[kani::unwind(70)]
fn ct_standard_decode_slice_returns_written_within_output() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = ct::STANDARD.decode_slice_clear_tail(&input, &mut output);

    if let Ok(written) = result {
        assert!(written <= output.len());
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn ct_standard_decode_slice_clear_tail_clears_output_on_error() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();
    let result = ct::STANDARD.decode_slice_clear_tail(&input, &mut output);

    if result.is_err() {
        assert!(output.iter().all(|byte| *byte == 0));
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn ct_standard_decode_in_place_clear_tail_clears_buffer_on_error() {
    let mut buffer = kani::any::<[u8; 4]>();
    let result = ct::STANDARD.decode_in_place_clear_tail(&mut buffer);

    if result.is_err() {
        assert!(buffer.iter().all(|byte| *byte == 0));
    }
}

#[kani::proof]
#[kani::unwind(70)]
fn ct_standard_validate_matches_decode_for_one_quantum() {
    let input = kani::any::<[u8; 4]>();
    let mut output = kani::any::<[u8; 3]>();

    let validate_ok = ct::STANDARD.validate_result(&input).is_ok();
    let decode_ok = ct::STANDARD
        .decode_slice_clear_tail(&input, &mut output)
        .is_ok();

    assert!(validate_ok == decode_ok);
}
