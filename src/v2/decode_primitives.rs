//! Shared strict-decoding arithmetic primitives.

pub(super) const fn pack_full_quantum(first: u8, second: u8, third: u8, fourth: u8) -> [u8; 3] {
    [
        (first << 2) | (second >> 4),
        (second << 4) | (third >> 2),
        (third << 6) | fourth,
    ]
}

#[allow(clippy::verbose_bit_mask)]
pub(super) const fn one_byte_tail_is_canonical(second: u8) -> bool {
    second & 0x0f == 0
}

#[allow(clippy::verbose_bit_mask)]
pub(super) const fn two_byte_tail_is_canonical(third: u8) -> bool {
    third & 0x03 == 0
}

pub(super) const fn is_legacy_ascii_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

#[cfg(kani)]
pub(crate) const fn pack_full_quantum_for_proof(values: [u8; 4]) -> [u8; 3] {
    pack_full_quantum(values[0], values[1], values[2], values[3])
}

#[cfg(all(kani, base64_ng_kani_advanced))]
pub(crate) const fn tail_is_canonical_for_proof(value: u8, decoded_len: usize) -> bool {
    match decoded_len {
        1 => one_byte_tail_is_canonical(value),
        2 => two_byte_tail_is_canonical(value),
        _ => false,
    }
}
