use base64_ng::{Alphabet, Engine};
use core::{
    cmp,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{self, AsyncRead, ReadBuf};

use crate::{decode_io_error, encode_io_error, wipe_bytes};

const ENCODE_INPUT_CAP: usize = 768;
const ENCODE_OUTPUT_CAP: usize = 1024;
const DECODE_INPUT_CAP: usize = 1024;
const DECODE_OUTPUT_CAP: usize = 768;

/// Async reader that streams raw bytes as Base64.
///
/// This adapter reads from `inner` in bounded chunks, preserves at most two
/// pending raw bytes between polls, and clears its pending/output buffers on
/// drop. It is cancellation-resumable: if a read future is dropped after
/// returning [`Poll::Pending`], polling the same adapter again continues from
/// the same internal state without duplicating or dropping bytes.
///
/// # Security
///
/// Internal cleanup is best-effort and limited to this adapter's fixed buffers.
/// It cannot clear copies held by the wrapped reader, the caller's output
/// buffer, registers, caches, swap, or crash dumps.
pub struct EncoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: R,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: [u8; ENCODE_OUTPUT_CAP],
    output_pos: usize,
    output_len: usize,
    finished: bool,
    failed: bool,
    _alphabet: PhantomData<A>,
}

impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new async Base64 encoder reader.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner,
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: [0; ENCODE_OUTPUT_CAP],
            output_pos: 0,
            output_len: 0,
            finished: false,
            failed: false,
            _alphabet: PhantomData,
        }
    }

    /// Returns whether the adapter has encountered an unrecoverable error.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    fn clear_buffers(&mut self) {
        self.pending.fill(0);
        self.pending_len = 0;
        self.output.fill(0);
        self.output_pos = 0;
        self.output_len = 0;
    }

    fn drain_output(&mut self, destination: &mut ReadBuf<'_>) -> bool {
        let available = self.output_len.saturating_sub(self.output_pos);
        if available == 0 || destination.remaining() == 0 {
            return false;
        }

        let count = cmp::min(available, destination.remaining());
        destination.put_slice(&self.output[self.output_pos..self.output_pos + count]);
        wipe_bytes(&mut self.output[self.output_pos..self.output_pos + count]);
        self.output_pos += count;
        if self.output_pos == self.output_len {
            self.output_pos = 0;
            self.output_len = 0;
        }
        true
    }

    fn append_encoded(&mut self, input: &[u8]) -> io::Result<()> {
        let written = self
            .engine
            .encode_slice(input, &mut self.output[self.output_len..])
            .map_err(encode_io_error)?;
        self.output_len += written;
        Ok(())
    }

    fn process_input(&mut self, input: &[u8]) -> io::Result<()> {
        let mut read = 0;

        if self.pending_len != 0 {
            let needed = 3 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(());
            }

            let mut quantum = [0u8; 3];
            quantum[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            quantum[self.pending_len..].copy_from_slice(&input[..needed]);
            let result = self.append_encoded(&quantum);
            wipe_bytes(&mut quantum);
            wipe_bytes(&mut self.pending);
            result?;
            self.pending_len = 0;
            read += needed;
        }

        let remaining = &input[read..];
        let full_len = remaining.len() / 3 * 3;
        if full_len != 0 {
            self.append_encoded(&remaining[..full_len])?;
        }

        let tail = &remaining[full_len..];
        if !tail.is_empty() {
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();
        }

        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.pending_len != 0 {
            let mut tail = [0u8; 2];
            tail[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            let result = self.append_encoded(&tail[..pending_len]);
            wipe_bytes(&mut tail);
            wipe_bytes(&mut self.pending);
            result?;
            self.pending_len = 0;
        }
        self.finished = true;
        Ok(())
    }
}

impl<R, A, const PAD: bool> AsyncRead for EncoderReader<R, A, PAD>
where
    R: AsyncRead + Unpin,
    A: Alphabet + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        destination: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio encoder reader is failed",
            )));
        }

        if self.drain_output(destination) || destination.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        if self.finished {
            return Poll::Ready(Ok(()));
        }

        loop {
            let mut input = [0u8; ENCODE_INPUT_CAP];
            let mut input_buf = ReadBuf::new(&mut input);
            match Pin::new(&mut self.inner).poll_read(context, &mut input_buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => {
                    wipe_bytes(&mut input);
                    self.failed = true;
                    self.clear_buffers();
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(Ok(())) => {
                    let read = input_buf.filled().len();
                    if read == 0 {
                        let result = self.finish();
                        wipe_bytes(&mut input);
                        if let Err(error) = result {
                            self.failed = true;
                            self.clear_buffers();
                            return Poll::Ready(Err(error));
                        }
                    } else {
                        let result = self.process_input(&input[..read]);
                        wipe_bytes(&mut input);
                        if let Err(error) = result {
                            self.failed = true;
                            self.clear_buffers();
                            return Poll::Ready(Err(error));
                        }
                    }

                    if self.drain_output(destination) || self.finished {
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        }
    }
}

impl<R, A, const PAD: bool> Drop for EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_buffers();
    }
}

