//! Streaming Base64 wrappers for `std::io`.
//!
//! Decoder adapters fail closed after malformed Base64 input. Encoder writer
//! adapters also expose failed-state diagnostics for unrecoverable internal
//! queue/encoding errors. Use
//! `is_failed()` for diagnostics; unchecked `into_inner()` remains
//! available when the wrapped reader or writer must be explicitly
//! recovered after a decode error.
//!
//! # Security
//!
//! Streaming decoders use the normal strict decode path. They preserve
//! localized I/O-style errors and are not constant-time decoders. For
//! secret-bearing frames where timing posture matters, collect the complete
//! framed payload first and then use `base64_ng::ct`:
//!
//! Streaming decoder writers commit decoded bytes as quads are accepted. If a
//! later quad in the same logical frame is malformed, valid leading decoded
//! bytes may already have reached the wrapped writer before `finish()` reports
//! failure. Callers that require atomic frame semantics must buffer the full
//! encoded frame first and use a non-streaming decoder. Callers that use
//! streaming decode for untrusted frames must not trust the wrapped writer's
//! output until `finish()` succeeds, and can inspect [`Decoder::is_failed`] for
//! diagnostics after each write.
//!
//! ```no_run
//! use std::io::Read;
//! use base64_ng::ct;
//!
//! const MAX_FRAME: usize = 4096;
//!
//! # fn decode_secret_frame<R: Read>(mut reader: R) -> Result<(), Box<dyn std::error::Error>> {
//! let mut frame = Vec::new();
//! reader.read_to_end(&mut frame)?;
//! let decoded = ct::STANDARD.decode_buffer::<MAX_FRAME>(&frame)?;
//! # let _ = decoded;
//! # Ok(())
//! # }
//! ```
//!
//! ```
//! use std::io::{Read, Write};
//! use base64_ng::{STANDARD, stream::{Decoder, DecoderReader, Encoder, EncoderReader}};
//!
//! let mut encoder = Encoder::new(Vec::new(), STANDARD);
//! encoder.write_all(b"he").unwrap();
//! encoder.write_all(b"llo").unwrap();
//! let encoded = encoder.finish().unwrap();
//! assert_eq!(encoded, b"aGVsbG8=");
//!
//! let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
//! let mut encoded = String::new();
//! reader.read_to_string(&mut encoded).unwrap();
//! assert_eq!(encoded, "aGVsbG8=");
//!
//! let mut decoder = Decoder::new(Vec::new(), STANDARD);
//! decoder.write_all(b"aGVs").unwrap();
//! decoder.write_all(b"bG8=").unwrap();
//! let decoded = decoder.finish().unwrap();
//! assert_eq!(decoded, b"hello");
//!
//! let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
//! let mut decoded = Vec::new();
//! reader.read_to_end(&mut decoded).unwrap();
//! assert_eq!(decoded, b"hello");
//! ```

use super::{Alphabet, DecodeError, EncodeError, Engine};
use std::io::{self, Read, Write};

struct OutputQueue<const CAP: usize> {
    buffer: [u8; CAP],
    start: usize,
    len: usize,
}

impl<const CAP: usize> OutputQueue<CAP> {
    const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            start: 0,
            len: 0,
        }
    }

    const fn is_empty(&self) -> bool {
        self.len == 0
    }

    const fn len(&self) -> usize {
        self.len
    }

    const fn capacity(&self) -> usize {
        self.len + self.available_capacity()
    }

    fn push_slice(&mut self, input: &[u8]) -> io::Result<()> {
        if input.len() > self.available_capacity() {
            return Err(io::Error::other(
                "base64 stream output queue capacity exceeded",
            ));
        }

        let mut read = 0;
        while read < input.len() {
            let write = (self.start + self.len) % CAP;
            self.buffer[write] = input[read];
            self.len += 1;
            read += 1;
        }

        Ok(())
    }

    fn copy_front(&self, output: &mut [u8]) -> usize {
        let count = core::cmp::min(self.len, output.len());
        let first = core::cmp::min(count, CAP - self.start);
        output[..first].copy_from_slice(&self.buffer[self.start..self.start + first]);

        let second = count - first;
        if second > 0 {
            output[first..first + second].copy_from_slice(&self.buffer[..second]);
        }

        count
    }

    fn discard_front(&mut self, count: usize) {
        let count = core::cmp::min(count, self.len);
        let first = core::cmp::min(count, CAP - self.start);
        crate::wipe_bytes(&mut self.buffer[self.start..self.start + first]);

        let second = count - first;
        if second > 0 {
            crate::wipe_bytes(&mut self.buffer[..second]);
        }

        self.start = (self.start + count) % CAP;
        self.len -= count;
        if self.len == 0 {
            self.start = 0;
        }
    }

    fn pop_slice(&mut self, output: &mut [u8]) -> usize {
        let count = self.copy_front(output);
        self.discard_front(count);
        count
    }

    fn clear_all(&mut self) {
        crate::wipe_bytes(&mut self.buffer);
        self.start = 0;
        self.len = 0;
    }

    const fn available_capacity(&self) -> usize {
        CAP - self.len
    }
}

