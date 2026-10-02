//! Batch internal encoding without changing the public four-byte sink calls.

use super::{CodecSettings, EncodePadding, EncodedChunk};

pub(in crate::v2) struct BufferedChunks<'a> {
    settings: CodecSettings,
    input: &'a [u8],
    bytes: [u8; 1024],
    read: usize,
    written: usize,
}

impl<'a> BufferedChunks<'a> {
    pub(in crate::v2) const fn new(settings: CodecSettings, input: &'a [u8]) -> Self {
        Self {
            settings,
            input,
            bytes: [0; 1024],
            read: 0,
            written: 0,
        }
    }

    pub(in crate::v2) fn next_batch(&mut self) -> Option<&[u8]> {
        if self.input.is_empty() {
            return None;
        }
        let len = self.input.len().min(768);
        let tail = match (len % 3, self.settings.encode_padding()) {
            (0, _) => 0,
            (_, EncodePadding::Padded) => 4,
            (remainder, EncodePadding::Unpadded) => remainder + 1,
        };
        self.written = len / 3 * 4 + tail;
        super::super::ordinary::encode_validated(
            self.settings,
            &self.input[..len],
            &mut self.bytes[..self.written],
        );
        self.input = &self.input[len..];
        self.read = self.written;
        Some(&self.bytes[..self.written])
    }
}

impl Iterator for BufferedChunks<'_> {
    type Item = EncodedChunk;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.read == self.written {
            self.next_batch()?;
            self.read = 0;
        }
        if self.written - self.read >= 4 {
            let bytes = &self.bytes[self.read..self.read + 4];
            let chunk = EncodedChunk {
                bytes: [bytes[0], bytes[1], bytes[2], bytes[3]],
                len: 4,
            };
            self.read += 4;
            return Some(chunk);
        }
        let len = self.written - self.read;
        let mut bytes = [0; 4];
        bytes[..len].copy_from_slice(&self.bytes[self.read..self.read + len]);
        self.read += len;
        Some(EncodedChunk { bytes, len })
    }
}
