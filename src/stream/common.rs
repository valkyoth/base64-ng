use crate::{DecodeError, EncodeError};
use std::io;

pub(super) fn encode_error_to_io(err: EncodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, err)
}

pub(super) fn decode_error_to_io(err: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, err)
}

pub(super) fn trailing_input_after_padding_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 decoder received trailing input after padding",
    )
}

pub(super) fn stream_decoder_failed_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 stream decoder is failed after malformed input",
    )
}

pub(super) fn stream_encoder_failed_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 stream encoder is failed after internal error",
    )
}

pub(super) const fn redacted_inner_state(present: bool) -> &'static str {
    if present { "<present>" } else { "<taken>" }
}