/// A streaming Base64 encoder for `std::io::Write`.
///
/// Like any [`Write`] implementation, [`Write::write`] may accept only
/// part of the provided input. Accepted input may be held as encoded
/// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
/// or a later write drains the wrapped writer. Use [`Write::write_all`]
/// when the whole input slice must be consumed.
pub struct Encoder<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: OutputQueue<1024>,
    finalized: bool,
    failed: bool,
}

impl<W, A, const PAD: bool> Encoder<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming encoder.
    #[must_use]
    pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: OutputQueue::new(),
            finalized: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped writer.
    #[must_use]
    pub fn get_ref(&self) -> &W {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }

    /// Returns the Base64 engine used by this adapter.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this adapter uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns the number of raw input bytes currently buffered until a
    /// complete 3-byte Base64 encode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this encoder currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional input bytes are needed to complete the
    /// currently buffered encode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            3 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of encoded bytes buffered for the wrapped
    /// writer after a previous write or flush could not fully drain them.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of encoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more encoded bytes can be buffered before this
    /// adapter must drain the wrapped writer.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this encoder has encoded output waiting to be
    /// written to the wrapped writer.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this encoder has been finalized.
    ///
    /// Once this returns `true`, later writes return an error.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether this encoder has failed closed after an unrecoverable
    /// internal encoding or buffering error.
    ///
    /// Ordinary wrapped-writer I/O errors are retryable and do not set this
    /// flag. Once this returns `true`, later writes, flushes, and finalization
    /// attempts return an error. The unchecked [`Self::into_inner`] method can
    /// still be used for explicit recovery of the wrapped writer.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// writer without discarding pending input.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && !self.has_pending_input() && !self.has_buffered_output()
    }

    /// Consumes the encoder without flushing pending input.
    ///
    /// Prefer [`Self::finish`] when the encoded output must be complete.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Consumes the encoder only when no partial input quantum is buffered.
    ///
    /// This does not flush or finalize the wrapped writer. It is a checked
    /// alternative to [`Self::into_inner`] for callers that want to avoid
    /// accidentally discarding pending input bytes.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &W {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream encoder inner writer was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }

    fn clear_output(&mut self) {
        self.output.clear_all();
    }
}

impl<W, A, const PAD: bool> Drop for Encoder<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> core::fmt::Debug for Encoder<W, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Encoder")
            .field("inner", &redacted_inner_state(self.inner.is_some()))
            .field("engine", &self.engine)
            .field("pending", &"<redacted>")
            .field("pending_len", &self.pending_len)
            .field("pending_input_needed_len", &self.pending_input_needed_len())
            .field("buffered_output_len", &self.output.len())
            .field("buffered_output_capacity", &self.output.capacity())
            .field(
                "buffered_output_remaining_capacity",
                &self.output.available_capacity(),
            )
            .field("can_into_inner", &self.can_into_inner())
            .field("finalized", &self.finalized)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<W, A, const PAD: bool> Encoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    /// Writes any pending input and flushes the wrapped writer without
    /// consuming this encoder.
    ///
    /// After this succeeds, [`Self::pending_len`] returns `0`, later
    /// writes are rejected, and [`Self::finish`] can still be used to
    /// recover the wrapped writer.
    /// This is useful when a caller needs to finalize a framed payload
    /// while keeping the stream adapter available for diagnostics or
    /// explicit recovery.
    pub fn try_finish(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }
        if !self.finalized {
            self.queue_pending_final()?;
            self.finalized = true;
        }
        self.flush()
    }

    /// Writes any pending input, flushes the wrapped writer, and returns it.
    pub fn finish(mut self) -> io::Result<W> {
        self.try_finish()?;
        Ok(self.take_inner())
    }

    fn queue_pending_final(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 2];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        let mut encoded = [0u8; 4];
        let result = self.queue_encoded_temp(&pending[..pending_len], &mut encoded);
        crate::wipe_bytes(&mut pending);
        result?;
        self.clear_pending();
        Ok(())
    }

    fn queue_encoded_temp(&mut self, input: &[u8], encoded: &mut [u8]) -> io::Result<()> {
        let written = match self.engine.encode_slice(input, encoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(encoded);
                self.failed = true;
                return Err(encode_error_to_io(err));
            }
        };

        let result = self.output.push_slice(&encoded[..written]);
        crate::wipe_bytes(encoded);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn drain_output(&mut self) -> io::Result<()> {
        let mut chunk = [0u8; 1024];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = self.inner_mut().write(&chunk[..pending]);
            crate::wipe_bytes(&mut chunk[..pending]);
            match result {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64 stream encoder could not drain buffered output",
                    ));
                }
                Ok(written) => {
                    if written > pending {
                        self.failed = true;
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "wrapped writer reported more bytes than provided",
                        ));
                    }
                    self.output.discard_front(written);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

