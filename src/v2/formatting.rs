//! Allocation-free formatting and exact counted-sink encoding.

use super::{
    chunks::{EncodedChunk, EncodedChunks},
    contracts::BackendFault,
    ordinary::OneShotError,
    specifications::{Base64, Codec, CodecSettings},
};

/// Error from allocation-free formatter encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FormatWriteError {
    /// Encoding preflight failed before any formatter call.
    Encoding(OneShotError),
    /// A formatter call returned `fmt::Error`.
    Formatter {
        /// Bytes passed through fully successful prior `write_str` calls.
        confirmed: usize,
    },
    /// A validated internal output invariant failed.
    Backend {
        /// Internal backend failure classification.
        fault: BackendFault,
        /// Bytes passed through fully successful prior `write_str` calls.
        confirmed: usize,
    },
}

impl FormatWriteError {
    /// Returns bytes confirmed by fully successful formatter calls.
    ///
    /// A failing `write_str` implementation may have partially mutated its
    /// sink before returning. Those unreported bytes are intentionally not
    /// included.
    #[must_use]
    pub const fn confirmed(&self) -> usize {
        match self {
            Self::Encoding(_) => 0,
            Self::Formatter { confirmed } | Self::Backend { confirmed, .. } => *confirmed,
        }
    }
}

impl core::fmt::Display for FormatWriteError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(formatter),
            Self::Formatter { confirmed } => write!(
                formatter,
                "formatter failed after {confirmed} confirmed Base64 bytes"
            ),
            Self::Backend { fault, confirmed } => write!(
                formatter,
                "Base64 backend {} failed after {confirmed} confirmed bytes",
                fault.as_str()
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FormatWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encoding(error) => Some(error),
            Self::Formatter { .. } | Self::Backend { .. } => None,
        }
    }
}

/// A sink whose successful writes report their exact accepted prefix.
///
/// `write` must return an accepted count no larger than `bytes.len()`. `Err`
/// must mean that the failing call accepted zero bytes. This stronger contract
/// lets [`Base64::encode_to_counted`] report exact committed progress; sinks
/// that can mutate before returning `Err` must use formatter or I/O contracts
/// with weaker prefix guarantees instead.
pub trait CountedSink {
    /// Sink-specific failure.
    type Error;

    /// Accepts and reports the exact committed prefix of `bytes`.
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error>;
}

/// Failure from exact-progress counted-sink encoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum CountedWriteError<E> {
    /// Encoding preflight failed before the sink was called.
    Encoding(OneShotError),
    /// The sink rejected a call without accepting bytes from that call.
    Sink {
        /// Sink-specific error.
        error: E,
        /// Exact bytes accepted by successful prior calls.
        committed: usize,
    },
    /// The sink accepted zero bytes from a non-empty call.
    WriteZero {
        /// Exact bytes accepted by successful prior calls.
        committed: usize,
    },
    /// The sink violated its count contract.
    InvalidCount {
        /// Count reported by the sink.
        reported: usize,
        /// Bytes offered to the sink.
        offered: usize,
        /// Exact bytes accepted before the invalid report.
        committed: usize,
    },
    /// A validated internal output invariant failed.
    Backend {
        /// Internal backend failure classification.
        fault: BackendFault,
        /// Exact bytes accepted by successful prior calls.
        committed: usize,
    },
}

impl<E> CountedWriteError<E> {
    /// Returns the exact committed byte count before failure.
    #[must_use]
    pub const fn committed(&self) -> usize {
        match self {
            Self::Encoding(_) => 0,
            Self::Sink { committed, .. }
            | Self::WriteZero { committed }
            | Self::InvalidCount { committed, .. }
            | Self::Backend { committed, .. } => *committed,
        }
    }
}

