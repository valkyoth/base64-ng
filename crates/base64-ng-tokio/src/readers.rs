use base64_ng::{Base64, Codec, DecoderState, EncoderState};
use core::{
    cmp,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{self, AsyncRead, ReadBuf};

use crate::{operation_io_error, wipe_bytes};

const ENCODE_INPUT_CAP: usize = 768;
const ENCODE_OUTPUT_CAP: usize = 1024;
const DECODE_INPUT_CAP: usize = 1024;
const DECODE_OUTPUT_CAP: usize = 768;

macro_rules! reader_observers {
    () => {
        /// Returns whether this adapter has entered its absorbing failure state.
        #[must_use]
        pub const fn is_failed(&self) -> bool {
            self.failed
        }

        /// Returns whether finalization completed successfully.
        #[must_use]
        pub const fn is_complete(&self) -> bool {
            self.finished && self.output_pos == self.output_len
        }

        /// Returns bytes irrevocably read from the wrapped source.
        #[must_use]
        pub const fn input_read(&self) -> usize {
            self.input_read
        }

        /// Returns bytes accepted by the shared Base64 transformer.
        #[must_use]
        pub const fn source_position(&self) -> usize {
            self.source_accepted
        }

        /// Returns output bytes already delivered to callers.
        #[must_use]
        pub const fn output_delivered(&self) -> usize {
            self.output_delivered
        }

        /// Returns remaining source bytes for an exact frame, or `None` for EOF mode.
        #[must_use]
        pub const fn remaining_input(&self) -> Option<usize> {
            self.boundary.remaining()
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Boundary {
    Eof,
    Exact { remaining: usize },
}

impl Boundary {
    fn read_cap(self, capacity: usize) -> usize {
        match self {
            Self::Eof => capacity,
            Self::Exact { remaining } => remaining.min(capacity),
        }
    }

    fn consume(&mut self, count: usize) {
        if let Self::Exact { remaining } = self {
            *remaining -= count;
        }
    }

    const fn remaining(self) -> Option<usize> {
        match self {
            Self::Eof => None,
            Self::Exact { remaining } => Some(remaining),
        }
    }
}

/// Async reader that transforms raw bytes into Base64 through the shared 2.0
/// incremental encoder.
///
/// [`Self::new`] finalizes only after the wrapped reader returns EOF.
/// [`Self::new_exact`] instead finalizes after exactly the declared number of
/// source bytes and leaves adjacent bytes unread. Both modes preserve state
/// across `Poll::Pending` and use fixed internal storage.
///
/// Output already returned through [`AsyncRead`] is irrevocably visible.
/// Internal cleanup is best-effort and cannot clear caller buffers, wrapped
/// reader storage, registers, caches, swap, or crash dumps.
pub struct EncoderReader<R> {
    inner: R,
    state: EncoderState,
    boundary: Boundary,
    input: [u8; ENCODE_INPUT_CAP],
    output: [u8; ENCODE_OUTPUT_CAP],
    output_pos: usize,
    output_len: usize,
    input_read: usize,
    source_accepted: usize,
    output_delivered: usize,
    finished: bool,
    failed: bool,
}

impl<R> EncoderReader<R> {
    /// Creates a reader that continues until the wrapped reader returns EOF.
    #[must_use]
    pub fn new<S: Codec>(inner: R, codec: &Base64<S>) -> Self {
        Self::with_boundary(inner, codec.encoder(), Boundary::Eof)
    }

    /// Creates a reader for one exact-length source frame.
    ///
    /// The adapter never reads beyond `input_len`. Premature EOF is reported as
    /// [`io::ErrorKind::UnexpectedEof`].
    #[must_use]
    pub fn new_exact<S: Codec>(inner: R, codec: &Base64<S>, input_len: usize) -> Self {
        Self::with_boundary(
            inner,
            codec.encoder(),
            Boundary::Exact {
                remaining: input_len,
            },
        )
    }

    fn with_boundary(inner: R, state: EncoderState, boundary: Boundary) -> Self {
        Self {
            inner,
            state,
            boundary,
            input: [0; ENCODE_INPUT_CAP],
            output: [0; ENCODE_OUTPUT_CAP],
            output_pos: 0,
            output_len: 0,
            input_read: 0,
            source_accepted: 0,
            output_delivered: 0,
            finished: false,
            failed: false,
        }
    }

    reader_observers!();

    fn clear_internal(&mut self) {
        self.state.clear();
        wipe_bytes(&mut self.input);
        wipe_bytes(&mut self.output);
        self.output_pos = 0;
        self.output_len = 0;
    }

    fn fail(&mut self, error: io::Error) -> Poll<io::Result<()>> {
        self.failed = true;
        self.clear_internal();
        Poll::Ready(Err(error))
    }

    fn drain(&mut self, destination: &mut ReadBuf<'_>) -> io::Result<bool> {
        let count = cmp::min(
            self.output_len.saturating_sub(self.output_pos),
            destination.remaining(),
        );
        if count == 0 {
            return Ok(false);
        }
        let next_delivered = self
            .output_delivered
            .checked_add(count)
            .ok_or_else(|| io::Error::other("base64-ng-tokio output position overflow"))?;
        destination.put_slice(&self.output[self.output_pos..self.output_pos + count]);
        wipe_bytes(&mut self.output[self.output_pos..self.output_pos + count]);
        self.output_pos += count;
        self.output_delivered = next_delivered;
        if self.output_pos == self.output_len {
            self.output_pos = 0;
            self.output_len = 0;
        }
        Ok(true)
    }

    fn transform_input(&mut self, read: usize) -> io::Result<()> {
        let result = self.state.update(&self.input[..read], &mut self.output);
        wipe_bytes(&mut self.input[..read]);
        let step = result.map_err(operation_io_error)?;
        if step.progress().input_consumed() != read {
            return Err(io::Error::other(
                "base64-ng-tokio encoder made partial progress",
            ));
        }
        self.output_pos = 0;
        self.output_len = step.progress().output_produced();
        self.source_accepted = self.state.source_position();
        Ok(())
    }

    fn finish_transform(&mut self) -> io::Result<()> {
        let step = self
            .state
            .finish(&mut self.output)
            .map_err(operation_io_error)?;
        self.output_pos = 0;
        self.output_len = step.progress().output_produced();
        self.finished = true;
        Ok(())
    }
}

/// Async reader that strictly decodes Base64 through the shared 2.0
/// incremental decoder.
///
/// This is an ordinary prefix-delivering API: plaintext returned before a
/// later malformed suffix is irrevocably exposed. Secret-bearing frames must
/// use a bounded validate-before-release secret API instead of this adapter.
pub struct DecoderReader<R> {
    inner: R,
    state: DecoderState,
    boundary: Boundary,
    input: [u8; DECODE_INPUT_CAP],
    output: [u8; DECODE_OUTPUT_CAP],
    output_pos: usize,
    output_len: usize,
    input_read: usize,
    source_accepted: usize,
    output_delivered: usize,
    finished: bool,
    failed: bool,
}

impl<R> DecoderReader<R> {
    /// Creates a strict decoder that continues until the wrapped reader returns
    /// EOF.
    #[must_use]
    pub fn new<S: Codec>(inner: R, codec: &Base64<S>) -> Self {
        Self::with_boundary(inner, codec.decoder(), Boundary::Eof)
    }

    /// Creates a strict decoder for one exact-length encoded frame.
    ///
    /// The adapter never reads beyond `input_len`. Premature EOF is reported as
    /// [`io::ErrorKind::UnexpectedEof`]. This remains an ordinary streaming API
    /// and does not defer plaintext release until full-frame validation.
    #[must_use]
    pub fn new_exact<S: Codec>(inner: R, codec: &Base64<S>, input_len: usize) -> Self {
        Self::with_boundary(
            inner,
            codec.decoder(),
            Boundary::Exact {
                remaining: input_len,
            },
        )
    }

    fn with_boundary(inner: R, state: DecoderState, boundary: Boundary) -> Self {
        Self {
            inner,
            state,
            boundary,
            input: [0; DECODE_INPUT_CAP],
            output: [0; DECODE_OUTPUT_CAP],
            output_pos: 0,
            output_len: 0,
            input_read: 0,
            source_accepted: 0,
            output_delivered: 0,
            finished: false,
            failed: false,
        }
    }

    reader_observers!();

    fn clear_internal(&mut self) {
        self.state.clear();
        wipe_bytes(&mut self.input);
        wipe_bytes(&mut self.output);
        self.output_pos = 0;
        self.output_len = 0;
    }

    fn fail(&mut self, error: io::Error) -> Poll<io::Result<()>> {
        self.failed = true;
        self.clear_internal();
        Poll::Ready(Err(error))
    }

    fn drain(&mut self, destination: &mut ReadBuf<'_>) -> io::Result<bool> {
        let count = cmp::min(
            self.output_len.saturating_sub(self.output_pos),
            destination.remaining(),
        );
        if count == 0 {
            return Ok(false);
        }
        let next_delivered = self
            .output_delivered
            .checked_add(count)
            .ok_or_else(|| io::Error::other("base64-ng-tokio output position overflow"))?;
        destination.put_slice(&self.output[self.output_pos..self.output_pos + count]);
        wipe_bytes(&mut self.output[self.output_pos..self.output_pos + count]);
        self.output_pos += count;
        self.output_delivered = next_delivered;
        if self.output_pos == self.output_len {
            self.output_pos = 0;
            self.output_len = 0;
        }
        Ok(true)
    }

    fn transform_input(&mut self, read: usize) -> io::Result<()> {
        let result = self.state.update(&self.input[..read], &mut self.output);
        wipe_bytes(&mut self.input[..read]);
        let step = result.map_err(operation_io_error)?;
        if step.progress().input_consumed() != read {
            return Err(io::Error::other(
                "base64-ng-tokio decoder made partial progress",
            ));
        }
        self.output_pos = 0;
        self.output_len = step.progress().output_produced();
        self.source_accepted = self.state.source_position();
        Ok(())
    }

    fn finish_transform(&mut self) -> io::Result<()> {
        let step = self
            .state
            .finish(&mut self.output)
            .map_err(operation_io_error)?;
        self.output_pos = 0;
        self.output_len = step.progress().output_produced();
        self.finished = true;
        Ok(())
    }
}

macro_rules! impl_async_read {
    ($reader:ident) => {
        impl<R> AsyncRead for $reader<R>
        where
            R: AsyncRead + Unpin,
        {
            fn poll_read(
                mut self: Pin<&mut Self>,
                context: &mut Context<'_>,
                destination: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                let this = &mut *self;
                if this.failed {
                    return Poll::Ready(Err(io::Error::other("base64-ng-tokio reader is failed")));
                }
                match this.drain(destination) {
                    Ok(true) => return Poll::Ready(Ok(())),
                    Err(error) => return this.fail(error),
                    Ok(false) => {}
                }
                if destination.remaining() == 0 || this.finished {
                    return Poll::Ready(Ok(()));
                }

                loop {
                    let read_cap = this.boundary.read_cap(this.input.len());
                    if read_cap == 0 {
                        if let Err(error) = this.finish_transform() {
                            return this.fail(error);
                        }
                    } else {
                        let (polled, read) = {
                            let mut input_buf = ReadBuf::new(&mut this.input[..read_cap]);
                            let polled =
                                Pin::new(&mut this.inner).poll_read(context, &mut input_buf);
                            (polled, input_buf.filled().len())
                        };
                        match polled {
                            Poll::Pending if read == 0 => return Poll::Pending,
                            Poll::Pending => {
                                return this.fail(io::Error::other(
                                    "base64-ng-tokio inner reader filled bytes before Pending",
                                ));
                            }
                            Poll::Ready(Err(error)) => return this.fail(error),
                            Poll::Ready(Ok(())) if read == 0 => {
                                if this.boundary.remaining().is_some() {
                                    return this.fail(io::Error::new(
                                        io::ErrorKind::UnexpectedEof,
                                        "base64-ng-tokio exact frame ended early",
                                    ));
                                }
                                if let Err(error) = this.finish_transform() {
                                    return this.fail(error);
                                }
                            }
                            Poll::Ready(Ok(())) => {
                                let Some(input_read) = this.input_read.checked_add(read) else {
                                    return this.fail(io::Error::other(
                                        "base64-ng-tokio input position overflow",
                                    ));
                                };
                                this.input_read = input_read;
                                this.boundary.consume(read);
                                if let Err(error) = this.transform_input(read) {
                                    return this.fail(error);
                                }
                            }
                        }
                    }

                    match this.drain(destination) {
                        Ok(true) => return Poll::Ready(Ok(())),
                        Err(error) => return this.fail(error),
                        Ok(false) if this.finished => return Poll::Ready(Ok(())),
                        Ok(false) => {}
                    }
                }
            }
        }

        impl<R> Drop for $reader<R> {
            fn drop(&mut self) {
                self.clear_internal();
            }
        }
    };
}

impl_async_read!(EncoderReader);
impl_async_read!(DecoderReader);