impl<W, A, const PAD: bool> Write for Encoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }
        self.drain_output()?;
        if self.finalized {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64 stream encoder received input after finalization",
            ));
        }
        if input.is_empty() {
            return Ok(0);
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 3 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(input.len());
            }

            let mut chunk = [0u8; 3];
            chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            chunk[self.pending_len..].copy_from_slice(&input[..needed]);

            let mut encoded = [0u8; 4];
            let result = self.queue_encoded_temp(&chunk, &mut encoded);
            crate::wipe_bytes(&mut chunk);
            result?;
            self.clear_pending();
            consumed += needed;
        }

        let remaining = &input[consumed..];
        let full_len = remaining.len() / 3 * 3;
        if full_len > 0 {
            let max_by_queue = self.output.available_capacity() / 4 * 3;
            let mut take = core::cmp::min(full_len, core::cmp::min(768, max_by_queue));
            take -= take % 3;

            if take == 0 {
                return Ok(consumed);
            }

            let mut encoded = [0u8; 1024];
            self.queue_encoded_temp(&remaining[..take], &mut encoded)?;
            consumed += take;

            if take < full_len {
                return Ok(consumed);
            }
        }

        let tail = &input[consumed..];
        self.pending[..tail.len()].copy_from_slice(tail);
        self.pending_len = tail.len();
        consumed += tail.len();

        Ok(consumed)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }
        self.drain_output()?;
        self.inner_mut().flush()
    }
}

fn encode_error_to_io(err: EncodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, err)
}

/// A streaming Base64 decoder for `std::io::Write`.
///
/// Like any [`Write`] implementation, [`Write::write`] may accept only
/// part of the provided input. Accepted input may be held as decoded
/// output until [`Write::flush`], [`Self::try_finish`], [`Self::finish`],
/// or a later write drains the wrapped writer. Use [`Write::write_all`]
/// when the whole input slice must be consumed.
///
/// # Security
///
/// This adapter uses the normal strict decoder, not the [`crate::ct`]
/// module. It may branch or return early based on malformed input and it
/// preserves strict error diagnostics. Do not use it for secret-bearing
/// payloads when malformed-input timing matters; decode a complete frame
/// with the matching `ct` engine instead.
pub struct Decoder<W, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<W>,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: OutputQueue<1024>,
    finished: bool,
    failed: bool,
    finalized: bool,
}

