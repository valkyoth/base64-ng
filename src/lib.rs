#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

//! `base64-ng` is a `no_std`-first Base64 encoder and decoder.
//!
//! This initial release provides strict scalar RFC 4648-style behavior and
//! caller-owned output buffers. Future SIMD fast paths will be required to
//! match this scalar module byte-for-byte.
//!
//! # Examples
//!
//! Encode and decode with caller-owned buffers:
//!
//! ```
//! use base64_ng::{STANDARD, checked_encoded_len};
//!
//! let input = b"hello";
//! const ENCODED_CAPACITY: usize = match checked_encoded_len(5, true) {
//!     Some(len) => len,
//!     None => panic!("encoded length overflow"),
//! };
//! let mut encoded = [0u8; ENCODED_CAPACITY];
//! let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"aGVsbG8=");
//!
//! let mut decoded = [0u8; 5];
//! let decoded_len = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
//! assert_eq!(&decoded[..decoded_len], input);
//! ```
//!
//! Use the URL-safe no-padding engine:
//!
//! ```
//! use base64_ng::URL_SAFE_NO_PAD;
//!
//! let mut encoded = [0u8; 3];
//! let encoded_len = URL_SAFE_NO_PAD.encode_slice(b"\xfb\xff", &mut encoded).unwrap();
//! assert_eq!(&encoded[..encoded_len], b"-_8");
//! ```

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "simd")]
mod simd;

#[cfg(feature = "stream")]
pub mod stream {
    //! Streaming Base64 wrappers for `std::io`.
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
    use std::collections::VecDeque;
    use std::io::{self, Read, Write};

    /// A streaming Base64 encoder for `std::io::Write`.
    pub struct Encoder<W, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: W,
        engine: Engine<A, PAD>,
        pending: [u8; 2],
        pending_len: usize,
    }

    impl<W, A, const PAD: bool> Encoder<W, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming encoder.
        #[must_use]
        pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
            Self {
                inner,
                engine,
                pending: [0; 2],
                pending_len: 0,
            }
        }

        /// Returns a shared reference to the wrapped writer.
        #[must_use]
        pub const fn get_ref(&self) -> &W {
            &self.inner
        }

        /// Returns a mutable reference to the wrapped writer.
        pub fn get_mut(&mut self) -> &mut W {
            &mut self.inner
        }

        /// Consumes the encoder without flushing pending input.
        ///
        /// Prefer [`Self::finish`] when the encoded output must be complete.
        #[must_use]
        pub fn into_inner(self) -> W {
            self.inner
        }
    }

    impl<W, A, const PAD: bool> Encoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        /// Writes any pending input, flushes the wrapped writer, and returns it.
        pub fn finish(mut self) -> io::Result<W> {
            self.write_pending_final()?;
            self.inner.flush()?;
            Ok(self.inner)
        }

        fn write_pending_final(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut encoded = [0u8; 4];
            let written = self
                .engine
                .encode_slice(&self.pending[..self.pending_len], &mut encoded)
                .map_err(encode_error_to_io)?;
            self.inner.write_all(&encoded[..written])?;
            self.pending_len = 0;
            Ok(())
        }
    }

    impl<W, A, const PAD: bool> Write for Encoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
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
                let written = self
                    .engine
                    .encode_slice(&chunk, &mut encoded)
                    .map_err(encode_error_to_io)?;
                self.inner.write_all(&encoded[..written])?;
                self.pending_len = 0;
                consumed += needed;
            }

            let remaining = &input[consumed..];
            let full_len = remaining.len() / 3 * 3;
            let mut offset = 0;
            let mut encoded = [0u8; 1024];
            while offset < full_len {
                let mut take = core::cmp::min(full_len - offset, 768);
                take -= take % 3;
                debug_assert!(take > 0);

                let written = self
                    .engine
                    .encode_slice(&remaining[offset..offset + take], &mut encoded)
                    .map_err(encode_error_to_io)?;
                self.inner.write_all(&encoded[..written])?;
                offset += take;
            }

            let tail = &remaining[full_len..];
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();

            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }

    fn encode_error_to_io(err: EncodeError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }

    /// A streaming Base64 decoder for `std::io::Write`.
    pub struct Decoder<W, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: W,
        engine: Engine<A, PAD>,
        pending: [u8; 4],
        pending_len: usize,
        finished: bool,
    }

    impl<W, A, const PAD: bool> Decoder<W, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming decoder.
        #[must_use]
        pub const fn new(inner: W, engine: Engine<A, PAD>) -> Self {
            Self {
                inner,
                engine,
                pending: [0; 4],
                pending_len: 0,
                finished: false,
            }
        }

        /// Returns a shared reference to the wrapped writer.
        #[must_use]
        pub const fn get_ref(&self) -> &W {
            &self.inner
        }

        /// Returns a mutable reference to the wrapped writer.
        pub fn get_mut(&mut self) -> &mut W {
            &mut self.inner
        }

        /// Consumes the decoder without flushing pending input.
        ///
        /// Prefer [`Self::finish`] when the decoded output must be complete.
        #[must_use]
        pub fn into_inner(self) -> W {
            self.inner
        }
    }

    impl<W, A, const PAD: bool> Decoder<W, A, PAD>
    where
        W: Write,
        A: Alphabet,
    {
        /// Validates final pending input, flushes the wrapped writer, and returns it.
        pub fn finish(mut self) -> io::Result<W> {
            self.write_pending_final()?;
            self.inner.flush()?;
            Ok(self.inner)
        }

        fn write_pending_final(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut decoded = [0u8; 3];
            let written = self
                .engine
                .decode_slice(&self.pending[..self.pending_len], &mut decoded)
                .map_err(decode_error_to_io)?;
            self.inner.write_all(&decoded[..written])?;
            self.pending_len = 0;
            Ok(())
        }

        fn write_full_quad(&mut self, input: [u8; 4]) -> io::Result<()> {
            let mut decoded = [0u8; 3];
            let written = self
                .engine
                .decode_slice(&input, &mut decoded)
                .map_err(decode_error_to_io)?;
            self.inner.write_all(&decoded[..written])?;
            if written < 3 {
                self.finished = true;
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
            if input.is_empty() {
                return Ok(0);
            }
            if self.finished {
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
                self.write_full_quad(quad)?;
                self.pending_len = 0;
                consumed += needed;
                if self.finished && consumed < input.len() {
                    return Err(trailing_input_after_padding_error());
                }
            }

            let remaining = &input[consumed..];
            let full_len = remaining.len() / 4 * 4;
            let mut offset = 0;
            while offset < full_len {
                let quad = [
                    remaining[offset],
                    remaining[offset + 1],
                    remaining[offset + 2],
                    remaining[offset + 3],
                ];
                self.write_full_quad(quad)?;
                offset += 4;
                if self.finished && offset < remaining.len() {
                    return Err(trailing_input_after_padding_error());
                }
            }

            let tail = &remaining[full_len..];
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();

            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
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

    /// A streaming Base64 decoder for `std::io::Read`.
    ///
    /// For padded engines, this reader stops at the terminal padded Base64
    /// block and leaves later bytes unread in the wrapped reader. This preserves
    /// boundaries for callers that decode one Base64 payload from a larger
    /// stream.
    pub struct DecoderReader<R, A, const PAD: bool>
    where
        A: Alphabet,
    {
        inner: R,
        engine: Engine<A, PAD>,
        pending: [u8; 4],
        pending_len: usize,
        output: VecDeque<u8>,
        finished: bool,
        terminal_seen: bool,
    }

    impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming decoder reader.
        #[must_use]
        pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
            Self {
                inner,
                engine,
                pending: [0; 4],
                pending_len: 0,
                output: VecDeque::new(),
                finished: false,
                terminal_seen: false,
            }
        }

        /// Returns a shared reference to the wrapped reader.
        #[must_use]
        pub const fn get_ref(&self) -> &R {
            &self.inner
        }

        /// Returns a mutable reference to the wrapped reader.
        pub fn get_mut(&mut self) -> &mut R {
            &mut self.inner
        }

        /// Consumes the decoder reader and returns the wrapped reader.
        #[must_use]
        pub fn into_inner(self) -> R {
            self.inner
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

            while self.output.is_empty() && !self.finished {
                self.fill_output()?;
            }

            let mut written = 0;
            while written < output.len() {
                let Some(byte) = self.output.pop_front() else {
                    break;
                };
                output[written] = byte;
                written += 1;
            }

            Ok(written)
        }
    }

    impl<R, A, const PAD: bool> DecoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn fill_output(&mut self) -> io::Result<()> {
            if self.terminal_seen {
                self.finished = true;
                return Ok(());
            }

            let mut input = [0u8; 4];
            let read = self.inner.read(&mut input[..4 - self.pending_len])?;
            if read == 0 {
                self.finished = true;
                self.push_final_pending()?;
                return Ok(());
            }

            self.pending[self.pending_len..self.pending_len + read].copy_from_slice(&input[..read]);
            self.pending_len += read;
            if self.pending_len < 4 {
                return Ok(());
            }

            let quad = self.pending;
            self.pending_len = 0;
            self.push_decoded(&quad)?;
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
            self.pending_len = 0;
            self.push_decoded(&pending[..pending_len])
        }

        fn push_decoded(&mut self, input: &[u8]) -> io::Result<()> {
            let mut decoded = [0u8; 3];
            let written = self
                .engine
                .decode_slice(input, &mut decoded)
                .map_err(decode_error_to_io)?;
            self.output.extend(&decoded[..written]);
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
        inner: R,
        engine: Engine<A, PAD>,
        pending: [u8; 2],
        pending_len: usize,
        output: VecDeque<u8>,
        finished: bool,
    }

    impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new streaming encoder reader.
        #[must_use]
        pub fn new(inner: R, engine: Engine<A, PAD>) -> Self {
            Self {
                inner,
                engine,
                pending: [0; 2],
                pending_len: 0,
                output: VecDeque::new(),
                finished: false,
            }
        }

        /// Returns a shared reference to the wrapped reader.
        #[must_use]
        pub const fn get_ref(&self) -> &R {
            &self.inner
        }

        /// Returns a mutable reference to the wrapped reader.
        pub fn get_mut(&mut self) -> &mut R {
            &mut self.inner
        }

        /// Consumes the encoder reader and returns the wrapped reader.
        #[must_use]
        pub fn into_inner(self) -> R {
            self.inner
        }
    }

    impl<R, A, const PAD: bool> Read for EncoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if output.is_empty() {
                return Ok(0);
            }

            while self.output.is_empty() && !self.finished {
                self.fill_output()?;
            }

            let mut written = 0;
            while written < output.len() {
                let Some(byte) = self.output.pop_front() else {
                    break;
                };
                output[written] = byte;
                written += 1;
            }

            Ok(written)
        }
    }

    impl<R, A, const PAD: bool> EncoderReader<R, A, PAD>
    where
        R: Read,
        A: Alphabet,
    {
        fn fill_output(&mut self) -> io::Result<()> {
            let mut input = [0u8; 768];
            let read = self.inner.read(&mut input)?;
            if read == 0 {
                self.finished = true;
                self.push_final_pending()?;
                return Ok(());
            }

            let mut consumed = 0;
            if self.pending_len > 0 {
                let needed = 3 - self.pending_len;
                if read < needed {
                    self.pending[self.pending_len..self.pending_len + read]
                        .copy_from_slice(&input[..read]);
                    self.pending_len += read;
                    return Ok(());
                }

                let mut chunk = [0u8; 3];
                chunk[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
                chunk[self.pending_len..].copy_from_slice(&input[..needed]);
                self.push_encoded(&chunk)?;
                self.pending_len = 0;
                consumed += needed;
            }

            let remaining = &input[consumed..read];
            let full_len = remaining.len() / 3 * 3;
            if full_len > 0 {
                self.push_encoded(&remaining[..full_len])?;
            }

            let tail = &remaining[full_len..];
            self.pending[..tail.len()].copy_from_slice(tail);
            self.pending_len = tail.len();
            Ok(())
        }

        fn push_final_pending(&mut self) -> io::Result<()> {
            if self.pending_len == 0 {
                return Ok(());
            }

            let mut pending = [0u8; 2];
            pending[..self.pending_len].copy_from_slice(&self.pending[..self.pending_len]);
            let pending_len = self.pending_len;
            self.pending_len = 0;
            self.push_encoded(&pending[..pending_len])
        }

        fn push_encoded(&mut self, input: &[u8]) -> io::Result<()> {
            let mut encoded = [0u8; 1024];
            let written = self
                .engine
                .encode_slice(input, &mut encoded)
                .map_err(encode_error_to_io)?;
            self.output.extend(&encoded[..written]);
            Ok(())
        }
    }
}

