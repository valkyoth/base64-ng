use super::{
    STANDARD, Standard, checked_encoded_len, ct, decode_byte, decode_chunk, decode_tail_unpadded,
    decoded_capacity, validate_tail_unpadded,
};

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
