//! Portable lookup-table decoding for ordinary strict RFC 4648 codecs only.
//! These input-indexed tables are deliberately not a constant-time primitive.

use super::specifications::{CodecSettings, DecodePadding, EncodePadding, TrailingBits};

const INVALID: u8 = 0x80;
const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
static STANDARD_VALUES: [u8; 256] = values(STANDARD);
static URL_SAFE_VALUES: [u8; 256] = values(URL_SAFE);

const fn values(alphabet: &[u8; 64]) -> [u8; 256] {
    let mut table = [INVALID; 256];
    let mut value = 0u8;
    while value < 64 {
        table[alphabet[value as usize] as usize] = value;
        value += 1;
    }
    table
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Family {
    Standard,
    UrlSafe,
}

impl Family {
    pub(super) fn for_settings(settings: CodecSettings) -> Option<Self> {
        if settings.trailing_bits() != TrailingBits::RequireCanonical
            || !matches!(
                (settings.encode_padding(), settings.decode_padding()),
                (EncodePadding::Padded, DecodePadding::RequireCanonical)
                    | (EncodePadding::Unpadded, DecodePadding::Forbid)
            )
        {
            return None;
        }
        match settings.alphabet().as_array() {
            table if table == STANDARD => Some(Self::Standard),
            table if table == URL_SAFE => Some(Self::UrlSafe),
            _ => None,
        }
    }

    pub(super) const fn table(self) -> &'static [u8; 256] {
        match self {
            Self::Standard => &STANDARD_VALUES,
            Self::UrlSafe => &URL_SAFE_VALUES,
        }
    }

    /// Returns exact length only after complete validation. Rejections carry
    /// no diagnostics; callers rerun the original validator for error parity.
    pub(super) fn validated_len(self, input: &[u8], padded: bool) -> Option<usize> {
        #[cfg(test)]
        crate::decode_validation::observation::record_fast();
        if input.is_empty() {
            return Some(0);
        }
        if (padded && !input.len().is_multiple_of(4)) || input.len() % 4 == 1 {
            return None;
        }
        let table = self.table();
        let body_len = (input.len() - 1) / 4 * 4;
        for quad in input[..body_len].as_chunks::<4>().0 {
            let flags = table[usize::from(quad[0])]
                | table[usize::from(quad[1])]
                | table[usize::from(quad[2])]
                | table[usize::from(quad[3])];
            if flags & INVALID != 0 {
                return None;
            }
        }
        let tail = &input[body_len..];
        let first = table[usize::from(tail[0])];
        let second = table[usize::from(tail[1])];
        if (first | second) & INVALID != 0 {
            return None;
        }
        let tail_len = match tail {
            [_, _] if !padded && second.is_multiple_of(16) => 1,
            [_, _, b'=', b'='] if padded && second.is_multiple_of(16) => 1,
            [_, _, third] if !padded => {
                let third = table[usize::from(*third)];
                if third & (INVALID | 3) != 0 {
                    return None;
                }
                2
            }
            [_, _, third, b'='] if padded => {
                let third = table[usize::from(*third)];
                if third & (INVALID | 3) != 0 {
                    return None;
                }
                2
            }
            [_, _, third, fourth] => {
                if (table[usize::from(*third)] | table[usize::from(*fourth)]) & INVALID != 0 {
                    return None;
                }
                3
            }
            _ => return None,
        };
        (body_len / 4).checked_mul(3)?.checked_add(tail_len)
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
pub(crate) fn validated_len_for_proof(input: &[u8], padded: bool, url_safe: bool) -> Option<usize> {
    let family = if url_safe {
        Family::UrlSafe
    } else {
        Family::Standard
    };
    family.validated_len(input, padded)
}