/// Constant-time-oriented scalar decoding APIs.
///
/// This module is separate from the default decoder so callers can opt into a
/// slower path with a narrower timing target. It avoids lookup tables indexed
/// by secret input bytes while mapping Base64 symbols, but it is not documented
/// as a formally verified cryptographic constant-time API.
pub mod ct {
    use super::{Alphabet, DecodeError, Standard, UrlSafe, ct_decode_slice};
    use core::marker::PhantomData;

    /// Standard Base64 constant-time-oriented decoder with padding.
    pub const STANDARD: CtEngine<Standard, true> = CtEngine::new();

    /// Standard Base64 constant-time-oriented decoder without padding.
    pub const STANDARD_NO_PAD: CtEngine<Standard, false> = CtEngine::new();

    /// URL-safe Base64 constant-time-oriented decoder with padding.
    pub const URL_SAFE: CtEngine<UrlSafe, true> = CtEngine::new();

    /// URL-safe Base64 constant-time-oriented decoder without padding.
    pub const URL_SAFE_NO_PAD: CtEngine<UrlSafe, false> = CtEngine::new();

    /// A zero-sized constant-time-oriented Base64 decoder.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct CtEngine<A, const PAD: bool> {
        alphabet: PhantomData<A>,
    }

    impl<A, const PAD: bool> CtEngine<A, PAD>
    where
        A: Alphabet,
    {
        /// Creates a new constant-time-oriented decoder engine.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                alphabet: PhantomData,
            }
        }

        /// Decodes `input` into `output`, returning the number of bytes
        /// written.
        ///
        /// This path uses branch-minimized arithmetic for Base64 symbol
        /// mapping and avoids secret-indexed lookup tables. Input length,
        /// padding length, output length, and final success or failure remain
        /// public. Malformed input errors are intentionally non-localized; use
        /// the normal strict decoder when exact error indexes are required.
        ///
        /// # Examples
        ///
        /// ```
        /// use base64_ng::ct;
        ///
        /// let mut output = [0u8; 5];
        /// let written = ct::STANDARD
        ///     .decode_slice(b"aGVsbG8=", &mut output)
        ///     .unwrap();
        ///
        /// assert_eq!(&output[..written], b"hello");
        /// ```
        pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
            ct_decode_slice::<A, PAD>(input, output)
        }
    }
}