impl<W, A, const PAD: bool> Decoder<W, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming decoder.
    ///
    /// # Security
    ///
    /// Streaming decoders use the normal strict decode path. They are not
    /// constant-time-oriented secret decoders.
    #[must_use]
    pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
            finalized: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped writer.
    #[must_use]
    pub fn get_ref(&self) -> &W {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.inner_mut()
    }

    /// Returns the Base64 engine used by this adapter.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this adapter uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns the number of encoded input bytes currently buffered until
    /// a complete 4-byte Base64 decode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this decoder currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional input bytes are needed to complete the
    /// currently buffered decode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            4 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of decoded bytes buffered for the wrapped writer
    /// after a previous write or flush could not fully drain them.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of decoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more decoded bytes can be buffered before this
    /// adapter must drain the wrapped writer.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this decoder has decoded output waiting to be
    /// written to the wrapped writer.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this decoder has processed a terminal padded block.
    ///
    /// Once this returns `true`, later calls to [`Write::write`] with
    /// additional input return an error because strict Base64 does not
    /// permit trailing payload bytes after padding.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.finished
    }

    /// Returns whether this decoder has been finalized.
    ///
    /// Once this returns `true`, later non-empty writes return an error.
    #[must_use]
    pub const fn is_finalized(&self) -> bool {
        self.finalized
    }

    /// Returns whether this decoder has rejected malformed Base64 input.
    ///
    /// Once this returns `true`, later writes, flushes, and finalization
    /// attempts return an error. The unchecked [`Self::into_inner`] method
    /// can still be used for explicit recovery of the wrapped writer.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// writer without discarding pending encoded input.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && !self.has_pending_input() && !self.has_buffered_output()
    }

    /// Consumes the decoder without flushing pending input.
    ///
    /// Prefer [`Self::finish`] when the decoded output must be complete.
    #[must_use]
    pub fn into_inner(mut self) -> W {
        self.take_inner()
    }

    /// Consumes the decoder only when no partial input quantum is buffered.
    ///
    /// This does not flush or finalize the wrapped writer. It is a checked
    /// alternative to [`Self::into_inner`] for callers that want to avoid
    /// accidentally discarding pending encoded input bytes.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &W {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut W {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn take_inner(&mut self) -> W {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream decoder inner writer was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }

    fn clear_output(&mut self) {
        self.output.clear_all();
    }
}

impl<W, A, const PAD: bool> Drop for Decoder<W, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.clear_output();
    }
}

impl<W, A, const PAD: bool> core::fmt::Debug for Decoder<W, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Decoder")
            .field("inner", &redacted_inner_state(self.inner.is_some()))
            .field("engine", &self.engine)
            .field("pending", &"<redacted>")
            .field("pending_len", &self.pending_len)
            .field("pending_input_needed_len", &self.pending_input_needed_len())
            .field("buffered_output_len", &self.output.len())
            .field("buffered_output_capacity", &self.output.capacity())
            .field(
                "buffered_output_remaining_capacity",
                &self.output.available_capacity(),
            )
            .field("can_into_inner", &self.can_into_inner())
            .field("terminal_padding", &self.finished)
            .field("finalized", &self.finalized)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<W, A, const PAD: bool> Decoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    /// Validates any final pending input and flushes the wrapped writer
    /// without consuming this decoder.
    ///
    /// After this succeeds, [`Self::pending_len`] returns `0`, later
    /// writes are rejected, and [`Self::finish`] can still be used to
    /// recover the wrapped writer.
    /// If the final buffered input is malformed, an error is returned and
    /// the caller still owns the decoder for diagnostics or explicit
    /// recovery.
    pub fn try_finish(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if !self.finalized {
            self.queue_pending_final()?;
            self.finalized = true;
        }
        self.flush()
    }

    /// Validates final pending input, flushes the wrapped writer, and returns it.
    pub fn finish(mut self) -> io::Result<W> {
        self.try_finish()?;
        Ok(self.take_inner())
    }

    fn queue_pending_final(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 4];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&pending[..pending_len], &mut decoded);
        crate::wipe_bytes(&mut pending);
        if let Err(err) = result {
            self.clear_pending();
            return Err(err);
        }
        self.clear_pending();
        Ok(())
    }

    fn queue_full_quad(&mut self, mut input: [u8; 4]) -> io::Result<()> {
        let mut decoded = [0u8; 3];
        let result = self.queue_decoded_temp(&input, &mut decoded);
        crate::wipe_bytes(&mut input);
        let written = result?;
        if written < 3 {
            self.finished = true;
        }
        Ok(())
    }

    fn queue_decoded_temp(&mut self, input: &[u8], decoded: &mut [u8]) -> io::Result<usize> {
        let written = match self.engine.decode_slice(input, decoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(decoded);
                self.failed = true;
                return Err(decode_error_to_io(err));
            }
        };

        let result = self.output.push_slice(&decoded[..written]);
        crate::wipe_bytes(decoded);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        Ok(written)
    }

    fn drain_output(&mut self) -> io::Result<()> {
        let mut chunk = [0u8; 1024];
        while !self.output.is_empty() {
            let pending = self.output.copy_front(&mut chunk);
            let result = self.inner_mut().write(&chunk[..pending]);
            crate::wipe_bytes(&mut chunk[..pending]);
            match result {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "base64 stream decoder could not drain buffered output",
                    ));
                }
                Ok(written) => {
                    if written > pending {
                        self.failed = true;
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "wrapped writer reported more bytes than provided",
                        ));
                    }
                    self.output.discard_front(written);
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}

