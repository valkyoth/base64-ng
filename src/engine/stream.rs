#[cfg(feature = "stream")]
use crate::stream;
use crate::{Alphabet, Engine};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Wraps a `std::io::Write` value in a streaming Base64 encoder.
    ///
    /// This is a convenience constructor for [`stream::Encoder::new`] that
    /// keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Write;
    /// use base64_ng::STANDARD;
    ///
    /// let mut encoder = STANDARD.encoder_writer(Vec::new());
    /// encoder.write_all(b"hello").unwrap();
    /// assert_eq!(encoder.finish().unwrap(), b"aGVsbG8=");
    /// ```
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn encoder_writer<W>(&self, inner: W) -> stream::Encoder<W, A, PAD> {
        stream::Encoder::new(inner, *self)
    }

    /// Wraps a `std::io::Write` value in a streaming Base64 decoder.
    ///
    /// This is a convenience constructor for [`stream::Decoder::new`] that
    /// keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Write;
    /// use base64_ng::STANDARD;
    ///
    /// let mut decoder = STANDARD.decoder_writer(Vec::new());
    /// decoder.write_all(b"aGVsbG8=").unwrap();
    /// assert_eq!(decoder.finish().unwrap(), b"hello");
    /// ```
    ///
    /// # Security
    ///
    /// Streaming decoders use the normal strict decode path, not the
    /// [`crate::ct`] module. Do not use this adapter for secret-bearing
    /// payloads when malformed-input timing matters.
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn decoder_writer<W>(&self, inner: W) -> stream::Decoder<W, A, PAD> {
        stream::Decoder::new(inner, *self)
    }

    /// Wraps a `std::io::Read` value in a streaming Base64 encoder.
    ///
    /// This is a convenience constructor for [`stream::EncoderReader::new`]
    /// that keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Read;
    /// use base64_ng::STANDARD;
    ///
    /// let mut reader = STANDARD.encoder_reader(&b"hello"[..]);
    /// let mut encoded = String::new();
    /// reader.read_to_string(&mut encoded).unwrap();
    /// assert_eq!(encoded, "aGVsbG8=");
    /// ```
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn encoder_reader<R>(&self, inner: R) -> stream::EncoderReader<R, A, PAD> {
        stream::EncoderReader::new(inner, *self)
    }

    /// Wraps a `std::io::Read` value in a streaming Base64 decoder.
    ///
    /// This is a convenience constructor for [`stream::DecoderReader::new`]
    /// that keeps the selected engine attached to the call site.
    ///
    /// ```
    /// use std::io::Read;
    /// use base64_ng::STANDARD;
    ///
    /// let mut reader = STANDARD.decoder_reader(&b"aGVsbG8="[..]);
    /// let mut decoded = Vec::new();
    /// reader.read_to_end(&mut decoded).unwrap();
    /// assert_eq!(decoded, b"hello");
    /// ```
    ///
    /// # Security
    ///
    /// Streaming decoder readers use the normal strict decode path, not the
    /// [`crate::ct`] module. Do not use this adapter for secret-bearing
    /// payloads when malformed-input timing matters.
    #[cfg(feature = "stream")]
    #[must_use]
    pub fn decoder_reader<R>(&self, inner: R) -> stream::DecoderReader<R, A, PAD> {
        stream::DecoderReader::new(inner, *self)
    }
}