/// Standard Base64 engine with padding.
pub const STANDARD: Engine<Standard, true> = Engine::new();

/// Standard Base64 engine without padding.
pub const STANDARD_NO_PAD: Engine<Standard, false> = Engine::new();

/// URL-safe Base64 engine with padding.
pub const URL_SAFE: Engine<UrlSafe, true> = Engine::new();

/// URL-safe Base64 engine without padding.
pub const URL_SAFE_NO_PAD: Engine<UrlSafe, false> = Engine::new();

/// Returns the encoded length for an input length and padding policy.
///
/// This function returns [`EncodeError::LengthOverflow`] instead of panicking.
/// Use [`checked_encoded_len`] when an `Option<usize>` is more convenient.
///
/// # Examples
///
/// ```
/// use base64_ng::encoded_len;
///
/// assert_eq!(encoded_len(5, true).unwrap(), 8);
/// assert_eq!(encoded_len(5, false).unwrap(), 7);
/// assert!(encoded_len(usize::MAX, true).is_err());
/// ```
pub const fn encoded_len(input_len: usize, padded: bool) -> Result<usize, EncodeError> {
    match checked_encoded_len(input_len, padded) {
        Some(len) => Ok(len),
        None => Err(EncodeError::LengthOverflow),
    }
}

/// Returns the encoded length, or `None` if it would overflow `usize`.
///
/// # Examples
///
/// ```
/// use base64_ng::checked_encoded_len;
///
/// assert_eq!(checked_encoded_len(5, true), Some(8));
/// assert_eq!(checked_encoded_len(usize::MAX, true), None);
/// ```
#[must_use]
pub const fn checked_encoded_len(input_len: usize, padded: bool) -> Option<usize> {
    let groups = input_len / 3;
    if groups > usize::MAX / 4 {
        return None;
    }
    let full = groups * 4;
    let rem = input_len % 3;
    if rem == 0 {
        Some(full)
    } else if padded {
        full.checked_add(4)
    } else {
        full.checked_add(rem + 1)
    }
}

/// Returns the maximum decoded length for an encoded input length.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_capacity;
///
/// assert_eq!(decoded_capacity(8), 6);
/// assert_eq!(decoded_capacity(7), 5);
/// ```
#[must_use]
pub const fn decoded_capacity(encoded_len: usize) -> usize {
    let rem = encoded_len % 4;
    encoded_len / 4 * 3
        + if rem == 2 {
            1
        } else if rem == 3 {
            2
        } else {
            0
        }
}

/// Returns the exact decoded length implied by input length and padding.
///
/// This validates padding placement and impossible lengths, but it does not
/// validate alphabet membership or non-canonical trailing bits.
///
/// # Examples
///
/// ```
/// use base64_ng::decoded_len;
///
/// assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
/// assert_eq!(decoded_len(b"aGVsbG8", false).unwrap(), 5);
/// ```
pub fn decoded_len(input: &[u8], padded: bool) -> Result<usize, DecodeError> {
    if padded {
        decoded_len_padded(input)
    } else {
        decoded_len_unpadded(input)
    }
}

/// A Base64 alphabet.
pub trait Alphabet {
    /// Encoding table indexed by 6-bit values.
    const ENCODE: [u8; 64];

    /// Decode one byte into a 6-bit value.
    fn decode(byte: u8) -> Option<u8>;
}

/// The RFC 4648 standard Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Standard;

impl Alphabet for Standard {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_ascii_base64(byte, Self::ENCODE[62], Self::ENCODE[63])
    }
}

/// The RFC 4648 URL-safe Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UrlSafe;

impl Alphabet for UrlSafe {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_ascii_base64(byte, Self::ENCODE[62], Self::ENCODE[63])
    }
}

#[inline]
const fn encode_base64_value<A: Alphabet>(value: u8) -> u8 {
    encode_ascii_base64(value, A::ENCODE[62], A::ENCODE[63])
}

#[inline]
const fn encode_ascii_base64(value: u8, value_62_byte: u8, value_63_byte: u8) -> u8 {
    let upper = mask_if(value < 26);
    let lower = mask_if(value.wrapping_sub(26) < 26);
    let digit = mask_if(value.wrapping_sub(52) < 10);
    let value_62 = mask_if(value == 0x3e);
    let value_63 = mask_if(value == 0x3f);

    (value.wrapping_add(b'A') & upper)
        | (value.wrapping_sub(26).wrapping_add(b'a') & lower)
        | (value.wrapping_sub(52).wrapping_add(b'0') & digit)
        | (value_62_byte & value_62)
        | (value_63_byte & value_63)
}

#[inline]
fn decode_ascii_base64(byte: u8, value_62_byte: u8, value_63_byte: u8) -> Option<u8> {
    let upper = mask_if(byte.wrapping_sub(b'A') <= b'Z' - b'A');
    let lower = mask_if(byte.wrapping_sub(b'a') <= b'z' - b'a');
    let digit = mask_if(byte.wrapping_sub(b'0') <= b'9' - b'0');
    let value_62 = mask_if(byte == value_62_byte);
    let value_63 = mask_if(byte == value_63_byte);
    let valid = upper | lower | digit | value_62 | value_63;

    let decoded = (byte.wrapping_sub(b'A') & upper)
        | (byte.wrapping_sub(b'a').wrapping_add(26) & lower)
        | (byte.wrapping_sub(b'0').wrapping_add(52) & digit)
        | (0x3e & value_62)
        | (0x3f & value_63);

    if valid == 0 { None } else { Some(decoded) }
}

#[inline]
const fn mask_if(condition: bool) -> u8 {
    0u8.wrapping_sub(condition as u8)
}

mod backend {
    use super::{
        Alphabet, DecodeError, EncodeError, checked_encoded_len, decode_padded, decode_unpadded,
        encode_base64_value,
    };

    pub(super) fn encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        #[cfg(feature = "simd")]
        match super::simd::active_backend() {
            super::simd::ActiveBackend::Scalar => {}
        }

        scalar_encode_slice::<A, PAD>(input, output)
    }

    pub(super) fn decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        #[cfg(feature = "simd")]
        match super::simd::active_backend() {
            super::simd::ActiveBackend::Scalar => {}
        }

        scalar_decode_slice::<A, PAD>(input, output)
    }

    #[cfg(test)]
    pub(super) fn scalar_reference_encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        scalar_encode_slice::<A, PAD>(input, output)
    }

    #[cfg(test)]
    pub(super) fn scalar_reference_decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        scalar_decode_slice::<A, PAD>(input, output)
    }

    fn scalar_encode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError>
    where
        A: Alphabet,
    {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        if output.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }

        let mut read = 0;
        let mut write = 0;
        while read + 3 <= input.len() {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = encode_base64_value::<A>(b0 >> 2);
            output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

            read += 3;
            write += 4;
        }

        match input.len() - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                    write += 2;
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                write += 3;
                if PAD {
                    output[write] = b'=';
                    write += 1;
                }
            }
            _ => unreachable!(),
        }

        Ok(write)
    }

    fn scalar_decode_slice<A, const PAD: bool>(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError>
    where
        A: Alphabet,
    {
        if input.is_empty() {
            return Ok(0);
        }

        if PAD {
            decode_padded::<A>(input, output)
        } else {
            decode_unpadded::<A>(input, output)
        }
    }
}

