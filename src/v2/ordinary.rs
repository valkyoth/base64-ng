//! Ordinary one-shot and in-place codec ownership boundary.

#[cfg(test)]
use crate::{DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

/// Temporary compatibility adapter used only by the Commit 4 differential
/// fixture layer. Later commits replace this forwarding boundary.
#[cfg(test)]
pub(super) fn encode(
    profile: super::rfc4648_oracle::Profile,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    match profile {
        super::rfc4648_oracle::Profile::StandardPadded => STANDARD.encode_slice(input, output),
        super::rfc4648_oracle::Profile::StandardUnpadded => {
            STANDARD_NO_PAD.encode_slice(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafePadded => URL_SAFE.encode_slice(input, output),
        super::rfc4648_oracle::Profile::UrlSafeUnpadded => {
            URL_SAFE_NO_PAD.encode_slice(input, output)
        }
    }
}

/// Temporary compatibility adapter used only by the Commit 4 differential
/// fixture layer. Later commits replace this forwarding boundary.
#[cfg(test)]
pub(super) fn decode(
    profile: super::rfc4648_oracle::Profile,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    match profile {
        super::rfc4648_oracle::Profile::StandardPadded => STANDARD.decode_slice(input, output),
        super::rfc4648_oracle::Profile::StandardUnpadded => {
            STANDARD_NO_PAD.decode_slice(input, output)
        }
        super::rfc4648_oracle::Profile::UrlSafePadded => URL_SAFE.decode_slice(input, output),
        super::rfc4648_oracle::Profile::UrlSafeUnpadded => {
            URL_SAFE_NO_PAD.decode_slice(input, output)
        }
    }
}