/// Async reader that streams Base64 input as decoded bytes.
///
/// This adapter decodes strict Base64 quanta as they arrive and preserves at
/// most three pending encoded bytes between polls. If malformed input is
/// observed, the adapter fails closed and clears its internal buffers.
///
/// # Security
///
/// Streaming decode is not atomic. Decoded bytes from valid leading quanta may
/// already have been returned before a later malformed quantum is observed. For
/// atomic secret-bearing frames, use `decode_reader_to_writer_limited` or a
/// `ct` staged decode after collecting a bounded frame.
pub struct DecoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: R,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: [u8; DECODE_OUTPUT_CAP],
    output_pos: usize,
    output_len: usize,
    finished: bool,
    failed: bool,
    terminal_padding: bool,
    _alphabet: PhantomData<A>,
}

impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new async Base64 decoder reader.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner,
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: [0; DECODE_OUTPUT_CAP],
            output_pos: 0,
            output_len: 0,
            finished: false,
            failed: false,
            terminal_padding: false,
            _alphabet: PhantomData,
        }
    }

    /// Returns whether the adapter has encountered an unrecoverable error.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    fn clear_buffers(&mut self) {
        self.pending.fill(0);
        self.pending_len = 0;
        self.output.fill(0);
        self.output_pos = 0;
        self.output_len = 0;
    }

    fn drain_output(&mut self, destination: &mut ReadBuf<'_>) -> bool {
        let available = self.output_len.saturating_sub(self.output_pos);
        if available == 0 || destination.remaining() == 0 {
            return false;
        }

        let count = cmp::min(available, destination.remaining());
        destination.put_slice(&self.output[self.output_pos..self.output_pos + count]);
        wipe_bytes(&mut self.output[self.output_pos..self.output_pos + count]);
        self.output_pos += count;
        if self.output_pos == self.output_len {
            self.output_pos = 0;
            self.output_len = 0;
        }
        true
    }

    fn append_decoded(&mut self, input: &[u8]) -> io::Result<usize> {
        let written = self
            .engine
            .decode_slice(input, &mut self.output[self.output_len..])
            .map_err(decode_io_error)?;
        self.output_len += written;
        Ok(written)
    }

    fn process_quad(&mut self, mut quad: [u8; 4]) -> io::Result<()> {
        if self.terminal_padding {
            wipe_bytes(&mut quad);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio decoder reader received trailing input after padding",
            ));
        }

        let result = self.append_decoded(&quad);
        let saw_terminal = quad.contains(&b'=');
        wipe_bytes(&mut quad);
        result?;

        if saw_terminal {
            self.terminal_padding = true;
        }
        Ok(())
    }

    fn process_input(&mut self, input: &[u8]) -> io::Result<()> {
        if self.terminal_padding && !input.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "base64-ng-tokio decoder reader received trailing input after padding",
            ));
        }

        let mut read = 0;

        if self.pending_len != 0 {
            let needed = 4 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(());
            }

            let mut quad = [0u8; 4];
            quad[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            quad[self.pending_len..].copy_from_slice(&input[..needed]);
            self.process_quad(quad)?;
            wipe_bytes(&mut self.pending);
            self.pending_len = 0;
            read += needed;
        }

        while read + 4 <= input.len() {
            let quad = [
                input[read],
                input[read + 1],
                input[read + 2],
                input[read + 3],
            ];
            self.process_quad(quad)?;
            let saw_terminal = self.terminal_padding;
            read += 4;

            if saw_terminal && read != input.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "base64-ng-tokio decoder reader received trailing input after padding",
                ));
            }
        }

        let tail = &input[read..];
        if !tail.is_empty() {
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();
        }

        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.pending_len != 0 {
            if PAD || self.pending_len == 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "base64-ng-tokio decoder reader received incomplete final quantum",
                ));
            }

            let mut tail = [0u8; 4];
            tail[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            self.append_decoded(&tail[..pending_len])?;
            wipe_bytes(&mut tail);
            wipe_bytes(&mut self.pending);
            self.pending_len = 0;
        }
        self.finished = true;
        Ok(())
    }
}

impl<R, A, const PAD: bool> AsyncRead for DecoderReader<R, A, PAD>
where
    R: AsyncRead + Unpin,
    A: Alphabet + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        destination: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.failed {
            return Poll::Ready(Err(io::Error::other(
                "base64-ng-tokio decoder reader is failed",
            )));
        }

        if self.drain_output(destination) || destination.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        if self.finished {
            return Poll::Ready(Ok(()));
        }

        loop {
            let mut input = [0u8; DECODE_INPUT_CAP];
            let mut input_buf = ReadBuf::new(&mut input);
            match Pin::new(&mut self.inner).poll_read(context, &mut input_buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => {
                    wipe_bytes(&mut input);
                    self.failed = true;
                    self.clear_buffers();
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(Ok(())) => {
                    let read = input_buf.filled().len();
                    if read == 0 {
                        let result = self.finish();
                        wipe_bytes(&mut input);
                        if let Err(error) = result {
                            self.failed = true;
                            self.clear_buffers();
                            return Poll::Ready(Err(error));
                        }
                    } else {
                        let result = self.process_input(&input[..read]);
                        wipe_bytes(&mut input);
                        if let Err(error) = result {
                            self.failed = true;
                            self.clear_buffers();
                            return Poll::Ready(Err(error));
                        }
                    }

                    if self.drain_output(destination) || self.finished {
                        return Poll::Ready(Ok(()));
                    }
                }
            }
        }
    }
}

impl<R, A, const PAD: bool> Drop for DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_buffers();
    }
}
