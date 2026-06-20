use super::equality::{ct_accumulate_u8, ct_mask_eq_u8, ct_mask_nonzero_u8, report_ct_error};
use super::padded::{ct_decode_padded, ct_decode_padded_in_place, ct_padded_final_quantum};
use super::unpadded::{ct_decode_unpadded, ct_decode_unpadded_in_place};
use crate::{Alphabet, DecodeError, read_quad, wipe_bytes, wipe_tail};

pub(super) fn ct_decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded::<A>(input, output)
    } else {
        ct_decode_unpadded::<A>(input, output)
    }
}

pub(super) fn ct_decode_slice_staged_clear_tail<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    staging: &mut [u8],
) -> Result<usize, DecodeError> {
    let required = match ct_decoded_len::<A, PAD>(input) {
        Ok(required) => required,
        Err(err) => {
            wipe_bytes(output);
            wipe_bytes(staging);
            return Err(err);
        }
    };

    if output.len() < required {
        wipe_bytes(output);
        wipe_bytes(staging);
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    if staging.len() < required {
        wipe_bytes(output);
        wipe_bytes(staging);
        return Err(DecodeError::StagingTooSmall {
            required,
            available: staging.len(),
        });
    }

    let written = match ct_decode_slice::<A, PAD>(input, &mut staging[..required]) {
        Ok(written) => written,
        Err(err) => {
            wipe_bytes(output);
            wipe_bytes(staging);
            return Err(err);
        }
    };

    output[..written].copy_from_slice(&staging[..written]);
    wipe_bytes(staging);
    wipe_tail(output, written);
    Ok(written)
}

pub(super) fn ct_decode_in_place<A: Alphabet, const PAD: bool>(
    buffer: &mut [u8],
) -> Result<usize, DecodeError> {
    if buffer.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded_in_place::<A>(buffer)
    } else {
        ct_decode_unpadded_in_place::<A>(buffer)
    }
}

pub(super) fn ct_validate_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<(), DecodeError> {
    if input.is_empty() {
        return Ok(());
    }

    if PAD {
        ct_validate_padded::<A>(input)
    } else {
        ct_validate_unpadded::<A>(input)
    }
}

pub(super) fn ct_decoded_len<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<usize, DecodeError> {
    ct_validate_decode::<A, PAD>(input)?;
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        Ok(input.len() / 4 * 3 - ct_padding_len(input))
    } else {
        let full_quads = input.len() / 4 * 3;
        match input.len() % 4 {
            0 => Ok(full_quads),
            2 => Ok(full_quads + 1),
            3 => Ok(full_quads + 2),
            _ => Err(DecodeError::InvalidLength),
        }
    }
}

fn ct_validate_padded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
    let (_, final_invalid_byte, final_invalid_padding, _) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;

    report_ct_error(invalid_byte, invalid_padding)
}

fn ct_validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut read = 0;

    while read + 4 <= input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (_, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (_, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (_, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (_, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        read += 4;
    }

    match read_tail_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding) {
        [] => {}
        [b0, b1] => {
            let (_, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
        }
        [b0, b1, b2] => {
            let (_, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (_, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

    report_ct_error(invalid_byte, invalid_padding)
}

fn read_tail(input: &[u8], offset: usize) -> Result<&[u8], DecodeError> {
    input.get(offset..).ok_or(DecodeError::InvalidLength)
}

pub(super) fn read_quad_or_mark_invalid(
    input: &[u8],
    offset: usize,
    invalid_byte: &mut u8,
    invalid_padding: &mut u8,
) -> [u8; 4] {
    if let Ok(quad) = read_quad(input, offset) {
        quad
    } else {
        debug_assert!(
            false,
            "read_quad failed inside length-validated constant-time decode loop"
        );
        *invalid_byte = 0xff;
        *invalid_padding = 0xff;
        [0; 4]
    }
}

pub(super) fn read_tail_or_mark_invalid<'a>(
    input: &'a [u8],
    offset: usize,
    invalid_byte: &mut u8,
    invalid_padding: &mut u8,
) -> &'a [u8] {
    if let Ok(tail) = read_tail(input, offset) {
        tail
    } else {
        debug_assert!(
            false,
            "read_tail failed inside length-validated constant-time decode loop"
        );
        *invalid_byte = 0xff;
        *invalid_padding = 0xff;
        &[]
    }
}

#[inline(never)]
#[allow(unsafe_code)]
pub(super) fn ct_decode_alphabet_byte<A: Alphabet>(byte: u8) -> (u8, u8) {
    let mut decoded = 0u8;
    let mut valid = 0u8;
    let mut candidate = 0u8;

    while candidate < 64 {
        let matches = core::hint::black_box(ct_mask_eq_u8(
            core::hint::black_box(byte),
            core::hint::black_box(A::ENCODE[candidate as usize]),
        ));
        decoded = ct_accumulate_u8(decoded, candidate & matches);
        valid = ct_accumulate_u8(valid, matches);
        candidate += 1;
    }

    (decoded, valid)
}

pub(super) fn ct_padding_len(input: &[u8]) -> usize {
    let Some((&last, before_last_prefix)) = input.split_last() else {
        return 0;
    };
    let Some(&before_last) = before_last_prefix.last() else {
        return 0;
    };
    usize::from(ct_mask_eq_u8(last, b'=') & 1) + usize::from(ct_mask_eq_u8(before_last, b'=') & 1)
}
