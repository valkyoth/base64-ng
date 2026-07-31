//! Secret codec and high-assurance policy ownership boundary.

use super::alphabet::{ALPHABET_LEN, ValidatedAlphabet};
use crate::ct_mask_eq_u8;

/// Maps one encoded symbol through a crate-owned fixed 64-entry scan.
///
/// Commit 5 uses this helper for semantic evidence only. Commit 20 owns the
/// optimizer barriers, result gate, and complete secret-codec timing claim.
#[allow(dead_code)]
pub(super) const fn decode_alphabet_byte(alphabet: &ValidatedAlphabet, byte: u8) -> Option<u8> {
    let mut index = 0;
    let mut candidate = 0u8;
    let mut decoded = 0u8;
    let mut valid = 0u8;
    while index < ALPHABET_LEN {
        let matches = ct_mask_eq_u8(byte, alphabet.as_array()[index]);
        decoded |= candidate & matches;
        valid |= matches;
        index += 1;
        candidate += 1;
    }

    if valid == 0 { None } else { Some(decoded) }
}
