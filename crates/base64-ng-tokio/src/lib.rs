#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional Tokio async helpers for `base64-ng`.
//!
//! These helpers are read-all/write-all convenience functions, not streaming
//! state machines. The `*_limited` variants enforce a caller-provided maximum
//! input size before allocation grows past that bound. Output is written only
//! after the complete input has been read and encoded or decoded, which avoids
//! partial decoded writes on malformed input and keeps cancellation semantics
//! simple for the companion crate.

use base64_ng::{Alphabet, Engine};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Reads all bytes from `reader`, encodes them, and writes the encoded output.
///
/// # Errors
///
/// Returns I/O errors from the reader or writer, and wraps Base64 encoding
/// errors as [`io::ErrorKind::InvalidInput`].
pub async fn encode_reader_to_writer<A, const PAD: bool, R, W>(
    engine: &Engine<A, PAD>,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    A: Alphabet,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut input = Vec::new();
    reader.read_to_end(&mut input).await?;
    let output = engine.encode_vec(&input).map_err(encode_io_error)?;
    writer.write_all(&output).await?;
    Ok(output.len() as u64)
}

/// Reads at most `max_input_len` bytes from `reader`, encodes them, and writes
/// the encoded output.
///
/// If the input exceeds `max_input_len`, this returns
/// [`io::ErrorKind::InvalidData`] and does not write to `writer`.
///
/// # Errors
///
/// Returns I/O errors from the reader or writer, reports oversized input as
/// [`io::ErrorKind::InvalidData`], and wraps Base64 encoding errors as
/// [`io::ErrorKind::InvalidInput`].
pub async fn encode_reader_to_writer_limited<A, const PAD: bool, R, W>(
    engine: &Engine<A, PAD>,
    reader: &mut R,
    writer: &mut W,
    max_input_len: usize,
) -> io::Result<u64>
where
    A: Alphabet,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_limited(reader, max_input_len).await?;
    let output = engine.encode_vec(&input).map_err(encode_io_error)?;
    writer.write_all(&output).await?;
    Ok(output.len() as u64)
}

/// Reads all Base64 bytes from `reader`, decodes them, and writes decoded bytes.
///
/// Decoding happens before any output is written. If input is malformed, the
/// writer is untouched by this helper.
///
/// # Errors
///
/// Returns I/O errors from the reader or writer, and wraps Base64 decoding
/// errors as [`io::ErrorKind::InvalidData`].
pub async fn decode_reader_to_writer<A, const PAD: bool, R, W>(
    engine: &Engine<A, PAD>,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    A: Alphabet,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let mut input = Vec::new();
    reader.read_to_end(&mut input).await?;
    let output = engine.decode_vec(&input).map_err(decode_io_error)?;
    writer.write_all(&output).await?;
    Ok(output.len() as u64)
}

/// Reads at most `max_input_len` Base64 bytes from `reader`, decodes them, and
/// writes decoded bytes.
///
/// If the input exceeds `max_input_len` or is malformed, this returns before
/// writing to `writer`.
///
/// # Errors
///
/// Returns I/O errors from the reader or writer, reports oversized or malformed
/// input as [`io::ErrorKind::InvalidData`], and writes no decoded output on
/// either condition.
pub async fn decode_reader_to_writer_limited<A, const PAD: bool, R, W>(
    engine: &Engine<A, PAD>,
    reader: &mut R,
    writer: &mut W,
    max_input_len: usize,
) -> io::Result<u64>
where
    A: Alphabet,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_limited(reader, max_input_len).await?;
    let output = engine.decode_vec(&input).map_err(decode_io_error)?;
    writer.write_all(&output).await?;
    Ok(output.len() as u64)
}

/// Encodes `input` into an owned byte vector.
///
/// # Errors
///
/// Returns an I/O error if Base64 encoding fails.
pub fn encode_to_vec<A, const PAD: bool>(
    engine: &Engine<A, PAD>,
    input: impl AsRef<[u8]>,
) -> io::Result<Vec<u8>>
where
    A: Alphabet,
{
    engine.encode_vec(input.as_ref()).map_err(encode_io_error)
}

/// Decodes `input` into an owned byte vector.
///
/// # Errors
///
/// Returns an I/O error if Base64 decoding fails.
pub fn decode_to_vec<A, const PAD: bool>(
    engine: &Engine<A, PAD>,
    input: impl AsRef<[u8]>,
) -> io::Result<Vec<u8>>
where
    A: Alphabet,
{
    engine.decode_vec(input.as_ref()).map_err(decode_io_error)
}

fn encode_io_error(error: base64_ng::EncodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn decode_io_error(error: base64_ng::DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.kind().as_str())
}

async fn read_to_end_limited<R>(reader: &mut R, max_input_len: usize) -> io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let mut input = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok(input);
        }

        if read > max_input_len.saturating_sub(input.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio input exceeds configured limit",
            ));
        }

        input.extend_from_slice(&chunk[..read]);
    }
}