impl<E: core::fmt::Display> core::fmt::Display for CountedWriteError<E> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(formatter),
            Self::Sink { error, committed } => write!(
                formatter,
                "counted sink failed after {committed} committed Base64 bytes: {error}"
            ),
            Self::WriteZero { committed } => write!(
                formatter,
                "counted sink accepted zero bytes after {committed} committed Base64 bytes"
            ),
            Self::InvalidCount {
                reported,
                offered,
                committed,
            } => write!(
                formatter,
                "counted sink reported {reported} bytes for a {offered}-byte write after \
                 {committed} committed Base64 bytes"
            ),
            Self::Backend { fault, committed } => write!(
                formatter,
                "Base64 backend {} failed after {committed} committed bytes",
                fault.as_str()
            ),
        }
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + 'static> std::error::Error for CountedWriteError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encoding(error) => Some(error),
            Self::Sink { error, .. } => Some(error),
            Self::WriteZero { .. } | Self::InvalidCount { .. } | Self::Backend { .. } => None,
        }
    }
}

/// Lazy allocation-free encoded display for one borrowed input.
///
/// This value owns copied validated codec settings and borrows only the input.
/// Construct it with [`Base64::display`] so length errors are returned before
/// formatting begins.
#[derive(Clone, Copy)]
pub struct EncodedDisplay<'a> {
    settings: CodecSettings,
    input: &'a [u8],
}

impl core::fmt::Debug for EncodedDisplay<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EncodedDisplay")
            .field("input_len", &self.input.len())
            .finish_non_exhaustive()
    }
}

impl core::fmt::Display for EncodedDisplay<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for chunk in EncodedChunks::new(self.settings, self.input) {
            formatter.write_str(chunk_text(&chunk).map_err(|_| core::fmt::Error)?)?;
        }
        Ok(())
    }
}

impl<S: Codec> Base64<S> {
    /// Creates a lazy allocation-free display after encoding preflight.
    pub fn display<'a>(&self, input: &'a [u8]) -> Result<EncodedDisplay<'a>, OneShotError> {
        self.encoded_len(input.len())?;
        Ok(EncodedDisplay {
            settings: self.settings(),
            input,
        })
    }

    /// Encodes through `core::fmt::Write` without allocating.
    ///
    /// On formatter failure, confirmed progress excludes the failing call
    /// because `fmt::Write` cannot report whether that call partially mutated
    /// its sink.
    pub fn encode_to_fmt<W: core::fmt::Write + ?Sized>(
        &self,
        input: &[u8],
        writer: &mut W,
    ) -> Result<usize, FormatWriteError> {
        let chunks = self
            .encoded_chunks(input)
            .map_err(FormatWriteError::Encoding)?;
        let mut confirmed = 0;
        for chunk in chunks {
            let text = chunk_text(&chunk).map_err(|_| FormatWriteError::Backend {
                fault: BackendFault::ImpossibleState,
                confirmed,
            })?;
            writer
                .write_str(text)
                .map_err(|_| FormatWriteError::Formatter { confirmed })?;
            confirmed += text.len();
        }
        Ok(confirmed)
    }

    /// Encodes through an exact-progress counted sink without allocating.
    pub fn encode_to_counted<W: CountedSink + ?Sized>(
        &self,
        input: &[u8],
        writer: &mut W,
    ) -> Result<usize, CountedWriteError<W::Error>> {
        let chunks = self
            .encoded_chunks(input)
            .map_err(CountedWriteError::Encoding)?;
        let mut committed = 0;
        for chunk in chunks {
            let mut pending = chunk.as_bytes();
            while !pending.is_empty() {
                let written = writer
                    .write(pending)
                    .map_err(|error| CountedWriteError::Sink { error, committed })?;
                if written == 0 {
                    return Err(CountedWriteError::WriteZero { committed });
                }
                if written > pending.len() {
                    return Err(CountedWriteError::InvalidCount {
                        reported: written,
                        offered: pending.len(),
                        committed,
                    });
                }
                committed += written;
                pending = &pending[written..];
            }
        }
        Ok(committed)
    }
}

fn chunk_text(chunk: &EncodedChunk) -> Result<&str, core::str::Utf8Error> {
    chunk.as_str()
}
