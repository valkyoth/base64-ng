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
pub mod compat;
mod const_transforms;
#[allow(dead_code)]
pub(crate) mod contracts;
mod formatting;
mod in_place;
#[allow(dead_code)]
pub(crate) mod incremental;
#[allow(dead_code)]
pub(crate) mod incremental_decoder;
pub mod legacy;
#[allow(dead_code)]
mod lifecycle;
mod ordinary;
#[cfg(feature = "alloc")]
mod ordinary_alloc;
mod profiles;
#[cfg(not(feature = "secrets"))]
pub(crate) mod secret;
#[cfg(feature = "secrets")]
pub mod secret;
#[cfg(feature = "secrets")]
mod secret_decoder;
#[cfg(feature = "secrets")]
mod secret_encoder;
#[cfg(kani)]
pub(crate) use secret_encoder::{
    final_quantum_output_len_for_proof, require_disjoint_ranges_for_proof,
};
mod secret_in_place;
#[allow(dead_code)]
pub(crate) mod specifications;
pub mod web;
#[allow(dead_code)]
mod wrapping;

pub use alphabet::{BINHEX_ALPHABET, ValidatedAlphabet, ValidatedAlphabetError};
pub use bounded::{BufferLengthError, DecodedArray, EncodedArray};
pub use chunks::{EncodedChunk, EncodedChunks};
pub use const_transforms::ConstTransformError;
pub use contracts::{
    AssuranceClass, Atomicity, BackendClass, BackendFault, Failure, InputError, InputErrorKind,
    OperationError, OutputFull, Progress, ProtocolScope, Status, Step, TerminalError,
};
pub use formatting::{CountedSink, CountedWriteError, EncodedDisplay, FormatWriteError};
pub use in_place::InPlaceError;
#[cfg(kani)]
pub(crate) use in_place::{encoded_tail_len, quantum_decoded_len, tail_decoded_len};
pub use incremental::EncoderState;
pub use incremental_decoder::DecoderState;
pub use ordinary::OneShotError;
pub use profiles::{
    BCRYPT_ALPHABET_NO_PAD, BodyCodec, CRYPT_ALPHABET_NO_PAD, IMAP_MUTF7_ALPHABET_NO_PAD,
    MIME_BODY_STRICT, PBKDF2_ALPHABET_NO_PAD, PEM_BODY_CRLF, PEM_BODY_LF,
};
pub use specifications::{
    Base64, Codec, CodecBuilder, CodecBuilderError, CodecSettings, DecodePadding, EncodePadding,
    RuntimeSpec, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, StrictStandardPadded, StrictStandardUnpadded, StrictUrlSafePadded,
    StrictUrlSafeUnpadded, TrailingBits,
};
pub use wrapping::{
    LineEnding as BodyLineEnding, LineWrap as BodyWrap, LineWrapError as BodyWrapError,
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
mod legacy_tests;
#[cfg(test)]
mod one_shot_tests;
#[cfg(test)]
mod profile_tests;
#[cfg(test)]
mod rfc4648_oracle;
#[cfg(test)]
mod secret_decoder_tests;
#[cfg(test)]
mod secret_encoder_tests;
#[cfg(test)]
mod secret_in_place_tests;
#[cfg(test)]
mod secret_storage_tests;
#[cfg(test)]
mod specification_tests;
#[cfg(test)]
mod web_no_alloc_tests;
#[cfg(test)]
mod web_tests;
#[cfg(test)]
mod wrapping_tests;