/// A zero-sized Base64 engine parameterized by alphabet and padding policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Engine<A, const PAD: bool> {
    alphabet: core::marker::PhantomData<A>,
}

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Creates a new engine value.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alphabet: core::marker::PhantomData,
        }
    }

    /// Returns the encoded length for this engine's padding policy.
    pub const fn encoded_len(&self, input_len: usize) -> Result<usize, EncodeError> {
        encoded_len(input_len, PAD)
    }

    /// Returns the encoded length for this engine, or `None` on overflow.
    #[must_use]
    pub const fn checked_encoded_len(&self, input_len: usize) -> Option<usize> {
        checked_encoded_len(input_len, PAD)
    }

    /// Returns the exact decoded length implied by input length and padding.
    ///
    /// This validates padding placement and impossible lengths, but it does not
    /// validate alphabet membership or non-canonical trailing bits.
    pub fn decoded_len(&self, input: &[u8]) -> Result<usize, DecodeError> {
        decoded_len(input, PAD)
    }

    /// Returns the exact decoded length for the explicit legacy profile.
    ///
    /// The legacy profile ignores ASCII space, tab, carriage return, and line
    /// feed bytes before applying the same alphabet, padding, and canonical-bit
    /// checks as strict decoding.
    pub fn decoded_len_legacy(&self, input: &[u8]) -> Result<usize, DecodeError> {
        validate_legacy_decode::<A, PAD>(input)
    }

    /// Encodes a fixed-size input into a fixed-size output array in const contexts.
    ///
    /// Stable Rust does not yet allow this API to return an array whose length
    /// is computed from `INPUT_LEN` directly. Instead, the caller supplies the
    /// output length through the destination type and this function panics
    /// during const evaluation if the length is wrong.
    ///
    /// # Panics
    ///
    /// Panics if `OUTPUT_LEN` is not exactly the encoded length for `INPUT_LEN`
    /// and this engine's padding policy, or if that length overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// const HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
    /// const URL_SAFE: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");
    ///
    /// assert_eq!(&HELLO, b"aGVsbG8=");
    /// assert_eq!(&URL_SAFE, b"-_8");
    /// ```
    ///
    /// Incorrect output lengths fail during const evaluation:
    ///
    /// ```compile_fail
    /// use base64_ng::STANDARD;
    ///
    /// const TOO_SHORT: [u8; 7] = STANDARD.encode_array(b"hello");
    /// ```
    #[must_use]
    pub const fn encode_array<const INPUT_LEN: usize, const OUTPUT_LEN: usize>(
        &self,
        input: &[u8; INPUT_LEN],
    ) -> [u8; OUTPUT_LEN] {
        let Some(required) = checked_encoded_len(INPUT_LEN, PAD) else {
            panic!("encoded base64 length overflows usize");
        };
        assert!(
            required == OUTPUT_LEN,
            "base64 output array has incorrect length"
        );

        let mut output = [0u8; OUTPUT_LEN];
        let mut read = 0;
        let mut write = 0;
        while INPUT_LEN - read >= 3 {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];

            output[write] = encode_base64_value::<A>(b0 >> 2);
            output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            output[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            output[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);

            read += 3;
            write += 4;
        }

        match INPUT_LEN - read {
            0 => {}
            1 => {
                let b0 = input[read];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                write += 2;
                if PAD {
                    output[write] = b'=';
                    output[write + 1] = b'=';
                }
            }
            2 => {
                let b0 = input[read];
                let b1 = input[read + 1];
                output[write] = encode_base64_value::<A>(b0 >> 2);
                output[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                output[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                if PAD {
                    output[write + 3] = b'=';
                }
            }
            _ => unreachable!(),
        }

        output
    }

    /// Encodes `input` into `output`, returning the number of bytes written.
    pub fn encode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        backend::encode_slice::<A, PAD>(input, output)
    }

    /// Encodes `input` into `output` and clears all bytes after the encoded
    /// prefix.
    ///
    /// If encoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 12];
    /// let written = STANDARD
    ///     .encode_slice_clear_tail(b"hello", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"aGVsbG8=");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn encode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, EncodeError> {
        let written = match self.encode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                output.fill(0);
                return Err(err);
            }
        };
        output[written..].fill(0);
        Ok(written)
    }

    /// Encodes `input` into a newly allocated byte vector.
    #[cfg(feature = "alloc")]
    pub fn encode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
        let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
        let mut output = alloc::vec![0; required];
        let written = self.encode_slice(input, &mut output)?;
        output.truncate(written);
        Ok(output)
    }

    /// Encodes `input` into a newly allocated UTF-8 string.
    ///
    /// Base64 output is ASCII by construction. This helper is available with
    /// the `alloc` feature and has the same encoding semantics as
    /// [`Self::encode_slice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
    ///
    /// assert_eq!(STANDARD.encode_string(b"hello").unwrap(), "aGVsbG8=");
    /// assert_eq!(URL_SAFE_NO_PAD.encode_string(b"\xfb\xff").unwrap(), "-_8");
    /// ```
    #[cfg(feature = "alloc")]
    pub fn encode_string(&self, input: &[u8]) -> Result<alloc::string::String, EncodeError> {
        let output = self.encode_vec(input)?;
        match alloc::string::String::from_utf8(output) {
            Ok(output) => Ok(output),
            Err(_) => unreachable!("base64 encoder produced non-UTF-8 output"),
        }
    }

    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0u8; 8];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        if input_len > buffer.len() {
            return Err(EncodeError::InputTooLarge {
                input_len,
                buffer_len: buffer.len(),
            });
        }

        let required = checked_encoded_len(input_len, PAD).ok_or(EncodeError::LengthOverflow)?;
        if buffer.len() < required {
            return Err(EncodeError::OutputTooSmall {
                required,
                available: buffer.len(),
            });
        }

        let mut read = input_len;
        let mut write = required;

        match input_len % 3 {
            0 => {}
            1 => {
                read -= 1;
                let b0 = buffer[read];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                    buffer[write + 2] = b'=';
                    buffer[write + 3] = b'=';
                } else {
                    write -= 2;
                    buffer[write] = encode_base64_value::<A>(b0 >> 2);
                    buffer[write + 1] = encode_base64_value::<A>((b0 & 0b0000_0011) << 4);
                }
            }
            2 => {
                read -= 2;
                let b0 = buffer[read];
                let b1 = buffer[read + 1];
                if PAD {
                    write -= 4;
                    buffer[write] = encode_base64_value::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                    buffer[write + 3] = b'=';
                } else {
                    write -= 3;
                    buffer[write] = encode_base64_value::<A>(b0 >> 2);
                    buffer[write + 1] =
                        encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
                    buffer[write + 2] = encode_base64_value::<A>((b1 & 0b0000_1111) << 2);
                }
            }
            _ => unreachable!(),
        }

        while read > 0 {
            read -= 3;
            write -= 4;
            let b0 = buffer[read];
            let b1 = buffer[read + 1];
            let b2 = buffer[read + 2];

            buffer[write] = encode_base64_value::<A>(b0 >> 2);
            buffer[write + 1] = encode_base64_value::<A>(((b0 & 0b0000_0011) << 4) | (b1 >> 4));
            buffer[write + 2] = encode_base64_value::<A>(((b1 & 0b0000_1111) << 2) | (b2 >> 6));
            buffer[write + 3] = encode_base64_value::<A>(b2 & 0b0011_1111);
        }

        debug_assert_eq!(write, 0);
        Ok(&mut buffer[..required])
    }

    /// Encodes the first `input_len` bytes of `buffer` in place and clears all
    /// bytes after the encoded prefix.
    ///
    /// If encoding fails because `input_len` is too large, the output buffer is
    /// too small, or the encoded length overflows `usize`, the entire buffer is
    /// cleared before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = [0xff; 12];
    /// buffer[..5].copy_from_slice(b"hello");
    /// let encoded = STANDARD.encode_in_place_clear_tail(&mut buffer, 5).unwrap();
    /// assert_eq!(encoded, b"aGVsbG8=");
    /// ```
    pub fn encode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
        input_len: usize,
    ) -> Result<&'a mut [u8], EncodeError> {
        let len = match self.encode_in_place(buffer, input_len) {
            Ok(encoded) => encoded.len(),
            Err(err) => {
                buffer.fill(0);
                return Err(err);
            }
        };
        buffer[len..].fill(0);
        Ok(&mut buffer[..len])
    }

    /// Decodes `input` into `output`, returning the number of bytes written.
    ///
    /// This is strict decoding. Whitespace, mixed alphabets, malformed padding,
    /// and trailing non-padding data are rejected.
    pub fn decode_slice(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        backend::decode_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` into `output` and clears all bytes after the decoded
    /// prefix.
    ///
    /// If decoding fails, the entire output buffer is cleared before the error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_clear_tail(b"aGk=", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice(input, output) {
            Ok(written) => written,
            Err(err) => {
                output.fill(0);
                return Err(err);
            }
        };
        output[written..].fill(0);
        Ok(written)
    }

    /// Decodes `input` using the explicit legacy whitespace profile.
    ///
    /// ASCII space, tab, carriage return, and line feed bytes are ignored.
    /// Alphabet selection, padding placement, trailing data after padding, and
    /// non-canonical trailing bits remain strict.
    pub fn decode_slice_legacy(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall {
                required,
                available: output.len(),
            });
        }
        decode_legacy_to_slice::<A, PAD>(input, output)
    }

    /// Decodes `input` using the explicit legacy whitespace profile and clears
    /// all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire output buffer is cleared
    /// before the error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut output = [0xff; 8];
    /// let written = STANDARD
    ///     .decode_slice_legacy_clear_tail(b" aG\r\nk= ", &mut output)
    ///     .unwrap();
    ///
    /// assert_eq!(&output[..written], b"hi");
    /// assert!(output[written..].iter().all(|byte| *byte == 0));
    /// ```
    pub fn decode_slice_legacy_clear_tail(
        &self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = match self.decode_slice_legacy(input, output) {
            Ok(written) => written,
            Err(err) => {
                output.fill(0);
                return Err(err);
            }
        };
        output[written..].fill(0);
        Ok(written)
    }

    /// Decodes `input` into a newly allocated byte vector.
    ///
    /// This is strict decoding with the same semantics as [`Self::decode_slice`].
    #[cfg(feature = "alloc")]
    pub fn decode_vec(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                output.fill(0);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes `input` into a newly allocated byte vector using the explicit
    /// legacy whitespace profile.
    #[cfg(feature = "alloc")]
    pub fn decode_vec_legacy(&self, input: &[u8]) -> Result<alloc::vec::Vec<u8>, DecodeError> {
        let required = validate_legacy_decode::<A, PAD>(input)?;
        let mut output = alloc::vec![0; required];
        let written = match self.decode_slice_legacy(input, &mut output) {
            Ok(written) => written,
            Err(err) => {
                output.fill(0);
                return Err(err);
            }
        };
        output.truncate(written);
        Ok(output)
    }

    /// Decodes the buffer in place and returns the decoded prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD_NO_PAD;
    ///
    /// let mut buffer = *b"Zm9vYmFy";
    /// let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"foobar");
    /// ```
    pub fn decode_in_place<'a>(&self, buffer: &'a mut [u8]) -> Result<&'a mut [u8], DecodeError> {
        let len = Self::decode_slice_to_start(buffer)?;
        Ok(&mut buffer[..len])
    }

    /// Decodes the buffer in place and clears all bytes after the decoded prefix.
    ///
    /// If decoding fails, the entire buffer is cleared before the error is
    /// returned. Use this variant when the encoded or partially decoded data is
    /// sensitive and the caller wants best-effort cleanup without adding a
    /// dependency.
    ///
    /// # Examples
    ///
    /// ```
    /// use base64_ng::STANDARD;
    ///
    /// let mut buffer = *b"aGk=";
    /// let decoded = STANDARD.decode_in_place_clear_tail(&mut buffer).unwrap();
    /// assert_eq!(decoded, b"hi");
    /// ```
    pub fn decode_in_place_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let len = match Self::decode_slice_to_start(buffer) {
            Ok(len) => len,
            Err(err) => {
                buffer.fill(0);
                return Err(err);
            }
        };
        buffer[len..].fill(0);
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile.
    ///
    /// Ignored whitespace is compacted out before decoding. If validation
    /// fails, the buffer contents are unspecified.
    pub fn decode_in_place_legacy<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        let _required = validate_legacy_decode::<A, PAD>(buffer)?;
        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }
        let len = Self::decode_slice_to_start(&mut buffer[..write])?;
        Ok(&mut buffer[..len])
    }

    /// Decodes `buffer` in place using the explicit legacy whitespace profile
    /// and clears all bytes after the decoded prefix.
    ///
    /// If validation or decoding fails, the entire buffer is cleared before the
    /// error is returned.
    pub fn decode_in_place_legacy_clear_tail<'a>(
        &self,
        buffer: &'a mut [u8],
    ) -> Result<&'a mut [u8], DecodeError> {
        if let Err(err) = validate_legacy_decode::<A, PAD>(buffer) {
            buffer.fill(0);
            return Err(err);
        }

        let mut write = 0;
        let mut read = 0;
        while read < buffer.len() {
            let byte = buffer[read];
            if !is_legacy_whitespace(byte) {
                buffer[write] = byte;
                write += 1;
            }
            read += 1;
        }

        let len = match Self::decode_slice_to_start(&mut buffer[..write]) {
            Ok(len) => len,
            Err(err) => {
                buffer.fill(0);
                return Err(err);
            }
        };
        buffer[len..].fill(0);
        Ok(&mut buffer[..len])
    }

    fn decode_slice_to_start(buffer: &mut [u8]) -> Result<usize, DecodeError> {
        let input_len = buffer.len();
        let mut read = 0;
        let mut write = 0;
        while read + 4 <= input_len {
            let chunk = [
                buffer[read],
                buffer[read + 1],
                buffer[read + 2],
                buffer[read + 3],
            ];
            let written = decode_chunk::<A, PAD>(&chunk, &mut buffer[write..])
                .map_err(|err| err.with_index_offset(read))?;
            read += 4;
            write += written;
            if written < 3 {
                if read != input_len {
                    return Err(DecodeError::InvalidPadding { index: read - 4 });
                }
                return Ok(write);
            }
        }

        let rem = input_len - read;
        if rem == 0 {
            return Ok(write);
        }
        if PAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut tail = [0u8; 3];
        tail[..rem].copy_from_slice(&buffer[read..input_len]);
        decode_tail_unpadded::<A>(&tail[..rem], &mut buffer[write..])
            .map_err(|err| err.with_index_offset(read))
            .map(|n| write + n)
    }
}

/// Encoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The encoded output length would overflow `usize`.
    LengthOverflow,
    /// The caller-provided input length exceeds the provided buffer.
    InputTooLarge {
        /// Requested input bytes.
        input_len: usize,
        /// Available buffer bytes.
        buffer_len: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LengthOverflow => f.write_str("base64 output length overflows usize"),
            Self::InputTooLarge {
                input_len,
                buffer_len,
            } => write!(
                f,
                "base64 input length {input_len} exceeds buffer length {buffer_len}"
            ),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Decoding error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The encoded input length is impossible for the selected padding policy.
    InvalidLength,
    /// A byte is not valid for the selected alphabet.
    InvalidByte {
        /// Byte index in the input.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// Padding is missing, misplaced, or non-canonical.
    InvalidPadding {
        /// Byte index where padding became invalid.
        index: usize,
    },
    /// The output buffer is too small.
    OutputTooSmall {
        /// Required output bytes.
        required: usize,
        /// Available output bytes.
        available: usize,
    },
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("invalid base64 input length"),
            Self::InvalidByte { index, byte } => {
                write!(f, "invalid base64 byte 0x{byte:02x} at index {index}")
            }
            Self::InvalidPadding { index } => write!(f, "invalid base64 padding at index {index}"),
            Self::OutputTooSmall {
                required,
                available,
            } => write!(
                f,
                "base64 decode output buffer too small: required {required}, available {available}"
            ),
        }
    }
}

