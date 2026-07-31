//! Internal ownership boundaries for the emerging 2.0 implementation.

// Commit 5 establishes this internal value before Commit 6 wires it into the
// public specification model.
#[allow(dead_code)]
pub(crate) mod alphabet;
#[cfg(feature = "alloc")]
mod append;
mod backend_health;
mod bounded;
mod chunks;
mod const_transforms;
#[allow(dead_code)]
pub(crate) mod contracts;
mod formatting;
mod in_place;
#[allow(dead_code)]
pub(crate) mod incremental;
#[allow(dead_code)]
pub(crate) mod incremental_decoder;
#[allow(dead_code)]
mod lifecycle;
mod ordinary;
#[cfg(feature = "alloc")]
mod ordinary_alloc;
mod secret;
mod secret_in_place;
#[allow(dead_code)]
pub(crate) mod specifications;
#[allow(dead_code)]
mod wrapping;

pub use alphabet::{ValidatedAlphabet, ValidatedAlphabetError};
pub use bounded::{BufferLengthError, DecodedArray, EncodedArray, SecretArray};
pub use chunks::{EncodedChunk, EncodedChunks};
pub use const_transforms::ConstTransformError;
pub use contracts::{
    AssuranceClass, Atomicity, BackendClass, BackendFault, Failure, InputError, InputErrorKind,
    OperationError, OutputFull, Progress, ProtocolScope, Status, Step, TerminalError,
};
pub use formatting::{CountedSink, CountedWriteError, EncodedDisplay, FormatWriteError};
pub use in_place::InPlaceError;
pub use incremental::EncoderState;
pub use incremental_decoder::DecoderState;
pub use ordinary::OneShotError;
pub use specifications::{
    Base64, Codec, CodecBuilder, CodecBuilderError, CodecSettings, DecodePadding, EncodePadding,
    RuntimeSpec, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, StrictStandardPadded, StrictStandardUnpadded, StrictUrlSafePadded,
    StrictUrlSafeUnpadded, TrailingBits,
};

#[cfg(test)]
mod alphabet_tests;
#[cfg(test)]
mod append_tests;
#[cfg(test)]
mod chunk_tests;
#[cfg(test)]
mod const_buffer_tests;
#[cfg(test)]
mod contract_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod formatting_tests;
#[cfg(test)]
mod in_place_tests;
#[cfg(test)]
mod incremental_decoder_tests;
#[cfg(test)]
mod incremental_decoder_unpadded_tests;
#[cfg(test)]
mod incremental_encoder_tests;
#[cfg(test)]
mod one_shot_tests;
#[cfg(test)]
mod rfc4648_oracle;
#[cfg(test)]
mod secret_in_place_tests;
#[cfg(test)]
mod specification_tests;
#[cfg(test)]
mod wrapping_tests;
