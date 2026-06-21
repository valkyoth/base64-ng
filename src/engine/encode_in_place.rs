use crate::{Alphabet, EncodeError, Engine, encode_backend, wipe_bytes, wipe_tail};

impl<A, const PAD: bool> Engine<A, PAD>
where
    A: Alphabet,
{
    /// Encodes the first `input_len` bytes of `buffer` in place.
    ///
    /// The buffer must have enough spare capacity for the encoded output. The
    /// implementation writes from right to left, so unread input bytes are not
    /// overwritten before they are encoded.
    ///
    /// # Panics
    ///
    /// Panics only if an internal right-to-left encode invariant is violated.
    /// This indicates a bug in `base64-ng`; valid or malformed caller input is
    /// reported through [`EncodeError`] instead.
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
        let len = encode_backend::encode_in_place::<A, PAD>(buffer, input_len)?;
        Ok(&mut buffer[..len])
    }

    /// Encodes the first `input_len` bytes of `buffer` in place and clears all
    /// bytes after the encoded prefix.
    ///
    /// If encoding fails because `input_len` is too large, the output buffer is
    /// too small, or the encoded length overflows `usize`, the entire buffer is
    /// cleared before the error is returned. This includes the original
    /// plaintext in `buffer[..input_len]`; keep another copy before calling
    /// this method if recovery of the original input is required after an
    /// error.
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
                wipe_bytes(buffer);
                return Err(err);
            }
        };
        wipe_tail(buffer, len);
        Ok(&mut buffer[..len])
    }
}