impl DecodeError {
    fn with_index_offset(self, offset: usize) -> Self {
        match self {
            Self::InvalidByte { index, byte } => Self::InvalidByte {
                index: index + offset,
                byte,
            },
            Self::InvalidPadding { index } => Self::InvalidPadding {
                index: index + offset,
            },
            Self::InvalidLength | Self::OutputTooSmall { .. } => self,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

fn validate_legacy_decode<A: Alphabet, const PAD: bool>(
    input: &[u8],
) -> Result<usize, DecodeError> {
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut required = 0;
    let mut terminal_seen = false;

    for (index, byte) in input.iter().copied().enumerate() {
        if is_legacy_whitespace(byte) {
            continue;
        }
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written =
                validate_chunk::<A, PAD>(&chunk).map_err(|err| map_chunk_error(err, &indexes))?;
            required += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(required);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    validate_tail_unpadded::<A>(&chunk[..chunk_len])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))?;
    Ok(required + decoded_capacity(chunk_len))
}

fn decode_legacy_to_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    let mut chunk = [0u8; 4];
    let mut indexes = [0usize; 4];
    let mut chunk_len = 0;
    let mut write = 0;
    let mut terminal_seen = false;

    for (index, byte) in input.iter().copied().enumerate() {
        if is_legacy_whitespace(byte) {
            continue;
        }
        if terminal_seen {
            return Err(DecodeError::InvalidPadding { index });
        }

        chunk[chunk_len] = byte;
        indexes[chunk_len] = index;
        chunk_len += 1;

        if chunk_len == 4 {
            let written = decode_chunk::<A, PAD>(&chunk, &mut output[write..])
                .map_err(|err| map_chunk_error(err, &indexes))?;
            write += written;
            terminal_seen = written < 3;
            chunk_len = 0;
        }
    }

    if chunk_len == 0 {
        return Ok(write);
    }
    if PAD {
        return Err(DecodeError::InvalidLength);
    }

    decode_tail_unpadded::<A>(&chunk[..chunk_len], &mut output[write..])
        .map_err(|err| map_partial_chunk_error(err, &indexes, chunk_len))
        .map(|n| write + n)
}

#[inline]
const fn is_legacy_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn map_chunk_error(err: DecodeError, indexes: &[usize; 4]) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidLength | DecodeError::OutputTooSmall { .. } => err,
    }
}

fn map_partial_chunk_error(err: DecodeError, indexes: &[usize; 4], len: usize) -> DecodeError {
    match err {
        DecodeError::InvalidByte { index, byte } if index < len => DecodeError::InvalidByte {
            index: indexes[index],
            byte,
        },
        DecodeError::InvalidPadding { index } if index < len => DecodeError::InvalidPadding {
            index: indexes[index],
        },
        DecodeError::InvalidByte { .. }
        | DecodeError::InvalidPadding { .. }
        | DecodeError::InvalidLength
        | DecodeError::OutputTooSmall { .. } => err,
    }
}

fn decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read < input.len() {
        let written = decode_chunk::<A, true>(&input[read..read + 4], &mut output[write..])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }
    Ok(write)
}

#[cfg(feature = "alloc")]
fn validate_decode<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        validate_padded::<A>(input)
    } else {
        validate_unpadded::<A>(input)
    }
}

#[cfg(feature = "alloc")]
fn validate_padded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let required = decoded_len_padded(input)?;

    let mut read = 0;
    while read < input.len() {
        let written = validate_chunk::<A, true>(&input[read..read + 4])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        if written < 3 && read != input.len() {
            return Err(DecodeError::InvalidPadding { index: read - 4 });
        }
    }

    Ok(required)
}

#[cfg(feature = "alloc")]
fn validate_unpadded<A: Alphabet>(input: &[u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;

    let mut read = 0;
    while read + 4 <= input.len() {
        validate_chunk::<A, false>(&input[read..read + 4])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
    }
    validate_tail_unpadded::<A>(&input[read..]).map_err(|err| err.with_index_offset(read))?;

    Ok(required)
}

fn decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    let required = decoded_len_unpadded(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + 4 <= input.len() {
        let written = decode_chunk::<A, false>(&input[read..read + 4], &mut output[write..])
            .map_err(|err| err.with_index_offset(read))?;
        read += 4;
        write += written;
    }
    decode_tail_unpadded::<A>(&input[read..], &mut output[write..])
        .map_err(|err| err.with_index_offset(read))
        .map(|n| write + n)
}

fn decoded_len_padded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    let mut padding = 0;
    if input[input.len() - 1] == b'=' {
        padding += 1;
    }
    if input[input.len() - 2] == b'=' {
        padding += 1;
    }
    if padding == 0
        && let Some(index) = input.iter().position(|byte| *byte == b'=')
    {
        return Err(DecodeError::InvalidPadding { index });
    }
    if padding > 0 {
        let first_pad = input.len() - padding;
        if let Some(index) = input[..first_pad].iter().position(|byte| *byte == b'=') {
            return Err(DecodeError::InvalidPadding { index });
        }
    }
    Ok(input.len() / 4 * 3 - padding)
}

fn decoded_len_unpadded(input: &[u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }
    if let Some(index) = input.iter().position(|byte| *byte == b'=') {
        return Err(DecodeError::InvalidPadding { index });
    }
    Ok(decoded_capacity(input.len()))
}

fn validate_chunk<A: Alphabet, const PAD: bool>(input: &[u8]) -> Result<usize, DecodeError> {
    debug_assert_eq!(input.len(), 4);
    let _v0 = decode_byte::<A>(input[0], 0)?;
    let v1 = decode_byte::<A>(input[1], 1)?;

    match (input[2], input[3]) {
        (b'=', b'=') if PAD => {
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: input.iter().position(|byte| *byte == b'=').unwrap_or(0),
        }),
        _ => {
            decode_byte::<A>(input[2], 2)?;
            decode_byte::<A>(input[3], 3)?;
            Ok(3)
        }
    }
}

fn decode_chunk<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    debug_assert_eq!(input.len(), 4);
    let v0 = decode_byte::<A>(input[0], 0)?;
    let v1 = decode_byte::<A>(input[1], 1)?;

    match (input[2], input[3]) {
        (b'=', b'=') if PAD => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        (b'=', _) if PAD => Err(DecodeError::InvalidPadding { index: 2 }),
        (_, b'=') if PAD => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        (b'=', _) | (_, b'=') => Err(DecodeError::InvalidPadding {
            index: input.iter().position(|byte| *byte == b'=').unwrap_or(0),
        }),
        _ => {
            if output.len() < 3 {
                return Err(DecodeError::OutputTooSmall {
                    required: 3,
                    available: output.len(),
                });
            }
            let v2 = decode_byte::<A>(input[2], 2)?;
            let v3 = decode_byte::<A>(input[3], 3)?;
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            output[2] = (v2 << 6) | v3;
            Ok(3)
        }
    }
}

