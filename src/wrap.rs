//! Strict and legacy line wrapping helpers.

use crate::{
    Alphabet, DecodeError, EncodeError, LineWrap, decode_chunk, decode_tail_unpadded,
    decoded_capacity, validate_chunk, validate_tail_unpadded,
};

pub(crate) fn write_wrapped_bytes(
    input: &[u8],
    output: &mut [u8],
    output_offset: &mut usize,
    column: &mut usize,
    wrap: LineWrap,
) -> Result<(), EncodeError> {
    for byte in input {
        write_wrapped_byte(*byte, output, output_offset, column, wrap)?;
    }
    Ok(())
}

pub(crate) fn write_wrapped_byte(
    byte: u8,
    output: &mut [u8],
    output_offset: &mut usize,
    column: &mut usize,
    wrap: LineWrap,
) -> Result<(), EncodeError> {
    if *column == wrap.line_len {
        let line_ending = wrap.line_ending.as_bytes();
        let mut index = 0;
        while index < line_ending.len() {
            if *output_offset >= output.len() {
                return Err(EncodeError::OutputTooSmall {
                    required: *output_offset + 1,
                    available: output.len(),
                });
            }
            output[*output_offset] = line_ending[index];
            *output_offset += 1;
            index += 1;
        }
        *column = 0;
    }

    if *output_offset >= output.len() {
        return Err(EncodeError::OutputTooSmall {
            required: *output_offset + 1,
            available: output.len(),
        });
    }
    output[*output_offset] = byte;
    *output_offset += 1;
    *column += 1;
    Ok(())
}

struct LegacyBytes<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> LegacyBytes<'a> {
    const fn new(input: &'a [u8]) -> Self {
        Self { input, index: 0 }
    }

    fn next_byte(&mut self) -> Option<(usize, u8)> {
        while self.index < self.input.len() {
            let index = self.index;
            let byte = self.input[index];
            self.index += 1;
            if !is_legacy_whitespace(byte) {
                return Some((index, byte));
            }
        }
        None
    }
}

pub(crate) fn validate_legacy_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<usize, DecodeError> {
    let mut bytes = LegacyBytes::new(input);
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut required = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte() {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written =
                validate_chunk::<A, PAD>(chunk).map_err(|err| map_chunk_error(err, &indexes))?;
            required += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(required);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    validate_tail_unpadded::<A>(&chunk[..chunk_len])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))?;
    Ok(required + decoded_capacity(chunk_len))
}

pub(crate) fn decode_legacy_to_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let mut bytes = LegacyBytes::new(input);
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut write = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte() {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let available = output.len();
            let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| map_chunk_error(err, &indexes))?;
            write += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(write);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    decode_tail_unpadded::<A>(&chunk[..chunk_len], &mut output[write..])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))
        .map(|n| write + n)
}

struct WrappedBytes<'a> {
    input: &'a [u8],
    wrap: LineWrap,
    index: usize,
    line_len: usize,
}

impl<'a> WrappedBytes<'a> {
    const fn new(input: &'a [u8], wrap: LineWrap) -> Result<Self, DecodeError> {
        if wrap.line_len == 0 {
            return Err(DecodeError::InvalidLineWrap { index: 0 });
        }
        Ok(Self {
            input,
            wrap,
            index: 0,
            line_len: 0,
        })
    }

    fn next_byte(&mut self) -> Result<Option<(usize, u8)>, DecodeError> {
        loop {
            if self.index == self.input.len() {
                return Ok(None);
            }

            if self.starts_with_line_ending() {
                let line_end_index = self.index;
                if self.line_len == 0 {
                    return Err(DecodeError::InvalidLineWrap {
                        index: line_end_index,
                    });
                }

                self.index += self.wrap.line_ending.byte_len();
                if self.index == self.input.len() {
                    self.line_len = 0;
                    return Ok(None);
                }

                if self.line_len != self.wrap.line_len {
                    return Err(DecodeError::InvalidLineWrap {
                        index: line_end_index,
                    });
                }
                self.line_len = 0;
                continue;
            }

            let byte = self.input[self.index];
            if matches!(byte, b'\r' | b'\n') {
                return Err(DecodeError::InvalidLineWrap { index: self.index });
            }

            self.line_len += 1;
            if self.line_len > self.wrap.line_len {
                return Err(DecodeError::InvalidLineWrap { index: self.index });
            }

            let index = self.index;
            self.index += 1;
            return Ok(Some((index, byte)));
        }
    }

    fn starts_with_line_ending(&self) -> bool {
        let line_ending = self.wrap.line_ending.as_bytes();
        let Some(end) = self.index.checked_add(line_ending.len()) else {
            return false;
        };
        end <= self.input.len() && &self.input[self.index..end] == line_ending
    }
}

pub(crate) fn validate_wrapped_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
    wrap: LineWrap,
) -> Result<usize, DecodeError> {
    let mut bytes = WrappedBytes::new(input, wrap)?;
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut required = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte()? {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written =
                validate_chunk::<A, PAD>(chunk).map_err(|err| map_chunk_error(err, &indexes))?;
            required += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(required);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    validate_tail_unpadded::<A>(&chunk[..chunk_len])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))?;
    Ok(required + decoded_capacity(chunk_len))
}

pub(crate) fn decode_wrapped_to_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    wrap: LineWrap,
) -> Result<usize, DecodeError> {
    let mut bytes = WrappedBytes::new(input, wrap)?;
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut write = 0;
    let mut terminal_seen = false;

    while let Some((index, byte)) = bytes.next_byte()? {
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let available = output.len();
            let output_tail = output.get_mut(write..).ok_or(DecodeError::OutputTooSmall {
                required: write,
                available,
            })?;
            let written = decode_chunk::<A, PAD>(chunk, output_tail)
                .map_err(|err| map_chunk_error(err, &indexes))?;
            write += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(write);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    decode_tail_unpadded::<A>(&chunk[..chunk_len], &mut output[write..])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))
        .map(|n| write + n)
}

pub(crate) fn compact_wrapped_input(
    buffer: &mut [u8],
    wrap: LineWrap,
) -> Result<usize, DecodeError> {
    if !wrap.is_valid() {
        return Err(DecodeError::InvalidLineWrap { index: 0 });
    }

    let line_ending = wrap.line_ending.as_bytes();
    let line_ending_len = line_ending.len();
    let mut read = 0;
    let mut write = 0;

    while read < buffer.len() {
        let line_end = read + line_ending_len;
        if buffer.get(read..line_end) == Some(line_ending) {
            read = line_end;
            continue;
        }

        buffer[write] = buffer[read];
        write += 1;
        read += 1;
    }

    Ok(write)
}

#[inline]
pub(crate) const fn is_legacy_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn map_chunk_error(err: DecodeError, indexes: &[usize; 4]) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidInput
        | DecodeError::InvalidLineWrap { .. }
        | DecodeError::InvalidLength
        | DecodeError::OutputTooSmall { .. }
        | DecodeError::StagingTooSmall { .. } => err,
    }
}

fn map_partial_chunk_error(err: DecodeError, indexes: &[usize; 4], len: usize) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } if index < len => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } if index < len => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidByte { .. }
        | DecodeError::InvalidPadding { .. }
        | DecodeError::InvalidLineWrap { .. }
        | DecodeError::InvalidInput
        | DecodeError::InvalidLength
        | DecodeError::OutputTooSmall { .. }
        | DecodeError::StagingTooSmall { .. } => err,
    }
}
