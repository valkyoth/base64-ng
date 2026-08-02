#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Optional Tokio async helpers for `base64-ng`.
//!
//! The crate provides two API tiers:
//!
//! - read-all convenience functions, with `*_limited` variants for
//!   peer-controlled request or frame boundaries.
//! - manual [`AsyncRead`] and [`AsyncWrite`] streaming adapters with fixed
//!   internal buffers and explicit drop cleanup.
//!
//! Reader adapters are implemented as explicit poll state machines over the
//! shared 2.0 incremental core. [`EncoderReader::new_exact`] and
//! [`DecoderReader::new_exact`] stop at an exact frame boundary without an
//! overflow lookahead read. Writer adapters use the same shared states and
//! preserve bounded queued output across backpressure and cancellation.
//!
//! # Security
//!
//! The read-all helpers use RAII-guarded temporary `Vec<u8>` allocations and
//! the normal strict decode path. The guards wipe initialized bytes and spare
//! capacity on ordinary return, I/O error, or future cancellation. They are
//! not constant-time-oriented token validators or high-assurance secret
//! decoders. For secret-bearing async frames, collect a bounded frame under
//! the application's approved memory policy and decode through
//! the 2.0 `secrets` capability or an approved protected-memory companion.
//! Streaming decoder output accepted by an inner writer is irrevocably
//! exposed, even when a later encoded suffix is malformed.

mod decoder_writer;
mod encoder_writer;
mod queue;
mod readers;

pub use decoder_writer::DecoderWriter;
pub use encoder_writer::EncoderWriter;
pub use readers::{DecoderReader, EncoderReader};

use base64_ng::{Base64, Codec, Failure, OneShotError, OperationError};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const READ_ALL_EAGER_CAP: usize = 8192;

/// Reads all bytes from `reader`, encodes them, and writes the encoded output.
///
/// # Errors
///
/// Returns I/O errors from the reader or writer, and wraps Base64 encoding
/// errors as [`io::ErrorKind::InvalidInput`].
pub async fn encode_reader_to_writer<S, R, W>(
    codec: &Base64<S>,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    S: Codec,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_guarded(reader).await?;
    let output = WipingVec::from_vec(
        codec
            .encode_to_string(input.as_slice())
            .map_err(one_shot_encode_io_error)?
            .into_bytes(),
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
pub async fn encode_reader_to_writer_limited<S, R, W>(
    codec: &Base64<S>,
    reader: &mut R,
    writer: &mut W,
    max_input_len: usize,
) -> io::Result<u64>
where
    S: Codec,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_limited(reader, max_input_len).await?;
    let output = WipingVec::from_vec(
        codec
            .encode_to_string(input.as_slice())
            .map_err(one_shot_encode_io_error)?
            .into_bytes(),
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
pub async fn decode_reader_to_writer<S, R, W>(
    codec: &Base64<S>,
    reader: &mut R,
    writer: &mut W,
) -> io::Result<u64>
where
    S: Codec,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_guarded(reader).await?;
    let output = WipingVec::from_vec(
        codec
            .decode_to_vec(input.as_slice())
            .map_err(one_shot_decode_io_error)?,
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
pub async fn decode_reader_to_writer_limited<S, R, W>(
    codec: &Base64<S>,
    reader: &mut R,
    writer: &mut W,
    max_input_len: usize,
) -> io::Result<u64>
where
    S: Codec,
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let input = read_to_end_limited(reader, max_input_len).await?;
    let output = WipingVec::from_vec(
        codec
            .decode_to_vec(input.as_slice())
            .map_err(one_shot_decode_io_error)?,
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
pub fn encode_to_vec<S>(codec: &Base64<S>, input: impl AsRef<[u8]>) -> io::Result<Vec<u8>>
where
    S: Codec,
{
    codec
        .encode_to_string(input.as_ref())
        .map(String::into_bytes)
        .map_err(one_shot_encode_io_error)
}

/// Decodes `input` into an owned byte vector.
///
/// # Errors
///
/// Returns an I/O error if Base64 decoding fails.
pub fn decode_to_vec<S>(codec: &Base64<S>, input: impl AsRef<[u8]>) -> io::Result<Vec<u8>>
where
    S: Codec,
{
    codec
        .decode_to_vec(input.as_ref())
        .map_err(one_shot_decode_io_error)
}

fn one_shot_encode_io_error(error: OneShotError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn one_shot_decode_io_error(error: OneShotError) -> io::Error {
    if let OneShotError::Input(input) = error {
        io::Error::new(io::ErrorKind::InvalidData, input.kind().as_str())
    } else {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }
}

fn operation_io_error(error: OperationError) -> io::Error {
    match error {
        OperationError::Failed(Failure::Input(input)) => {
            io::Error::new(io::ErrorKind::InvalidData, input.kind().as_str())
        }
        _ => io::Error::other(error.as_str()),
    }
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
