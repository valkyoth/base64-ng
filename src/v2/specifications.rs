//! Sealed, validated codec specifications for the 2.0 core.

use super::alphabet::{
    STANDARD_ALPHABET, URL_SAFE_ALPHABET, ValidatedAlphabet, ValidatedAlphabetError,
};

/// Whether ordinary encoding emits canonical `=` padding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EncodePadding {
    /// Emit canonical padding.
    Padded,
    /// Omit padding.
    Unpadded,
}

/// Which padding forms ordinary decoding accepts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DecodePadding {
    /// Require the canonical padding count for the decoded length.
    RequireCanonical,
    /// Reject every padding byte.
    Forbid,
    /// Accept canonical, absent, or partially present trailing padding.
    Indifferent,
}

/// Whether ordinary decoding enforces zero unused trailing bits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TrailingBits {
    /// Reject noncanonical unused trailing bits.
    RequireCanonical,
    /// Ignore nonzero unused trailing bits for compatibility inputs.
    AllowNonCanonical,
}

/// One complete, immutable ordinary codec policy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CodecSettings {
    alphabet: ValidatedAlphabet,
    encode_padding: EncodePadding,
    decode_padding: DecodePadding,
    trailing_bits: TrailingBits,
}

impl CodecSettings {
    const fn new(
        alphabet: ValidatedAlphabet,
        encode_padding: EncodePadding,
        decode_padding: DecodePadding,
        trailing_bits: TrailingBits,
    ) -> Self {
        Self {
            alphabet,
            encode_padding,
            decode_padding,
            trailing_bits,
        }
    }

    /// Returns the validated alphabet owned by this specification.
    #[must_use]
    pub const fn alphabet(&self) -> &ValidatedAlphabet {
        &self.alphabet
    }

    /// Returns the encode-padding policy.
    #[must_use]
    pub const fn encode_padding(&self) -> EncodePadding {
        self.encode_padding
    }

    /// Returns the decode-padding policy.
    #[must_use]
    pub const fn decode_padding(&self) -> DecodePadding {
        self.decode_padding
    }

    /// Returns the trailing-bit policy.
    #[must_use]
    pub const fn trailing_bits(&self) -> TrailingBits {
        self.trailing_bits
    }

    /// Returns whether the policy is eligible for a future secret codec.
    ///
    /// Runtime alphabets are eligible, but padding-indifferent and
    /// noncanonical-trailing-bit policies are deliberately excluded.
    #[must_use]
    pub const fn permits_secret_processing(&self) -> bool {
        !matches!(self.decode_padding, DecodePadding::Indifferent)
            && matches!(self.trailing_bits, TrailingBits::RequireCanonical)
    }
}

mod sealed {
    pub trait Sealed {}
}

/// The single sealed consumer boundary for a complete codec specification.
///
/// The trait is object-safe for integration boundaries, while ordinary hot
/// paths remain generic over one whole specification type. It intentionally
/// has no associated constants so runtime specifications and trait objects use
/// the same contract.
pub trait Codec: sealed::Sealed + Send + Sync {
    /// Returns the complete validated settings value.
    fn settings(&self) -> CodecSettings;
}

/// A codec value parameterized by one complete sealed specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Base64<S> {
    specification: S,
}

impl<S: Codec> Base64<S> {
    /// Constructs a codec from one sealed specification value.
    pub const fn new(specification: S) -> Self {
        Self { specification }
    }

    /// Returns the owned specification value.
    pub const fn specification(&self) -> &S {
        &self.specification
    }
    /// Returns the codec's complete validated settings.
    pub fn settings(&self) -> CodecSettings {
        self.specification.settings()
    }
}

macro_rules! strict_specification {
    ($name:ident, $alphabet:expr, $encode:expr, $decode:expr) => {
        #[doc = "A sealed strict RFC 4648 built-in specification."]
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
        pub struct $name;

        impl $name {
            const SETTINGS: CodecSettings =
                CodecSettings::new($alphabet, $encode, $decode, TrailingBits::RequireCanonical);

            pub(crate) const fn const_settings() -> CodecSettings {
                Self::SETTINGS
            }
        }

        impl sealed::Sealed for $name {}

        impl Codec for $name {
            fn settings(&self) -> CodecSettings {
                Self::SETTINGS
            }
        }
    };
}