impl<W, A, const PAD: bool> Write for Decoder<W, A, PAD>
where
    W: Write,
    A: Alphabet,
{
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if input.is_empty() {
            self.drain_output()?;
            return Ok(0);
        }
        self.drain_output()?;
        if self.finalized {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "base64 stream decoder received input after finalization",
            ));
        }
        if self.finished {
            self.failed = true;
            return Err(trailing_input_after_padding_error());
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 4 - self.pending_len;
            if input.len() < needed {
                self.pending[self.pending_len..self.pending_len + input.len()]
                    .copy_from_slice(input);
                self.pending_len += input.len();
                return Ok(input.len());
            }

            let mut quad = [0u8; 4];
            quad[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            quad[self.pending_len..].copy_from_slice(&input[..needed]);
            let result = self.queue_full_quad(quad);
            crate::wipe_bytes(&mut quad);
            if let Err(err) = result {
                self.clear_pending();
                return Err(err);
            }
            self.clear_pending();
            consumed += needed;

            if self.finished {
                return Ok(consumed);
            }
        }

        while input.len() - consumed >= 4 {
            if self.output.available_capacity() < 3 {
                return Ok(consumed);
            }

            let mut quad = [
                input[consumed],
                input[consumed + 1],
                input[consumed + 2],
                input[consumed + 3],
            ];
            let mut decoded = [0u8; 3];
            let written = match self.engine.decode_slice(&quad, &mut decoded) {
                Ok(written) => written,
                Err(err) => {
                    crate::wipe_bytes(&mut quad);
                    crate::wipe_bytes(&mut decoded);
                    self.failed = true;
                    if consumed > 0 {
                        return Ok(consumed);
                    }

                    return Err(decode_error_to_io(err));
                }
            };

            let result = self.output.push_slice(&decoded[..written]);
            crate::wipe_bytes(&mut quad);
            crate::wipe_bytes(&mut decoded);
            result?;
            consumed += 4;

            if written < 3 {
                self.finished = true;
                return Ok(consumed);
            }
        }

        let tail = &input[consumed..];
        self.pending[..tail.len()].copy_from_slice(tail);
        self.pending_len = tail.len();
        consumed += tail.len();

        Ok(consumed)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        self.drain_output()?;
        self.inner_mut().flush()
    }
}

fn decode_error_to_io(err: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, err)
}

fn trailing_input_after_padding_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 decoder received trailing input after padding",
    )
}

fn stream_decoder_failed_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 stream decoder is failed after malformed input",
    )
}

fn stream_encoder_failed_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 stream encoder is failed after internal error",
    )
}

/// A streaming Base64 decoder for `std::io::Read`.
///
/// For padded engines, this reader stops at the terminal padded Base64
/// block and leaves later bytes unread in the wrapped reader. This preserves
/// boundaries for callers that decode one Base64 payload from a larger
/// stream.
///
/// # Security
///
/// This adapter uses the normal strict decoder, not the [`crate::ct`]
/// module. It may branch or return early based on malformed input and it
/// preserves strict error diagnostics. Do not use it for secret-bearing
/// payloads when malformed-input timing matters; decode a complete frame
/// with the matching `ct` engine instead.
pub struct DecoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<R>,
    engine: Engine<A, PAD>,
    pending: [u8; 4],
    pending_len: usize,
    output: OutputQueue<3>,
    finished: bool,
    terminal_seen: bool,
    failed: bool,
}

impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming decoder reader.
    ///
    /// # Security
    ///
    /// Streaming decoder readers use the normal strict decode path. They
    /// are not constant-time-oriented secret decoders.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 4],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
            terminal_seen: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped reader.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner_mut()
    }

    /// Returns the Base64 engine used by this adapter.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this adapter uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns the number of encoded input bytes currently buffered until
    /// a complete 4-byte Base64 decode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this decoder reader currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional encoded input bytes are needed to
    /// complete the currently buffered decode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            4 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of decoded bytes currently buffered and ready to
    /// be read before this adapter polls the wrapped reader again.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of decoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more decoded bytes can be buffered before this
    /// adapter must return bytes to the caller.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this decoder reader currently has decoded output
    /// waiting in its internal queue.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this decoder reader has seen terminal padding.
    ///
    /// For padded engines, this becomes `true` after the terminal padded
    /// block is decoded. The wrapped reader is then left positioned after
    /// that Base64 block so adjacent framed bytes can be read by the
    /// caller.
    #[must_use]
    pub const fn has_terminal_padding(&self) -> bool {
        self.terminal_seen
    }

    /// Returns whether this decoder reader has reached EOF or terminal
    /// padding in the wrapped reader.
    ///
    /// This may become `true` before [`Self::is_finished`] when decoded
    /// output is still buffered for the caller.
    #[must_use]
    pub const fn has_finished_input(&self) -> bool {
        self.finished
    }

    /// Returns whether this reader has reached EOF or terminal padding
    /// and has no decoded output buffered for the caller.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished && self.output.is_empty()
    }

    /// Returns whether this decoder reader has rejected malformed Base64
    /// input.
    ///
    /// Once this returns `true`, later reads return an error. The unchecked
    /// [`Self::into_inner`] method can still be used for explicit recovery
    /// of the wrapped reader.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// reader without discarding buffered decoded output.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        !self.is_failed() && self.is_finished()
    }

    /// Consumes the decoder reader and returns the wrapped reader.
    #[must_use]
    pub fn into_inner(mut self) -> R {
        self.take_inner()
    }

    /// Consumes the decoder reader only after the Base64 payload is fully
    /// drained.
    ///
    /// For padded streams, terminal padding may leave adjacent framed bytes
    /// unread in the wrapped reader. This method succeeds only after all
    /// decoded output buffered by this adapter has been read, so recovering
    /// the wrapped reader does not silently discard decoded bytes.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<R, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &R {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut R {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn take_inner(&mut self) -> R {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream decoder reader inner reader was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }
}

impl<R, A, const PAD: bool> Drop for DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.output.clear_all();
    }
}

impl<R, A, const PAD: bool> core::fmt::Debug for DecoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("DecoderReader")
            .field("inner", &redacted_inner_state(self.inner.is_some()))
            .field("engine", &self.engine)
            .field("pending", &"<redacted>")
            .field("pending_len", &self.pending_len)
            .field("pending_input_needed_len", &self.pending_input_needed_len())
            .field("buffered_output_len", &self.output.len())
            .field("buffered_output_capacity", &self.output.capacity())
            .field(
                "buffered_output_remaining_capacity",
                &self.output.available_capacity(),
            )
            .field("can_into_inner", &self.can_into_inner())
            .field("finished", &self.finished)
            .field("terminal_padding", &self.terminal_seen)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<R, A, const PAD: bool> Read for DecoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.failed {
            return Err(stream_decoder_failed_error());
        }

        while self.output.is_empty() && !self.finished {
            self.fill_output()?;
        }

        Ok(self.output.pop_slice(output))
    }
}

impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn fill_output(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(stream_decoder_failed_error());
        }
        if self.terminal_seen {
            self.finished = true;
            return Ok(());
        }

        let mut input = [0u8; 4];
        let available = 4 - self.pending_len;
        let read = match self.inner_mut().read(&mut input[..available]) {
            Ok(read) => read,
            Err(err) => {
                crate::wipe_bytes(&mut input);
                return Err(err);
            }
        };
        if read == 0 {
            crate::wipe_bytes(&mut input);
            self.finished = true;
            self.push_final_pending()?;
            return Ok(());
        }

        self.pending[self.pending_len..self.pending_len + read].copy_from_slice(&input[..read]);
        crate::wipe_bytes(&mut input);
        self.pending_len += read;
        if self.pending_len < 4 {
            return Ok(());
        }

        let mut quad = self.pending;
        self.clear_pending();
        let result = self.push_decoded(&quad);
        crate::wipe_bytes(&mut quad);
        result?;
        if self.terminal_seen {
            self.finished = true;
        }
        Ok(())
    }

    fn push_final_pending(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 4];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        self.clear_pending();
        let result = self.push_decoded(&pending[..pending_len]);
        crate::wipe_bytes(&mut pending);
        result
    }

    fn push_decoded(&mut self, input: &[u8]) -> io::Result<()> {
        let mut decoded = [0u8; 3];
        let written = match self.engine.decode_slice(input, &mut decoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(&mut decoded);
                self.failed = true;
                return Err(decode_error_to_io(err));
            }
        };
        let result = self.output.push_slice(&decoded[..written]);
        crate::wipe_bytes(&mut decoded);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        if input.len() == 4 && written < 3 {
            self.terminal_seen = true;
        }
        Ok(())
    }
}

