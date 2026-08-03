use core::num::NonZeroUsize;

use crate::Base64MultibaseEncoding;

/// Non-failing incremental state after one operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Base64MultibaseStatus {
    /// More input or an explicit finish call is required.
    NeedInput,
    /// Retry with at least the reported destination capacity.
    OutputFull(NonZeroUsize),
    /// The complete value was emitted and the state is terminal.
    Complete,
}

/// Exact incremental progress for one call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Base64MultibaseStep {
    input_consumed: usize,
    output_produced: usize,
    status: Base64MultibaseStatus,
}

impl Base64MultibaseStep {
    pub(crate) const fn new(
        input_consumed: usize,
        output_produced: usize,
        status: Base64MultibaseStatus,
    ) -> Self {
        Self {
            input_consumed,
            output_produced,
            status,
        }
    }

    /// Returns source bytes accepted by this call.
    #[must_use]
    pub const fn input_consumed(self) -> usize {
        self.input_consumed
    }

    /// Returns destination bytes initialized by this call.
    #[must_use]
    pub const fn output_produced(self) -> usize {
        self.output_produced
    }

    /// Returns the resulting incremental state.
    #[must_use]
    pub const fn status(self) -> Base64MultibaseStatus {
        self.status
    }
}

/// Successful caller-owned decode result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedBase64Multibase {
    encoding: Base64MultibaseEncoding,
    written: usize,
}

impl DecodedBase64Multibase {
    pub(crate) const fn new(encoding: Base64MultibaseEncoding, written: usize) -> Self {
        Self { encoding, written }
    }

    /// Returns the exact prefix-selected encoding.
    #[must_use]
    pub const fn encoding(self) -> Base64MultibaseEncoding {
        self.encoding
    }

    /// Returns decoded bytes written to the destination prefix.
    #[must_use]
    pub const fn written(self) -> usize {
        self.written
    }
}

/// Successful allocated decode result.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedBase64MultibaseVec {
    encoding: Base64MultibaseEncoding,
    bytes: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl DecodedBase64MultibaseVec {
    pub(crate) const fn new(encoding: Base64MultibaseEncoding, bytes: alloc::vec::Vec<u8>) -> Self {
        Self { encoding, bytes }
    }

    /// Returns the exact prefix-selected encoding.
    #[must_use]
    pub const fn encoding(&self) -> Base64MultibaseEncoding {
        self.encoding
    }

    /// Returns the decoded bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the result and returns decoded bytes.
    #[must_use]
    pub fn into_bytes(self) -> alloc::vec::Vec<u8> {
        self.bytes
    }
}