strict_specification!(
    StrictStandardPadded,
    STANDARD_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::RequireCanonical
);
strict_specification!(
    StrictStandardUnpadded,
    STANDARD_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid
);
strict_specification!(
    StrictUrlSafePadded,
    URL_SAFE_ALPHABET,
    EncodePadding::Padded,
    DecodePadding::RequireCanonical
);
strict_specification!(
    StrictUrlSafeUnpadded,
    URL_SAFE_ALPHABET,
    EncodePadding::Unpadded,
    DecodePadding::Forbid
);

/// Strict RFC 4648 Standard Base64 with canonical padding.
pub const STRICT_STANDARD_PADDED: Base64<StrictStandardPadded> = Base64::new(StrictStandardPadded);
/// Strict RFC 4648 Standard Base64 without padding.
pub const STRICT_STANDARD_UNPADDED: Base64<StrictStandardUnpadded> =
    Base64::new(StrictStandardUnpadded);
/// Strict RFC 4648 URL-safe Base64 with canonical padding.
pub const STRICT_URL_SAFE_PADDED: Base64<StrictUrlSafePadded> = Base64::new(StrictUrlSafePadded);
/// Strict RFC 4648 URL-safe Base64 without padding.
pub const STRICT_URL_SAFE_UNPADDED: Base64<StrictUrlSafeUnpadded> =
    Base64::new(StrictUrlSafeUnpadded);

/// A complete owned runtime specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeSpec {
    settings: CodecSettings,
}

impl RuntimeSpec {
    pub(crate) const fn const_settings(&self) -> CodecSettings {
        self.settings
    }
}

impl sealed::Sealed for RuntimeSpec {}

impl Codec for RuntimeSpec {
    fn settings(&self) -> CodecSettings {
        self.settings
    }
}

/// A policy combination that cannot form a self-consistent codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecBuilderError {
    /// Encoding emits padding that decoding always rejects.
    EncodedPaddingRejected,
    /// Encoding omits padding that decoding requires.
    EncodedPaddingRequired,
}

impl core::fmt::Display for CodecBuilderError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EncodedPaddingRejected => {
                formatter.write_str("encoding emits padding rejected by decode policy")
            }
            Self::EncodedPaddingRequired => {
                formatter.write_str("encoding omits padding required by decode policy")
            }
        }
    }
}

/// Fallible no-allocation builder for an advanced ordinary runtime codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodecBuilder {
    settings: CodecSettings,
}

impl CodecBuilder {
    /// Starts with strict canonical padded policies and an owned alphabet.
    #[must_use]
    pub const fn new(alphabet: ValidatedAlphabet) -> Self {
        Self {
            settings: CodecSettings::new(
                alphabet,
                EncodePadding::Padded,
                DecodePadding::RequireCanonical,
                TrailingBits::RequireCanonical,
            ),
        }
    }

    /// Validates and owns an alphabet table before constructing the builder.
    pub const fn from_table(table: [u8; 64]) -> Result<Self, ValidatedAlphabetError> {
        match ValidatedAlphabet::new(table) {
            Ok(alphabet) => Ok(Self::new(alphabet)),
            Err(error) => Err(error),
        }
    }

    /// Copies and validates a runtime alphabet slice.
    pub const fn from_slice(bytes: &[u8]) -> Result<Self, ValidatedAlphabetError> {
        match ValidatedAlphabet::try_from_slice(bytes) {
            Ok(alphabet) => Ok(Self::new(alphabet)),
            Err(error) => Err(error),
        }
    }

    /// Sets whether encoding emits padding.
    #[must_use]
    pub const fn encode_padding(mut self, policy: EncodePadding) -> Self {
        self.settings.encode_padding = policy;
        self
    }

    /// Sets the ordinary decode-padding policy.
    #[must_use]
    pub const fn decode_padding(mut self, policy: DecodePadding) -> Self {
        self.settings.decode_padding = policy;
        self
    }

    /// Sets the ordinary trailing-bit canonicality policy.
    #[must_use]
    pub const fn trailing_bits(mut self, policy: TrailingBits) -> Self {
        self.settings.trailing_bits = policy;
        self
    }

    /// Validates policy compatibility and constructs an owned runtime codec.
    pub const fn build(self) -> Result<Base64<RuntimeSpec>, CodecBuilderError> {
        match (self.settings.encode_padding, self.settings.decode_padding) {
            (EncodePadding::Padded, DecodePadding::Forbid) => {
                Err(CodecBuilderError::EncodedPaddingRejected)
            }
            (EncodePadding::Unpadded, DecodePadding::RequireCanonical) => {
                Err(CodecBuilderError::EncodedPaddingRequired)
            }
            _ => Ok(Base64::new(RuntimeSpec {
                settings: self.settings,
            })),
        }
    }
}
