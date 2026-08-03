use alloc::{string::String, vec::Vec};
use core::fmt;

use base64_ng::clear_bytes;
use serde::de::Visitor;

use crate::adapter::{AdapterError, DecodeInput};

pub(crate) struct SecretInputVisitor<T>(pub(crate) T);

impl<'de, T: DecodeInput> Visitor<'de> for SecretInputVisitor<T> {
    type Value = T::Output;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("secret Base64 text as a string or byte string")
    }

    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.0.decode(value.as_bytes()).map_err(E::custom)
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.0.decode(value.as_bytes()).map_err(E::custom)
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        decode_owned_secret(self.0, value.into_bytes()).map_err(E::custom)
    }

    fn visit_borrowed_bytes<E: serde::de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        self.0.decode(value).map_err(E::custom)
    }

    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.0.decode(value).map_err(E::custom)
    }

    fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        decode_owned_secret(self.0, value).map_err(E::custom)
    }
}

fn decode_owned_secret<T: DecodeInput>(
    decoder: T,
    mut input: Vec<u8>,
) -> Result<T::Output, AdapterError> {
    let input = WipingOwnedInput::new(&mut input);
    decoder.decode(input.as_slice())
}

pub(crate) struct WipingOwnedInput<'a> {
    bytes: &'a mut Vec<u8>,
}

impl<'a> WipingOwnedInput<'a> {
    pub(crate) fn new(bytes: &'a mut Vec<u8>) -> Self {
        Self { bytes }
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        self.bytes
    }
}

impl Drop for WipingOwnedInput<'_> {
    fn drop(&mut self) {
        // Initializing existing spare capacity cannot allocate and lets the
        // reviewed cleanup primitive cover the complete owned allocation.
        self.bytes.resize(self.bytes.capacity(), 0);
        clear_bytes(self.bytes);
    }
}
