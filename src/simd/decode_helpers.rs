use crate::Alphabet;

pub(in crate::simd) fn copy_verified_decode_output<const PACKED: usize, const SCALAR: usize>(
    packed: &mut [u8; PACKED],
    scalar_output: &mut [u8; SCALAR],
    output: &mut [u8],
    written: usize,
) -> Result<(), crate::DecodeError> {
    if packed[..written] != scalar_output[..written] {
        crate::wipe_bytes(packed);
        crate::wipe_bytes(scalar_output);
        return Err(crate::DecodeError::InvalidInput);
    }

    output[..written].copy_from_slice(&packed[..written]);
    crate::wipe_bytes(packed);
    crate::wipe_bytes(scalar_output);
    Ok(())
}

pub(in crate::simd) fn fill_decode_values<A, const N: usize>(input: &[u8; N], values: &mut [u8; N])
where
    A: Alphabet,
{
    let mut index = 0;
    while index < input.len() {
        values[index] = match input[index] {
            b'=' => 0,
            byte => {
                if let Some(value) = A::decode(byte) {
                    value
                } else {
                    debug_assert!(false, "fill_decode_values called on unvalidated input");
                    0
                }
            }
        };
        index += 1;
    }
}
