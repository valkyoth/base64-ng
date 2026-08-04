//! Redacted secret storage and explicit exposure.
//!
//! Secret owners require deliberately named exposure or declassification
//! operations. Constant-time-oriented computation is provided by the bounded
//! secret encoder and decoder states; storage alone does not make an ordinary
//! codec safe for secret-bearing input.

use super::alphabet::{ALPHABET_LEN, ValidatedAlphabet};
use crate::ct_mask_eq_u8;

#[cfg(feature = "secrets")]
macro_rules! redacted_formatting {
    ($name:ty, $label:literal) => {
        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .debug_struct($label)
                    .field("bytes", &"<redacted>")
                    .field("len", &self.len())
                    .finish()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("<redacted secret>")
            }
        }
    };
}

#[cfg(feature = "secrets")]
mod encoders;
#[cfg(feature = "secrets")]
mod exposure;
#[cfg(feature = "secrets")]
mod frames;
#[cfg(feature = "secrets")]
mod owned;

#[cfg(all(feature = "secrets", feature = "alloc"))]
pub use encoders::SecretVecEncoder;
#[cfg(feature = "secrets")]
pub use encoders::{SecretArrayEncoder, SecretEncoder};
#[cfg(feature = "secrets")]
pub use exposure::{
    DeclassifiedOutput, ExposedSecret, ExposedSecretMut, SecretInput, SecretOutput,
};
#[cfg(all(feature = "secrets", feature = "alloc"))]
pub use frames::SecretVecFrame;
#[cfg(feature = "secrets")]
pub use frames::{SecretArrayFrame, SecretFrame};
#[cfg(all(feature = "secrets", feature = "alloc"))]
pub use owned::SecretVec;
#[cfg(feature = "secrets")]
pub use owned::{DeclassifiedArray, SecretArray};

#[cfg(feature = "secrets")]
pub use super::secret_decoder::{MAX_SECRET_STACK_DECODED, SecretDecodeError, SecretDecoderState};
#[cfg(feature = "secrets")]
pub use super::secret_encoder::{MAX_SECRET_STACK_ENCODED, SecretEncodeError, SecretEncoderState};

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
