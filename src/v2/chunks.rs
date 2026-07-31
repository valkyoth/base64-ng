//! Borrowed input iteration over synthesized encoded quanta.

use super::{
    ordinary::OneShotError,
    specifications::{Base64, Codec, CodecSettings, EncodePadding},
};

/// One synthesized Base64 output chunk.
///
/// Complete chunks contain four bytes. The final chunk may contain two or
/// three bytes for an unpadded codec. This value owns its synthesized bytes;
/// it is not a zero-copy view into the plaintext input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodedChunk {
    bytes: [u8; 4],
    len: usize,
}

impl EncodedChunk {
    /// Returns the initialized encoded bytes in this chunk.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the initialized encoded bytes as visible ASCII.
    ///
    /// Validated alphabets guarantee this succeeds. The `Result` keeps that
    /// invariant checked without introducing an unsafe UTF-8 conversion.
    pub fn as_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.as_bytes())
    }
}

impl core::fmt::Display for EncodedChunk {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str().map_err(|_| core::fmt::Error)?)
    }
}

/// Iterator over synthesized Base64 output chunks for one borrowed input.
///
/// The iterator borrows only `input`; it owns a copy of the validated codec
/// settings. Dropping the codec value after construction therefore does not
/// invalidate iteration, while the input cannot be dropped or mutated until
/// this iterator is released.
#[derive(Clone)]
pub struct EncodedChunks<'a> {
    settings: CodecSettings,
    input: &'a [u8],
    offset: usize,
}

impl core::fmt::Debug for EncodedChunks<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncodedChunks")
            .field("input_len", &self.input.len())
            .field("offset", &self.offset)
            .field("remaining_chunks", &self.remaining_chunks())
            .finish_non_exhaustive()
    }
}

impl<'a> EncodedChunks<'a> {
    pub(super) const fn new(settings: CodecSettings, input: &'a [u8]) -> Self {
        Self {
            settings,
            input,
            offset: 0,
        }
    }

    fn remaining_chunks(&self) -> usize {
        self.input.len().saturating_sub(self.offset).div_ceil(3)
    }
}

impl Iterator for EncodedChunks<'_> {
    type Item = EncodedChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.input.get(self.offset..)?;
        if remaining.is_empty() {
            return None;
        }

        let consumed = remaining.len().min(3);
        let chunk = encode_chunk(self.settings, &remaining[..consumed]);
        self.offset += consumed;
        Some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining_chunks();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for EncodedChunks<'_> {
    fn len(&self) -> usize {
        self.remaining_chunks()
    }
}

impl core::iter::FusedIterator for EncodedChunks<'_> {}

impl<S: Codec> Base64<S> {
    /// Returns a borrowed-input iterator over synthesized encoded chunks.
    ///
    /// Length arithmetic is checked before the iterator is returned, so
    /// iteration itself has no encoding-error path. Empty input yields no
    /// chunks. Padding, when configured, appears only in the final chunk.
    pub fn encoded_chunks<'a>(&self, input: &'a [u8]) -> Result<EncodedChunks<'a>, OneShotError> {
        self.encoded_len(input.len())?;
        Ok(EncodedChunks::new(self.settings(), input))
    }
}

fn encode_chunk(settings: CodecSettings, input: &[u8]) -> EncodedChunk {
    let alphabet = settings.alphabet().as_array();
    let first = input[0];
    let second = input.get(1).copied().unwrap_or(0);
    let third = input.get(2).copied().unwrap_or(0);
    let mut bytes = [b'='; 4];

    bytes[0] = alphabet[usize::from(first >> 2)];
    bytes[1] = alphabet[usize::from(((first & 3) << 4) | (second >> 4))];
    bytes[2] = alphabet[usize::from(((second & 15) << 2) | (third >> 6))];
    bytes[3] = alphabet[usize::from(third & 63)];

    let len = match input.len() {
        3 => 4,
        1 | 2 if settings.encode_padding() == EncodePadding::Padded => 4,
        2 => 3,
        1 => 2,
        _ => 0,
    };
    if settings.encode_padding() == EncodePadding::Padded {
        if input.len() == 1 {
            bytes[2] = b'=';
            bytes[3] = b'=';
        } else if input.len() == 2 {
            bytes[3] = b'=';
        }
    }

    EncodedChunk { bytes, len }
}