fn validate_tail_unpadded<A: Alphabet>(input: &[u8]) -> Result<(), DecodeError> {
    match input.len() {
        0 => Ok(()),
        2 => {
            decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            Ok(())
        }
        3 => {
            decode_byte::<A>(input[0], 0)?;
            decode_byte::<A>(input[1], 1)?;
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            Ok(())
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

fn decode_tail_unpadded<A: Alphabet>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    match input.len() {
        0 => Ok(0),
        2 => {
            if output.is_empty() {
                return Err(DecodeError::OutputTooSmall {
                    required: 1,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            if v1 & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidPadding { index: 1 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            Ok(1)
        }
        3 => {
            if output.len() < 2 {
                return Err(DecodeError::OutputTooSmall {
                    required: 2,
                    available: output.len(),
                });
            }
            let v0 = decode_byte::<A>(input[0], 0)?;
            let v1 = decode_byte::<A>(input[1], 1)?;
            let v2 = decode_byte::<A>(input[2], 2)?;
            if v2 & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidPadding { index: 2 });
            }
            output[0] = (v0 << 2) | (v1 >> 4);
            output[1] = (v1 << 4) | (v2 >> 2);
            Ok(2)
        }
        _ => Err(DecodeError::InvalidLength),
    }
}

fn decode_byte<A: Alphabet>(byte: u8, index: usize) -> Result<u8, DecodeError> {
    A::decode(byte).ok_or(DecodeError::InvalidByte { index, byte })
}

fn ct_decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.is_empty() {
        return Ok(0);
    }

    if PAD {
        ct_decode_padded::<A>(input, output)
    } else {
        ct_decode_unpadded::<A>(input, output)
    }
}

fn ct_decode_padded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let padding = ct_padding_len(input);
    let required = input.len() / 4 * 3 - padding;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read < input.len() {
        let is_last = read + 4 == input.len();
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];
        let b3 = input[read + 3];
        let (v0, valid0) = ct_decode_ascii_base64::<A>(b0);
        let (v1, valid1) = ct_decode_ascii_base64::<A>(b1);
        let (v2, valid2) = ct_decode_ascii_base64::<A>(b2);
        let (v3, valid3) = ct_decode_ascii_base64::<A>(b3);

        invalid_byte |= u8::from(!valid0);
        invalid_byte |= u8::from(!valid1);

        if is_last && padding == 2 {
            invalid_padding |= u8::from((v1 & 0b0000_1111) != 0);
            output[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        } else if is_last && padding == 1 {
            invalid_byte |= u8::from(!valid2);
            invalid_padding |= u8::from(b2 == b'=');
            invalid_padding |= u8::from((v2 & 0b0000_0011) != 0);
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        } else {
            invalid_byte |= u8::from(!valid2);
            invalid_byte |= u8::from(!valid3);
            invalid_padding |= u8::from(b2 == b'=');
            invalid_padding |= u8::from(b3 == b'=');
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            output[write + 2] = (v2 << 6) | v3;
            write += 3;
        }

        read += 4;
    }

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

fn ct_decode_unpadded<A: Alphabet>(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() % 4 == 1 {
        return Err(DecodeError::InvalidLength);
    }

    let required = decoded_capacity(input.len());
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut invalid_byte = 0u8;
    let mut invalid_padding = 0u8;
    let mut write = 0;
    let mut read = 0;

    while read + 4 <= input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];
        let b3 = input[read + 3];
        let (v0, valid0) = ct_decode_ascii_base64::<A>(b0);
        let (v1, valid1) = ct_decode_ascii_base64::<A>(b1);
        let (v2, valid2) = ct_decode_ascii_base64::<A>(b2);
        let (v3, valid3) = ct_decode_ascii_base64::<A>(b3);

        invalid_byte |= u8::from(!valid0);
        invalid_byte |= u8::from(!valid1);
        invalid_byte |= u8::from(!valid2);
        invalid_byte |= u8::from(!valid3);
        invalid_padding |= u8::from(b0 == b'=');
        invalid_padding |= u8::from(b1 == b'=');
        invalid_padding |= u8::from(b2 == b'=');
        invalid_padding |= u8::from(b3 == b'=');

        output[write] = (v0 << 2) | (v1 >> 4);
        output[write + 1] = (v1 << 4) | (v2 >> 2);
        output[write + 2] = (v2 << 6) | v3;
        read += 4;
        write += 3;
    }

    match input.len() - read {
        0 => {}
        2 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let (v0, valid0) = ct_decode_ascii_base64::<A>(b0);
            let (v1, valid1) = ct_decode_ascii_base64::<A>(b1);
            invalid_byte |= u8::from(!valid0);
            invalid_byte |= u8::from(!valid1);
            invalid_padding |= u8::from(b0 == b'=');
            invalid_padding |= u8::from(b1 == b'=');
            invalid_padding |= u8::from((v1 & 0b0000_1111) != 0);
            output[write] = (v0 << 2) | (v1 >> 4);
            write += 1;
        }
        3 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            let b2 = input[read + 2];
            let (v0, valid0) = ct_decode_ascii_base64::<A>(b0);
            let (v1, valid1) = ct_decode_ascii_base64::<A>(b1);
            let (v2, valid2) = ct_decode_ascii_base64::<A>(b2);
            invalid_byte |= u8::from(!valid0);
            invalid_byte |= u8::from(!valid1);
            invalid_byte |= u8::from(!valid2);
            invalid_padding |= u8::from(b0 == b'=');
            invalid_padding |= u8::from(b1 == b'=');
            invalid_padding |= u8::from(b2 == b'=');
            invalid_padding |= u8::from((v2 & 0b0000_0011) != 0);
            output[write] = (v0 << 2) | (v1 >> 4);
            output[write + 1] = (v1 << 4) | (v2 >> 2);
            write += 2;
        }
        _ => return Err(DecodeError::InvalidLength),
    }

    report_ct_error(invalid_byte, invalid_padding)?;
    Ok(write)
}

#[inline]
fn ct_decode_ascii_base64<A: Alphabet>(byte: u8) -> (u8, bool) {
    let upper = mask_if(byte.wrapping_sub(b'A') <= b'Z' - b'A');
    let lower = mask_if(byte.wrapping_sub(b'a') <= b'z' - b'a');
    let digit = mask_if(byte.wrapping_sub(b'0') <= b'9' - b'0');
    let value_62 = mask_if(byte == A::ENCODE[62]);
    let value_63 = mask_if(byte == A::ENCODE[63]);
    let valid = upper | lower | digit | value_62 | value_63;

    let decoded = (byte.wrapping_sub(b'A') & upper)
        | (byte.wrapping_sub(b'a').wrapping_add(26) & lower)
        | (byte.wrapping_sub(b'0').wrapping_add(52) & digit)
        | (0x3e & value_62)
        | (0x3f & value_63);

    (decoded, valid != 0)
}

fn ct_padding_len(input: &[u8]) -> usize {
    let last = input[input.len() - 1];
    let before_last = input[input.len() - 2];
    usize::from(mask_if(last == b'=') & 1) + usize::from(mask_if(before_last == b'=') & 1)
}

