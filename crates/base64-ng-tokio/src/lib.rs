#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional Tokio async helpers for `base64-ng`.
//!
//! These helpers are bounded convenience functions, not streaming state
//! machines. They read the async input to completion before writing output,
//! which avoids partial decoded writes on malformed input and keeps
//! cancellation semantics simple for the `1.0.9` companion release.

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
