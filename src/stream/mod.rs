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
//! The streaming adapters use fixed stack buffers up to 1024 bytes for bounded
//! I/O staging. This keeps heap behavior predictable, but callers embedding
//! these adapters in constrained `std` environments should account for that
//! stack footprint in deeply nested writer/reader chains.
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

mod common;
mod decoder;
mod decoder_reader;
mod encoder;
mod encoder_reader;
mod queue;

pub use decoder::Decoder;
pub use decoder_reader::DecoderReader;
pub use encoder::Encoder;
pub use encoder_reader::EncoderReader;

use common::{
    decode_error_to_io, encode_error_to_io, redacted_inner_state, stream_decoder_failed_error,
    stream_encoder_failed_error, trailing_input_after_padding_error,
};
use queue::OutputQueue;