fn report_ct_error(invalid_byte: u8, invalid_padding: u8) -> Result<(), DecodeError> {
    if invalid_padding != 0 {
        Err(DecodeError::InvalidPadding { index: 0 })
    } else if invalid_byte != 0 {
        Err(DecodeError::InvalidByte { index: 0, byte: 0 })
    } else {
        Ok(())
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{STANDARD, checked_encoded_len, decoded_capacity};

    #[kani::proof]
    fn checked_encoded_len_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let padded = kani::any::<bool>();
        let encoded = checked_encoded_len(len, padded).expect("u8 input length cannot overflow");

        assert!(encoded >= len);
        assert!(encoded <= len / 3 * 4 + 4);
    }

    #[kani::proof]
    fn decoded_capacity_is_bounded_for_small_inputs() {
        let len = usize::from(kani::any::<u8>());
        let capacity = decoded_capacity(len);

        assert!(capacity <= len / 4 * 3 + 2);
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_in_place_decode_returns_prefix_within_buffer() {
        let mut buffer = kani::any::<[u8; 8]>();
        let result = STANDARD.decode_in_place(&mut buffer);

        if let Ok(decoded) = result {
            assert!(decoded.len() <= 8);
        }
    }

    #[kani::proof]
    #[kani::unwind(3)]
    fn standard_clear_tail_decode_clears_buffer_on_error() {
        let mut buffer = kani::any::<[u8; 4]>();
        let result = STANDARD.decode_in_place_clear_tail(&mut buffer);

        if result.is_err() {
            assert!(buffer.iter().all(|byte| *byte == 0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_pattern(output: &mut [u8], seed: usize) {
        for (index, byte) in output.iter_mut().enumerate() {
            let value = (index * 73 + seed * 19) % 256;
            *byte = u8::try_from(value).unwrap();
        }
    }

    fn assert_encode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 256];
        let mut scalar = [0xaa; 256];

        let dispatched_result = engine.encode_slice(input, &mut dispatched);
        let scalar_result = backend::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);
        }

        let required = checked_encoded_len(input.len(), PAD).unwrap();
        if required > 0 {
            let mut dispatched_short = [0x55; 256];
            let mut scalar_short = [0xaa; 256];
            let available = required - 1;

            assert_eq!(
                engine.encode_slice(input, &mut dispatched_short[..available]),
                backend::scalar_reference_encode_slice::<A, PAD>(
                    input,
                    &mut scalar_short[..available],
                )
            );
        }
    }

    fn assert_decode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        let engine = Engine::<A, PAD>::new();
        let mut dispatched = [0x55; 128];
        let mut scalar = [0xaa; 128];

        let dispatched_result = engine.decode_slice(input, &mut dispatched);
        let scalar_result = backend::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar);

        assert_eq!(dispatched_result, scalar_result);
        if let Ok(written) = dispatched_result {
            assert_eq!(&dispatched[..written], &scalar[..written]);

            if written > 0 {
                let mut dispatched_short = [0x55; 128];
                let mut scalar_short = [0xaa; 128];
                let available = written - 1;

                assert_eq!(
                    engine.decode_slice(input, &mut dispatched_short[..available]),
                    backend::scalar_reference_decode_slice::<A, PAD>(
                        input,
                        &mut scalar_short[..available],
                    )
                );
            }
        }
    }

    fn assert_backend_round_trip_matches_scalar<A, const PAD: bool>(input: &[u8])
    where
        A: Alphabet,
    {
        assert_encode_backend_matches_scalar::<A, PAD>(input);

        let mut encoded = [0; 256];
        let encoded_len =
            backend::scalar_reference_encode_slice::<A, PAD>(input, &mut encoded).unwrap();
        assert_decode_backend_matches_scalar::<A, PAD>(&encoded[..encoded_len]);
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_canonical_inputs() {
        let mut input = [0; 128];

        for input_len in 0..=input.len() {
            fill_pattern(&mut input[..input_len], input_len);
            let input = &input[..input_len];

            assert_backend_round_trip_matches_scalar::<Standard, true>(input);
            assert_backend_round_trip_matches_scalar::<Standard, false>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, true>(input);
            assert_backend_round_trip_matches_scalar::<UrlSafe, false>(input);
        }
    }

    #[test]
    fn backend_dispatch_matches_scalar_reference_for_malformed_inputs() {
        for input in [
            &b"Z"[..],
            b"====",
            b"AA=A",
            b"Zh==",
            b"Zm9=",
            b"Zm9v$g==",
            b"Zm9vZh==",
        ] {
            assert_decode_backend_matches_scalar::<Standard, true>(input);
        }

        for input in [&b"Z"[..], b"AA=A", b"Zh", b"Zm9", b"Zm9vYg$"] {
            assert_decode_backend_matches_scalar::<Standard, false>(input);
        }

        assert_decode_backend_matches_scalar::<UrlSafe, true>(b"AA+A");
        assert_decode_backend_matches_scalar::<UrlSafe, false>(b"AA/A");
        assert_decode_backend_matches_scalar::<Standard, true>(b"AA-A");
        assert_decode_backend_matches_scalar::<Standard, false>(b"AA_A");
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_dispatch_scaffold_keeps_scalar_active() {
        assert_eq!(simd::active_backend(), simd::ActiveBackend::Scalar);
        let _candidate = simd::detected_candidate();
    }

    #[test]
    fn encodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"f"[..], &b"Zg=="[..]),
            (&b"fo"[..], &b"Zm8="[..]),
            (&b"foo"[..], &b"Zm9v"[..]),
            (&b"foob"[..], &b"Zm9vYg=="[..]),
            (&b"fooba"[..], &b"Zm9vYmE="[..]),
            (&b"foobar"[..], &b"Zm9vYmFy"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.encode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn decodes_standard_vectors() {
        let vectors = [
            (&b""[..], &b""[..]),
            (&b"Zg=="[..], &b"f"[..]),
            (&b"Zm8="[..], &b"fo"[..]),
            (&b"Zm9v"[..], &b"foo"[..]),
            (&b"Zm9vYg=="[..], &b"foob"[..]),
            (&b"Zm9vYmE="[..], &b"fooba"[..]),
            (&b"Zm9vYmFy"[..], &b"foobar"[..]),
        ];
        for (input, expected) in vectors {
            let mut output = [0u8; 16];
            let written = STANDARD.decode_slice(input, &mut output).unwrap();
            assert_eq!(&output[..written], expected);
        }
    }

    #[test]
    fn supports_unpadded_url_safe() {
        let mut encoded = [0u8; 16];
        let written = URL_SAFE_NO_PAD
            .encode_slice(b"\xfb\xff", &mut encoded)
            .unwrap();
        assert_eq!(&encoded[..written], b"-_8");

        let mut decoded = [0u8; 2];
        let written = URL_SAFE_NO_PAD
            .decode_slice(&encoded[..written], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..written], b"\xfb\xff");
    }

    #[test]
    fn decodes_in_place() {
        let mut buffer = *b"Zm9vYmFy";
        let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
        assert_eq!(decoded, b"foobar");
    }

    #[test]
    fn rejects_non_canonical_padding_bits() {
        let mut output = [0u8; 4];
        assert_eq!(
            STANDARD.decode_slice(b"Zh==", &mut output),
            Err(DecodeError::InvalidPadding { index: 1 })
        );
        assert_eq!(
            STANDARD.decode_slice(b"Zm9=", &mut output),
            Err(DecodeError::InvalidPadding { index: 2 })
        );
    }
}
