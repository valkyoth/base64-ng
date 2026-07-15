#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional Tokio async helpers for `base64-ng`.
//!
//! The crate provides two API tiers:
//!
//! - read-all/write-all convenience functions, with `*_limited` variants for
//!   peer-controlled request or frame boundaries.
//! - manual [`AsyncRead`] and [`AsyncWrite`] streaming adapters with fixed
//!   internal buffers and explicit drop cleanup.
//!
//! The streaming adapters are implemented as explicit state machines. They do
//! not use `async fn` internally, so cancellation can only leave data in each
//! adapter's fixed pending/output buffers; those buffers are cleared on drop.
//!
//! # Security
//!
//! The read-all helpers use RAII-guarded temporary `Vec<u8>` allocations and
//! the normal strict decode path. The guards wipe initialized bytes and spare
//! capacity on ordinary return, I/O error, or future cancellation. They are
//! not constant-time-oriented token validators or high-assurance secret
//! decoders. For secret-bearing async frames, collect a bounded frame under
//! the application's approved memory policy and decode through
//! `base64_ng::ct`, staged CT decode, `base64-ng-derive`, or
//! `base64-ng-sanitization`.

mod decoder_writer;
mod encoder_writer;
mod queue;
mod readers;

pub use decoder_writer::DecoderWriter;
pub use encoder_writer::EncoderWriter;
pub use readers::{DecoderReader, EncoderReader};

use base64_ng::{Alphabet, Engine};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const READ_ALL_EAGER_CAP: usize = 8192;

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
    let input = read_to_end_guarded(reader).await?;
    let output = WipingVec::from_vec(
        engine
            .encode_vec(input.as_slice())
            .map_err(encode_io_error)?,
    );
    let written = output.len() as u64;
    writer.write_all(output.as_slice()).await?;
    Ok(written)
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
    let output = WipingVec::from_vec(
        engine
            .encode_vec(input.as_slice())
            .map_err(encode_io_error)?,
    );
    let written = output.len() as u64;
    writer.write_all(output.as_slice()).await?;
    Ok(written)
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
    let input = read_to_end_guarded(reader).await?;
    let output = WipingVec::from_vec(
        engine
            .decode_vec(input.as_slice())
            .map_err(decode_io_error)?,
    );
    let written = output.len() as u64;
    writer.write_all(output.as_slice()).await?;
    Ok(written)
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
    let output = WipingVec::from_vec(
        engine
            .decode_vec(input.as_slice())
            .map_err(decode_io_error)?,
    );
    let written = output.len() as u64;
    writer.write_all(output.as_slice()).await?;
    Ok(written)
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

fn wipe_bytes(bytes: &mut [u8]) {
    base64_ng::secure_wipe(bytes);
}

struct WipingVec(Vec<u8>);

impl WipingVec {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    fn from_vec(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn extend_from_slice_wiping_old(
        &mut self,
        bytes: &[u8],
        capacity_limit: usize,
    ) -> io::Result<()> {
        let required = self.len().checked_add(bytes.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio input is too large",
            )
        })?;
        if required > capacity_limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio input exceeds configured limit",
            ));
        }

        if required <= self.0.capacity() {
            self.0.extend_from_slice(bytes);
            return Ok(());
        }

        let grown_capacity = self.0.capacity().saturating_mul(2).max(required);
        let replacement_capacity = grown_capacity.min(capacity_limit);
        let mut replacement = Self::with_capacity(replacement_capacity);
        replacement.0.extend_from_slice(&self.0);
        replacement.0.extend_from_slice(bytes);
        core::mem::swap(self, &mut replacement);
        drop(replacement);
        Ok(())
    }
}

impl Drop for WipingVec {
    fn drop(&mut self) {
        // Initialize the existing spare capacity without reallocating so the
        // reviewed wipe primitive covers the complete allocation.
        self.0.resize(self.0.capacity(), 0);
        wipe_bytes(&mut self.0);
        self.0.clear();
    }
}

struct WipingArray<const N: usize>([u8; N]);

impl<const N: usize> WipingArray<N> {
    const fn new() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> Drop for WipingArray<N> {
    fn drop(&mut self) {
        wipe_bytes(&mut self.0);
    }
}

async fn read_to_end_guarded<R>(reader: &mut R) -> io::Result<WipingVec>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let mut input = WipingVec::new();
    let mut chunk = WipingArray::<8192>::new();

    loop {
        let read = reader.read(&mut chunk.0).await?;
        if read == 0 {
            return Ok(input);
        }

        input.extend_from_slice_wiping_old(&chunk.0[..read], usize::MAX)?;
        wipe_bytes(&mut chunk.0[..read]);
    }
}

async fn read_to_end_limited<R>(reader: &mut R, max_input_len: usize) -> io::Result<WipingVec>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let mut input = WipingVec::with_capacity(max_input_len.min(READ_ALL_EAGER_CAP));
    let mut chunk = WipingArray::<8192>::new();

    loop {
        let remaining = max_input_len - input.len();
        let read_cap = if remaining < chunk.0.len() {
            remaining + 1
        } else {
            chunk.0.len()
        };
        let read = reader.read(&mut chunk.0[..read_cap]).await?;
        if read == 0 {
            return Ok(input);
        }

        if read > remaining {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio input exceeds configured limit",
            ));
        }

        input.extend_from_slice_wiping_old(&chunk.0[..read], max_input_len)?;
        wipe_bytes(&mut chunk.0[..read]);
    }
}