/// A streaming Base64 encoder for `std::io::Read`.
pub struct EncoderReader<R, A, const PAD: bool>
where
    A: Alphabet,
{
    inner: Option<R>,
    engine: Engine<A, PAD>,
    pending: [u8; 2],
    pending_len: usize,
    output: OutputQueue<1024>,
    finished: bool,
    failed: bool,
}

impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    /// Creates a new streaming encoder reader.
    #[must_use]
    pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
        Self {
            inner: Some(inner),
            engine,
            pending: [0; 2],
            pending_len: 0,
            output: OutputQueue::new(),
            finished: false,
            failed: false,
        }
    }

    /// Returns a shared reference to the wrapped reader.
    #[must_use]
    pub fn get_ref(&self) -> &R {
        self.inner_ref()
    }

    /// Returns a mutable reference to the wrapped reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner_mut()
    }

    /// Returns the Base64 engine used by this adapter.
    #[must_use]
    pub const fn engine(&self) -> Engine<A, PAD> {
        self.engine
    }

    /// Returns whether this adapter uses padded Base64.
    #[must_use]
    pub const fn is_padded(&self) -> bool {
        PAD
    }

    /// Returns the number of raw input bytes currently buffered until a
    /// complete 3-byte Base64 encode quantum is available.
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending_len
    }

    /// Returns whether this encoder reader currently holds a partial input
    /// quantum.
    #[must_use]
    pub const fn has_pending_input(&self) -> bool {
        self.pending_len != 0
    }

    /// Returns how many additional raw input bytes are needed to complete
    /// the currently buffered encode quantum.
    ///
    /// Returns `0` when no partial input quantum is buffered.
    #[must_use]
    pub const fn pending_input_needed_len(&self) -> usize {
        if self.has_pending_input() {
            3 - self.pending_len
        } else {
            0
        }
    }

    /// Returns the number of encoded bytes currently buffered and ready to
    /// be read before this adapter polls the wrapped reader again.
    #[must_use]
    pub const fn buffered_output_len(&self) -> usize {
        self.output.len()
    }

    /// Returns the maximum number of encoded bytes this adapter can buffer
    /// before returning bytes to the caller.
    #[must_use]
    pub const fn buffered_output_capacity(&self) -> usize {
        self.output.capacity()
    }

    /// Returns how many more encoded bytes can be buffered before this
    /// adapter must return bytes to the caller.
    #[must_use]
    pub const fn buffered_output_remaining_capacity(&self) -> usize {
        self.output.available_capacity()
    }

    /// Returns whether this encoder reader currently has encoded output
    /// waiting in its internal queue.
    #[must_use]
    pub const fn has_buffered_output(&self) -> bool {
        !self.output.is_empty()
    }

    /// Returns whether this encoder reader has reached EOF in the wrapped
    /// reader.
    ///
    /// This may become `true` before [`Self::is_finished`] when encoded
    /// output is still buffered for the caller.
    #[must_use]
    pub const fn has_finished_input(&self) -> bool {
        self.finished
    }

    /// Returns whether this reader has reached EOF and has no encoded
    /// output buffered for the caller.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished && self.output.is_empty()
    }

    /// Returns whether this adapter has failed closed after an internal
    /// stream error.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.failed
    }

    /// Returns whether [`Self::try_into_inner`] can recover the wrapped
    /// reader without discarding pending input or buffered encoded output.
    #[must_use]
    pub const fn can_into_inner(&self) -> bool {
        self.is_finished() && !self.failed
    }

    /// Consumes the encoder reader and returns the wrapped reader.
    #[must_use]
    pub fn into_inner(mut self) -> R {
        self.take_inner()
    }

    /// Consumes the encoder reader only after the encoded stream is fully
    /// drained.
    ///
    /// This is a checked alternative to [`Self::into_inner`] for callers
    /// that want to avoid accidentally discarding pending input or encoded
    /// output buffered inside the adapter.
    #[allow(clippy::result_large_err)]
    pub fn try_into_inner(mut self) -> Result<R, Self> {
        if !self.can_into_inner() {
            return Err(self);
        }
        Ok(self.take_inner())
    }

    fn inner_ref(&self) -> &R {
        match &self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn inner_mut(&mut self) -> &mut R {
        match &mut self.inner {
            Some(inner) => inner,
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn take_inner(&mut self) -> R {
        match self.inner.take() {
            Some(inner) => inner,
            None => unreachable!("stream encoder reader inner reader was already taken"),
        }
    }

    fn clear_pending(&mut self) {
        crate::wipe_bytes(&mut self.pending);
        self.pending_len = 0;
    }
}

impl<R, A, const PAD: bool> Drop for EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn drop(&mut self) {
        self.clear_pending();
        self.output.clear_all();
    }
}

impl<R, A, const PAD: bool> core::fmt::Debug for EncoderReader<R, A, PAD>
where
    A: Alphabet,
{
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncoderReader")
            .field("inner", &redacted_inner_state(self.inner.is_some()))
            .field("engine", &self.engine)
            .field("pending", &"<redacted>")
            .field("pending_len", &self.pending_len)
            .field("pending_input_needed_len", &self.pending_input_needed_len())
            .field("buffered_output_len", &self.output.len())
            .field("buffered_output_capacity", &self.output.capacity())
            .field(
                "buffered_output_remaining_capacity",
                &self.output.available_capacity(),
            )
            .field("can_into_inner", &self.can_into_inner())
            .field("finished", &self.finished)
            .field("failed", &self.failed)
            .finish()
    }
}

