use crate::{Alphabet, v2};
use std::io;

pub(super) struct EncoderDriver {
    state: Option<v2::EncoderState>,
}

impl EncoderDriver {
    pub(super) const fn new<A: Alphabet, const PAD: bool>() -> Self {
        let state = match v2::ValidatedAlphabet::new(A::ENCODE) {
            Ok(alphabet) => {
                let codec = v2::specifications::runtime_codec(
                    alphabet,
                    if PAD {
                        v2::EncodePadding::Padded
                    } else {
                        v2::EncodePadding::Unpadded
                    },
                    if PAD {
                        v2::DecodePadding::RequireCanonical
                    } else {
                        v2::DecodePadding::Forbid
                    },
                    v2::TrailingBits::RequireCanonical,
                );
                Some(v2::EncoderState::new(
                    codec.specification().const_settings(),
                ))
            }
            Err(_) => None,
        };
        Self { state }
    }

    pub(super) const fn pending_input_len(&self) -> usize {
        match &self.state {
            Some(state) => state.buffered_input_len(),
            None => 0,
        }
    }

    pub(super) fn update(&mut self, input: &[u8], output: &mut [u8]) -> io::Result<v2::Step> {
        self.state_mut()?
            .update(input, output)
            .map_err(operation_error)
    }

    pub(super) fn finish(&mut self, output: &mut [u8]) -> io::Result<v2::Step> {
        self.state_mut()?.finish(output).map_err(operation_error)
    }

    pub(super) fn wipe(&mut self) {
        if let Some(state) = &mut self.state {
            state.wipe();
        }
    }

    fn state_mut(&mut self) -> io::Result<&mut v2::EncoderState> {
        self.state.as_mut().ok_or_else(invalid_alphabet_error)
    }
}

pub(super) struct DecoderDriver {
    state: Option<v2::DecoderState>,
}

impl DecoderDriver {
    pub(super) const fn new<A: Alphabet, const PAD: bool>() -> Self {
        let state = match v2::ValidatedAlphabet::new(A::ENCODE) {
            Ok(alphabet) => {
                let codec = v2::specifications::runtime_codec(
                    alphabet,
                    if PAD {
                        v2::EncodePadding::Padded
                    } else {
                        v2::EncodePadding::Unpadded
                    },
                    if PAD {
                        v2::DecodePadding::RequireCanonical
                    } else {
                        v2::DecodePadding::Forbid
                    },
                    v2::TrailingBits::RequireCanonical,
                );
                Some(if PAD {
                    v2::DecoderState::new_padded(codec.specification().const_settings())
                } else {
                    v2::DecoderState::new_unpadded(codec.specification().const_settings())
                })
            }
            Err(_) => None,
        };
        Self { state }
    }

    pub(super) const fn pending_input_len(&self) -> usize {
        match &self.state {
            Some(state) => state.buffered_input_len(),
            None => 0,
        }
    }

    pub(super) const fn has_terminal_padding(&self) -> bool {
        match &self.state {
            Some(state) => state.has_terminal_padding(),
            None => false,
        }
    }

    pub(super) fn update(&mut self, input: &[u8], output: &mut [u8]) -> io::Result<v2::Step> {
        self.state_mut()?
            .update(input, output)
            .map_err(operation_error)
    }

    pub(super) fn finish(&mut self, output: &mut [u8]) -> io::Result<v2::Step> {
        self.state_mut()?.finish(output).map_err(operation_error)
    }

    pub(super) fn wipe(&mut self) {
        if let Some(state) = &mut self.state {
            state.wipe();
        }
    }

    fn state_mut(&mut self) -> io::Result<&mut v2::DecoderState> {
        self.state.as_mut().ok_or_else(invalid_alphabet_error)
    }
}

fn invalid_alphabet_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "base64 stream engine contains an invalid alphabet",
    )
}

fn operation_error(error: v2::OperationError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
