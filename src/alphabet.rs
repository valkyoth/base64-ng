//! Base64 alphabets and custom alphabet validation.

use crate::{ct_mask_eq_u8, ct_mask_lt_u8};

/// Alphabet validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlphabetError {
    /// The alphabet contains a non-visible-ASCII byte.
    InvalidByte {
        /// Byte index in the alphabet table.
        index: usize,
        /// Invalid byte value.
        byte: u8,
    },
    /// The alphabet contains the padding byte `=`.
    PaddingByte {
        /// Byte index in the alphabet table.
        index: usize,
    },
    /// The alphabet maps more than one value to the same byte.
    DuplicateByte {
        /// First byte index.
        first: usize,
        /// Second byte index.
        second: usize,
        /// Duplicated byte value.
        byte: u8,
    },
}

impl core::fmt::Display for AlphabetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidByte { index, byte } => {
                write!(
                    f,
                    "invalid base64 alphabet byte 0x{byte:02x} at index {index}"
                )
            }
            Self::PaddingByte { index } => {
                write!(f, "base64 alphabet contains padding byte at index {index}")
            }
            Self::DuplicateByte {
                first,
                second,
                byte,
            } => write!(
                f,
                "base64 alphabet byte 0x{byte:02x} is duplicated at indexes {first} and {second}"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AlphabetError {}

/// Defines a custom [`Alphabet`] from a 64-byte string literal.
///
/// The generated alphabet is validated at compile time with
/// [`validate_alphabet`]. Invalid, duplicate, or padding bytes fail the build
/// instead of creating a malformed runtime profile.
///
/// The generated implementation uses the conservative default
/// [`Alphabet::encode`] behavior: every emitted Base64 byte performs a fixed
/// 64-entry scan to avoid secret-indexed table lookups. Built-in alphabets use
/// optimized arithmetic mappers.
///
/// The generated [`Alphabet::decode`] implementation delegates to
/// [`decode_alphabet_byte`]. The constant-time-oriented [`ct`](crate::ct)
/// module scans the generated `ENCODE` table directly and does not call the
/// generated `decode` method.
///
/// # Examples
///
/// ```
/// base64_ng::define_alphabet! {
///     struct DotSlash = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
/// }
///
/// let engine = base64_ng::Engine::<DotSlash, false>::new();
/// let mut encoded = [0u8; 4];
/// let written = engine.encode_slice(&[0xff, 0xff, 0xff], &mut encoded).unwrap();
/// assert_eq!(&encoded[..written], b"9999");
/// ```
///
/// Invalid alphabets fail during compilation:
///
/// ```compile_fail
/// base64_ng::define_alphabet! {
///     struct Bad = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
/// }
/// ```
#[macro_export]
macro_rules! define_alphabet {
    ($(#[$meta:meta])* $vis:vis struct $name:ident = $encode:expr;) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        $vis struct $name;

        impl $crate::Alphabet for $name {
            const ENCODE: [u8; 64] = *$encode;

            #[inline]
            fn decode(byte: u8) -> Option<u8> {
                $crate::decode_alphabet_byte(byte, &Self::ENCODE)
            }
        }

        const _: [(); 1] = [(); match $crate::validate_alphabet(
            &<$name as $crate::Alphabet>::ENCODE,
        ) {
            Ok(()) => 1,
            Err(_) => 0,
        }];
    };
}

/// Validates a 64-byte Base64 alphabet table.
///
/// A valid alphabet must contain exactly 64 unique visible ASCII bytes and must
/// not contain the padding byte `=`.
///
/// # Examples
///
/// ```
/// use base64_ng::{Alphabet, Standard, validate_alphabet};
///
/// validate_alphabet(&Standard::ENCODE).unwrap();
/// ```
pub const fn validate_alphabet(encode: &[u8; 64]) -> Result<(), AlphabetError> {
    let mut index = 0;
    while index < encode.len() {
        let byte = encode[index];
        if !is_visible_ascii(byte) {
            return Err(AlphabetError::InvalidByte { index, byte });
        }
        if byte == b'=' {
            return Err(AlphabetError::PaddingByte { index });
        }

        let mut duplicate = index + 1;
        while duplicate < encode.len() {
            if encode[duplicate] == byte {
                return Err(AlphabetError::DuplicateByte {
                    first: index,
                    second: duplicate,
                    byte,
                });
            }
            duplicate += 1;
        }

        index += 1;
    }

    Ok(())
}

/// Decodes one byte by scanning a caller-provided alphabet table.
///
/// This helper is intended for custom [`Alphabet`] implementations. Validate
/// the table with [`validate_alphabet`] before trusting the alphabet in a
/// protocol or public API. The scan always visits all 64 entries before
/// returning so the match position does not create an early-return timing
/// signal in the source-level implementation.
///
/// # Security
///
/// This helper is part of the normal strict decoder path, not the
/// constant-time-oriented [`ct`](crate::ct) module. It is a `const fn` so it
/// does not use the optimizer barriers, volatile accumulator reads, or
/// generated-code evidence hooks used by the private `ct` scanner. Do not rely
/// on this helper for military or cryptographic constant-time guarantees under
/// LTO or future compiler rewrites. For secret-bearing custom alphabets, use
/// [`Engine::ct_decoder`](crate::Engine::ct_decoder) or the [`ct`](crate::ct)
/// module, which scans [`Alphabet::ENCODE`] directly and does not call
/// [`Alphabet::decode`].
///
/// # Examples
///
/// ```
/// use base64_ng::{Alphabet, decode_alphabet_byte};
///
/// struct DotSlash;
///
/// impl Alphabet for DotSlash {
///     const ENCODE: [u8; 64] =
///         *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
///
///     fn decode(byte: u8) -> Option<u8> {
///         decode_alphabet_byte(byte, &Self::ENCODE)
///     }
/// }
///
/// assert_eq!(DotSlash::decode(b'.'), Some(0));
/// assert_eq!(DotSlash::decode(b'9'), Some(63));
/// ```
#[must_use]
pub const fn decode_alphabet_byte(byte: u8, encode: &[u8; 64]) -> Option<u8> {
    let mut index = 0;
    let mut candidate = 0;
    let mut decoded = 0;
    let mut valid = 0;
    while index < encode.len() {
        let matches = ct_mask_eq_u8(byte, encode[index]);
        decoded |= candidate & matches;
        valid |= matches;
        index += 1;
        candidate += 1;
    }

    if valid == 0 { None } else { Some(decoded) }
}

/// A Base64 alphabet.
///
/// # Security
///
/// The default [`Alphabet::encode`] implementation is constant-time-oriented:
/// it scans all 64 alphabet entries instead of using `ENCODE[value as usize]`.
/// Direct callers that override `encode` with a table lookup make those direct
/// calls timing-sensitive with respect to the selected 6-bit value. Public
/// [`Engine`](crate::Engine) encoding does not call this overridable method:
/// [`Alphabet::ENCODE`] is its sole output definition for const, scalar, SIMD,
/// wrapped, and in-place surfaces.
///
/// The normal strict decode path calls [`Alphabet::decode`] and is not a
/// constant-time decoder. The [`ct`](crate::ct) module does not call
/// [`Alphabet::decode`]; it scans [`Alphabet::ENCODE`] directly with its own
/// fixed 64-entry mapper. A custom non-constant-time `decode` implementation
/// therefore affects normal strict decode diagnostics and timing, but not the
/// `ct` module's symbol-mapping loop.
pub trait Alphabet {
    /// Encoding table indexed by 6-bit values.
    const ENCODE: [u8; 64];

    /// Encode one 6-bit value into an alphabet byte.
    ///
    /// The default implementation scans the alphabet table instead of using a
    /// secret-indexed table lookup. Built-in alphabets override this with the
    /// branch-minimized ASCII arithmetic mapper. Custom alphabets that keep the
    /// default method prioritize timing posture over throughput for direct
    /// calls. This method is retained as a public low-level mapping helper for
    /// API compatibility; [`Engine`](crate::Engine) uses [`Self::ENCODE`]
    /// directly and is unaffected by overrides.
    #[must_use]
    fn encode(value: u8) -> u8 {
        encode_alphabet_value(value, &Self::ENCODE)
    }

    /// Decode one byte into a 6-bit value.
    ///
    /// Implementations that want conservative custom-alphabet timing posture
    /// should delegate to [`decode_alphabet_byte`], which scans all 64 entries
    /// before returning. The `ct` module ignores this method and scans
    /// [`Self::ENCODE`] directly.
    fn decode(byte: u8) -> Option<u8>;
}

const fn is_visible_ascii(byte: u8) -> bool {
    byte >= 0x21 && byte <= 0x7e
}

/// The RFC 4648 standard Base64 alphabet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Standard;

impl Alphabet for Standard {
    const ENCODE: [u8; 64] = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    #[inline]
    fn encode(value: u8) -> u8 {
        encode_ascii_base64(value, Self::ENCODE[62], Self::ENCODE[63])
    }

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
    fn encode(value: u8) -> u8 {
        encode_ascii_base64(value, Self::ENCODE[62], Self::ENCODE[63])
    }

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_ascii_base64(byte, Self::ENCODE[62], Self::ENCODE[63])
    }
}

/// The bcrypt Base64 alphabet.
///
/// This alphabet is commonly used by bcrypt hash strings. It is provided as an
/// alphabet/profile building block; `base64-ng` does not parse or verify full
/// bcrypt password-hash records.
///
/// # Security
///
/// The strict [`Alphabet::decode`] implementation delegates to
/// [`decode_alphabet_byte`]. That helper scans the full alphabet, but it is a
/// `const fn` and does not use the additional optimizer barriers used by the
/// [`ct`](crate::ct) module. Do not use strict `Engine<Bcrypt, _>` decode as a
/// token, key, or password-hash verifier. Use [`crate::ct::CtEngine`] with this
/// alphabet for secret-bearing comparison workflows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Bcrypt;

impl Alphabet for Bcrypt {
    const ENCODE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

/// The Unix `crypt(3)` Base64 alphabet.
///
/// This alphabet is provided as an explicit legacy interoperability profile.
/// `base64-ng` does not parse or verify complete password-hash records.
///
/// # Security
///
/// The strict [`Alphabet::decode`] implementation delegates to
/// [`decode_alphabet_byte`]. That helper scans the full alphabet, but it is a
/// `const fn` and does not use the additional optimizer barriers used by the
/// [`ct`](crate::ct) module. Do not use strict `Engine<Crypt, _>` decode as a
/// token, key, or password-hash verifier. Use [`crate::ct::CtEngine`] with this
/// alphabet for secret-bearing comparison workflows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Crypt;

impl Alphabet for Crypt {
    const ENCODE: [u8; 64] = *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    #[inline]
    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

#[inline]
pub(crate) const fn encode_base64_value<A: Alphabet>(value: u8) -> u8 {
    encode_alphabet_value(value, &A::ENCODE)
}

#[derive(Clone, Copy)]
pub(crate) enum RuntimeEncodeMapper {
    StandardFamily { value_62: u8, value_63: u8 },
    ScannedTable,
}

impl RuntimeEncodeMapper {
    pub(crate) const fn for_alphabet<A: Alphabet>() -> Self {
        const STANDARD_PREFIX: [u8; 62] =
            *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

        let mut index = 0;
        while index < STANDARD_PREFIX.len() {
            if A::ENCODE[index] != STANDARD_PREFIX[index] {
                return Self::ScannedTable;
            }
            index += 1;
        }

        let value_62 = A::ENCODE[62];
        let value_63 = A::ENCODE[63];
        if (value_62 == b'+' && value_63 == b'/') || (value_62 == b'-' && value_63 == b'_') {
            Self::StandardFamily { value_62, value_63 }
        } else {
            Self::ScannedTable
        }
    }

    #[inline]
    pub(crate) fn encode<A: Alphabet>(self, value: u8) -> u8 {
        match self {
            Self::StandardFamily { value_62, value_63 } => {
                encode_ascii_base64(value, value_62, value_63)
            }
            Self::ScannedTable => encode_alphabet_value(value, &A::ENCODE),
        }
    }
}

pub(crate) struct RuntimeEncodeMapperFor<A: Alphabet>(core::marker::PhantomData<A>);

impl<A: Alphabet> RuntimeEncodeMapperFor<A> {
    pub(crate) const VALUE: RuntimeEncodeMapper = RuntimeEncodeMapper::for_alphabet::<A>();
}

#[inline]
const fn encode_alphabet_value(value: u8, encode: &[u8; 64]) -> u8 {
    let mut output = 0;
    let mut index = 0;
    let mut candidate = 0;
    while index < encode.len() {
        output |= encode[index] & ct_mask_eq_u8(value, candidate);
        index += 1;
        candidate += 1;
    }
    output
}

#[inline]
const fn encode_ascii_base64(value: u8, value_62_byte: u8, value_63_byte: u8) -> u8 {
    let upper = ct_mask_lt_u8(value, 26);
    let lower = ct_mask_lt_u8(value.wrapping_sub(26), 26);
    let digit = ct_mask_lt_u8(value.wrapping_sub(52), 10);
    let value_62 = ct_mask_eq_u8(value, 0x3e);
    let value_63 = ct_mask_eq_u8(value, 0x3f);

    (value.wrapping_add(b'A') & upper)
        | (value.wrapping_sub(26).wrapping_add(b'a') & lower)
        | (value.wrapping_sub(52).wrapping_add(b'0') & digit)
        | (value_62_byte & value_62)
        | (value_63_byte & value_63)
}

#[inline]
fn decode_ascii_base64(byte: u8, value_62_byte: u8, value_63_byte: u8) -> Option<u8> {
    let upper = ct_mask_lt_u8(byte.wrapping_sub(b'A'), 26);
    let lower = ct_mask_lt_u8(byte.wrapping_sub(b'a'), 26);
    let digit = ct_mask_lt_u8(byte.wrapping_sub(b'0'), 10);
    let value_62 = ct_mask_eq_u8(byte, value_62_byte);
    let value_63 = ct_mask_eq_u8(byte, value_63_byte);
    let valid = upper | lower | digit | value_62 | value_63;

    let decoded = (byte.wrapping_sub(b'A') & upper)
        | (byte.wrapping_sub(b'a').wrapping_add(26) & lower)
        | (byte.wrapping_sub(b'0').wrapping_add(52) & digit)
        | (0x3e & value_62)
        | (0x3f & value_63);

    if valid == 0 { None } else { Some(decoded) }
}
