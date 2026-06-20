use super::decode::{
    ct_decode_alphabet_byte, read_quad_or_mark_invalid, read_tail_or_mark_invalid,
};
use super::equality::{ct_error_gate_barrier, ct_mask_eq_u8, ct_mask_nonzero_u8, report_ct_error};
use crate::{Alphabet, DecodeError, decoded_capacity, wipe_bytes};

pub(super) fn ct_decode_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(input.len());
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

    while read + 4 <= input.len() {
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
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    match read_tail_or_mark_invalid(input, read, &mut invalid_byte, &mut invalid_padding) {
        [] => {}
        [b0, b1] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            output[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        [b0, b1, b2] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

pub(super) fn ct_decode_unpadded_in_place<A: Alphabet>(
    buffer: &mut [u8],
) -> Result<usize, DecodeError> {
    if buffer.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(buffer.len());
    if required > buffer.len() {
        wipe_bytes(buffer);
        return Err(DecodeError::InvalidInput);
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= buffer.len() {
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
        invalid_padding |= ct_mask_eq_u8(b0, b'=');
        invalid_padding |= ct_mask_eq_u8(b1, b'=');
        invalid_padding |= ct_mask_eq_u8(b2, b'=');
        invalid_padding |= ct_mask_eq_u8(b3, b'=');

        buffer[write] = (v0 << 2) | (v1 >> 4);
        buffer[write + 1] = (v1 << 4) | (v2 >> 2);
        buffer[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    let tail = read_tail_or_mark_invalid(buffer, read, &mut invalid_byte, &mut invalid_padding);
    match tail {
        [] => {}
        [b0, b1] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v1 & 0b0000_1111);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        [b0, b1, b2] => {
            let (v0, valid0) = ct_decode_alphabet_byte::<A>(*b0);
            let (v1, valid1) = ct_decode_alphabet_byte::<A>(*b1);
            let (v2, valid2) = ct_decode_alphabet_byte::<A>(*b2);
            invalid_byte |= !valid0;
            invalid_byte |= !valid1;
            invalid_byte |= !valid2;
            invalid_padding |= ct_mask_eq_u8(*b0, b'=');
            invalid_padding |= ct_mask_eq_u8(*b1, b'=');
            invalid_padding |= ct_mask_eq_u8(*b2, b'=');
            invalid_padding |= ct_mask_nonzero_u8(v2 & 0b0000_0011);
            buffer[write] = (v0 << 2) | (v1 >> 4);
            buffer[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => {
            invalid_byte = 0xff;
            invalid_padding = 0xff;
        }
    }

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
