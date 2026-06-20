use super::decode::{ct_decode_alphabet_byte, ct_padding_len, read_quad_or_mark_invalid};
use super::equality::{ct_error_gate_barrier, ct_mask_eq_u8, ct_mask_nonzero_u8, report_ct_error};
use crate::{Alphabet, DecodeError, wipe_bytes};

pub(crate) fn ct_padded_final_quantum<A: Alphabet>(
    input: [u8; 4],
    padding: usize,
) -> ([u8; 3], u8, u8, usize) {
    let [b0, b1, b2, b3] = input;
    let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
    let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
    let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
    let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

    let padding_byte = match padding {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => return ([0; 3], 0xff, 0xff, 0),
    };
    let no_padding = ct_mask_eq_u8(padding_byte, 0);
    let one_padding = ct_mask_eq_u8(padding_byte, 1);
    let two_padding = ct_mask_eq_u8(padding_byte, 2);
    let require_v2 = no_padding | one_padding;
    let require_v3 = no_padding;

    let invalid_byte = !valid0 | !valid1 | (!valid2 & require_v2) | (!valid3 & require_v3);
    let invalid_padding = (ct_mask_nonzero_u8(v1 & 0b0000_1111) & two_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_nonzero_u8(v2 & 0b0000_0011)) & one_padding)
        | ((ct_mask_eq_u8(b2, b'=') | ct_mask_eq_u8(b3, b'=')) & no_padding);

    (
        [(v0 << 2) | (v1 >> 4), (v1 << 4) | (v2 >> 2), (v2 << 6) | v3],
        invalid_byte,
        invalid_padding,
        3 - padding,
    )
}

pub(super) fn ct_decode_padded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let required = input.len() / 4 * 3 - padding;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < input.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding);
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    output[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

pub(super) fn ct_decode_padded_in_place<A: Alphabet>(
    buffer: &mut [u8],
) -> Result<usize, DecodeError> {
    if !buffer.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(buffer);
    let required = buffer.len() / 4 * 3 - padding;
    if required > buffer.len() {
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 < buffer.len() {
        let [b0, b1, b2, b3] =
            read_quad_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
        let (v0, valid0) = ct_decode_alphabet_byte::<A>(b0);
        let (v1, valid1) = ct_decode_alphabet_byte::<A>(b1);
        let (v2, valid2) = ct_decode_alphabet_byte::<A>(b2);
        let (v3, valid3) = ct_decode_alphabet_byte::<A>(b3);

        invalid_byte |= !valid0;
        invalid_byte |= !valid1;
        invalid_byte |= !valid2;
        invalid_byte |= !valid3;
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');
        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        write += 3;
        read += 4;
    }

    let final_chunk =
        read_quad_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
    let (final_bytes, final_invalid_byte, final_invalid_padding, final_written) =
        ct_padded_final_quantum::<A>(final_chunk, padding);
    invalid_byte |= final_invalid_byte;
    invalid_padding |= final_invalid_padding;
    buffer[write..write + final_written].copy_from_slice(&final_bytes[..final_written]);
    write += final_written;

    if write != required {
        ct_error_gate_barrier(invalid_byte, invalid_padding);
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }
    if let Err(err) = report_ct_error(invalid_byte, invalid_padding) {
        wipe_bytes(buffer);
        return Err(err);
    }
    Ok(write)
}
