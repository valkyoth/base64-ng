//! Deliberately simple, test-only RFC 4648 Base64 oracle.
//!
//! This module must not use production alphabets, validators, length helpers,
//! state machines, or backends.

extern crate std;

use std::vec::Vec;

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Profile {
    StandardPadded,
    StandardUnpadded,
    UrlSafePadded,
    UrlSafeUnpadded,
}

impl Profile {
    fn alphabet(self) -> &'static [u8; 64] {
        match self {
            Self::StandardPadded | Self::StandardUnpadded => STANDARD,
            Self::UrlSafePadded | Self::UrlSafeUnpadded => URL_SAFE,
        }
    }

    fn padded(self) -> bool {
        matches!(self, Self::StandardPadded | Self::UrlSafePadded)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ErrorClass {
    Length,
    Byte,
    Padding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DecodeFailure {
    pub(super) class: ErrorClass,
    pub(super) offset: Option<usize>,
}

pub(super) fn encode(profile: Profile, input: &[u8]) -> Vec<u8> {
    let alphabet = profile.alphabet();
    let mut output = Vec::with_capacity(input.len().div_ceil(3) * 4);
    let mut chunks = input.chunks_exact(3);

    for chunk in &mut chunks {
        output.push(alphabet[usize::from(chunk[0] >> 2)]);
        output.push(alphabet[usize::from(((chunk[0] & 3) << 4) | (chunk[1] >> 4))]);
        output.push(alphabet[usize::from(((chunk[1] & 15) << 2) | (chunk[2] >> 6))]);
        output.push(alphabet[usize::from(chunk[2] & 63)]);
    }

    match chunks.remainder() {
        [] => {}
        [first] => {
            output.push(alphabet[usize::from(first >> 2)]);
            output.push(alphabet[usize::from((first & 3) << 4)]);
            if profile.padded() {
                output.extend_from_slice(b"==");
            }
        }
        [first, second] => {
            output.push(alphabet[usize::from(first >> 2)]);
            output.push(alphabet[usize::from(((first & 3) << 4) | (second >> 4))]);
            output.push(alphabet[usize::from((second & 15) << 2)]);
            if profile.padded() {
                output.push(b'=');
            }
        }
        _ => unreachable!("chunks_exact remainder is shorter than three bytes"),
    }

    output
}

pub(super) fn decode(profile: Profile, input: &[u8]) -> Result<Vec<u8>, DecodeFailure> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if profile.padded() && !input.len().is_multiple_of(4) {
        return Err(failure(ErrorClass::Length, None));
    }
    if !profile.padded() && input.len() % 4 == 1 {
        return Err(failure(ErrorClass::Length, None));
    }

    let mut output = Vec::with_capacity((input.len() / 4) * 3 + 2);
    let full_quads = input.len() / 4;

    for quad_index in 0..full_quads {
        let offset = quad_index * 4;
        let quad = &input[offset..offset + 4];
        let final_quad = offset + 4 == input.len();
        decode_quad(profile, quad, offset, final_quad, &mut output)?;
    }

    if !profile.padded() {
        let tail_offset = full_quads * 4;
        decode_unpadded_tail(profile, &input[tail_offset..], tail_offset, &mut output)?;
    }

    Ok(output)
}

fn decode_quad(
    profile: Profile,
    quad: &[u8],
    offset: usize,
    final_quad: bool,
    output: &mut Vec<u8>,
) -> Result<(), DecodeFailure> {
    let first = value(profile, quad[0], offset)?;
    let second = value(profile, quad[1], offset + 1)?;

    match (quad[2], quad[3]) {
        (b'=', b'=') if profile.padded() && final_quad => {
            if second & 15 != 0 {
                return Err(failure(ErrorClass::Padding, Some(offset + 1)));
            }
            output.push((first << 2) | (second >> 4));
        }
        (b'=', _) if profile.padded() => {
            return Err(failure(ErrorClass::Padding, Some(offset + 2)));
        }
        (third, b'=') if profile.padded() && final_quad => {
            let third = value(profile, third, offset + 2)?;
            if third & 3 != 0 {
                return Err(failure(ErrorClass::Padding, Some(offset + 2)));
            }
            output.push((first << 2) | (second >> 4));
            output.push((second << 4) | (third >> 2));
        }
        (_, b'=') if profile.padded() => {
            return Err(failure(ErrorClass::Padding, Some(offset + 3)));
        }
        (b'=', _) | (_, b'=') => {
            let padding = if quad[2] == b'=' {
                offset + 2
            } else {
                offset + 3
            };
            return Err(failure(ErrorClass::Padding, Some(padding)));
        }
        (third, fourth) => {
            let third = value(profile, third, offset + 2)?;
            let fourth = value(profile, fourth, offset + 3)?;
            output.push((first << 2) | (second >> 4));
            output.push((second << 4) | (third >> 2));
            output.push((third << 6) | fourth);
        }
    }

    Ok(())
}

fn decode_unpadded_tail(
    profile: Profile,
    tail: &[u8],
    offset: usize,
    output: &mut Vec<u8>,
) -> Result<(), DecodeFailure> {
    match tail {
        [] => Ok(()),
        [_] => Err(failure(ErrorClass::Length, None)),
        [first, second] => {
            let first = value(profile, *first, offset)?;
            let second = value(profile, *second, offset + 1)?;
            if second & 15 != 0 {
                return Err(failure(ErrorClass::Padding, Some(offset + 1)));
            }
            output.push((first << 2) | (second >> 4));
            Ok(())
        }
        [first, second, third] => {
            let first = value(profile, *first, offset)?;
            let second = value(profile, *second, offset + 1)?;
            let third = value(profile, *third, offset + 2)?;
            if third & 3 != 0 {
                return Err(failure(ErrorClass::Padding, Some(offset + 2)));
            }
            output.push((first << 2) | (second >> 4));
            output.push((second << 4) | (third >> 2));
            Ok(())
        }
        _ => unreachable!("tail is shorter than four bytes"),
    }
}

fn value(profile: Profile, byte: u8, offset: usize) -> Result<u8, DecodeFailure> {
    for (candidate, alphabet_byte) in profile.alphabet().iter().copied().enumerate() {
        if byte == alphabet_byte {
            return Ok(u8::try_from(candidate).expect("Base64 alphabet index fits u8"));
        }
    }
    if byte == b'=' {
        Err(failure(ErrorClass::Padding, Some(offset)))
    } else {
        Err(failure(ErrorClass::Byte, Some(offset)))
    }
}

const fn failure(class: ErrorClass, offset: Option<usize>) -> DecodeFailure {
    DecodeFailure { class, offset }
}