impl<R, A, const PAD: bool> Read for EncoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(stream_encoder_failed_error());
        }

        if output.is_empty() {
            return Ok(0);
        }

        while self.output.is_empty() && !self.finished {
            self.fill_output()?;
        }

        Ok(self.output.pop_slice(output))
    }
}

impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
where
    R: Read,
    A: Alphabet,
{
    fn fill_output(&mut self) -> io::Result<()> {
        let mut input = [0u8; 768];
        let read = match self.inner_mut().read(&mut input) {
            Ok(read) => read,
            Err(err) => {
                crate::wipe_bytes(&mut input);
                return Err(err);
            }
        };
        if read == 0 {
            crate::wipe_bytes(&mut input);
            self.finished = true;
            if let Err(err) = self.push_final_pending() {
                self.failed = true;
                return Err(err);
            }
            return Ok(());
        }

        let mut consumed = 0;
        if self.pending_len > 0 {
            let needed = 3 - self.pending_len;
            if read < needed {
                self.pending[self.pending_len..self.pending_len + read]
                    .copy_from_slice(&input[..read]);
                self.pending_len += read;
                crate::wipe_bytes(&mut input);
                return Ok(());
            }

            let mut chunk = [0u8; 3];
            chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            chunk[self.pending_len..].copy_from_slice(&input[..needed]);
            let result = self.push_encoded(&chunk);
            crate::wipe_bytes(&mut chunk);
            if let Err(err) = result {
                crate::wipe_bytes(&mut input);
                self.failed = true;
                return Err(err);
            }
            self.clear_pending();
            consumed += needed;
        }

        let remaining = &input[consumed..read];
        let full_len = remaining.len() / 3 * 3;
        let tail_len = remaining.len() - full_len;
        let mut tail = [0u8; 2];
        tail[..tail_len].copy_from_slice(&remaining[full_len..]);
        let result = if full_len > 0 {
            self.push_encoded(&remaining[..full_len])
        } else {
            Ok(())
        };
        crate::wipe_bytes(&mut input);
        if let Err(err) = result {
            crate::wipe_bytes(&mut tail);
            self.failed = true;
            return Err(err);
        }
        self.pending[..tail_len].copy_from_slice(&tail[..tail_len]);
        crate::wipe_bytes(&mut tail);
        self.pending_len = tail_len;
        Ok(())
    }

    fn push_final_pending(&mut self) -> io::Result<()> {
        if self.pending_len == 0 {
            return Ok(());
        }

        let mut pending = [0u8; 2];
        pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
        let pending_len = self.pending_len;
        self.clear_pending();
        let result = self.push_encoded(&pending[..pending_len]);
        crate::wipe_bytes(&mut pending);
        result
    }

    fn push_encoded(&mut self, input: &[u8]) -> io::Result<()> {
        let mut encoded = [0u8; 1024];
        let written = match self.engine.encode_slice(input, &mut encoded) {
            Ok(written) => written,
            Err(err) => {
                crate::wipe_bytes(&mut encoded);
                return Err(encode_error_to_io(err));
            }
        };
        let result = self.output.push_slice(&encoded[..written]);
        crate::wipe_bytes(&mut encoded);
        result
    }
}

const fn redacted_inner_state(present: bool) -> &'static str {
    if present { "<present>" } else { "<taken>" }
}
