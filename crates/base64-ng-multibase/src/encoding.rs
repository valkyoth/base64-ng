use base64_ng::{
    Base64, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, StrictStandardPadded, StrictStandardUnpadded, StrictUrlSafePadded,
    StrictUrlSafeUnpadded,
};

/// One admitted Base64-family multibase registration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Base64MultibaseEncoding {
    /// `m`: RFC 4648 Standard alphabet without padding.
    Base64,
    /// `M`: RFC 4648 Standard alphabet with canonical padding.
    Base64Pad,
    /// `u`: RFC 4648 URL-safe alphabet without padding.
    Base64Url,
    /// `U`: RFC 4648 URL-safe alphabet with canonical padding.
    Base64UrlPad,
}

/// Status recorded by the pinned multibase registry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MultibaseRegistryStatus {
    /// The registry marks this encoding final.
    Final,
    /// The registry marks this encoding experimental.
    Experimental,
}

impl Base64MultibaseEncoding {
    /// Every Base64-family encoding admitted by this crate.
    pub const ALL: [Self; 4] = [
        Self::Base64,
        Self::Base64Pad,
        Self::Base64Url,
        Self::Base64UrlPad,
    ];

    /// Returns the exact, case-sensitive multibase prefix byte.
    #[must_use]
    pub const fn prefix(self) -> u8 {
        match self {
            Self::Base64 => b'm',
            Self::Base64Pad => b'M',
            Self::Base64Url => b'u',
            Self::Base64UrlPad => b'U',
        }
    }

    /// Returns the registry encoding name.
    #[must_use]
    pub const fn registry_name(self) -> &'static str {
        match self {
            Self::Base64 => "base64",
            Self::Base64Pad => "base64pad",
            Self::Base64Url => "base64url",
            Self::Base64UrlPad => "base64urlpad",
        }
    }

    /// Returns the status in the pinned registry snapshot.
    #[must_use]
    pub const fn registry_status(self) -> MultibaseRegistryStatus {
        match self {
            Self::Base64Pad => MultibaseRegistryStatus::Experimental,
            Self::Base64 | Self::Base64Url | Self::Base64UrlPad => MultibaseRegistryStatus::Final,
        }
    }

    /// Resolves one exact Base64-family prefix.
    #[must_use]
    pub const fn from_prefix(prefix: u8) -> Option<Self> {
        match prefix {
            b'm' => Some(Self::Base64),
            b'M' => Some(Self::Base64Pad),
            b'u' => Some(Self::Base64Url),
            b'U' => Some(Self::Base64UrlPad),
            _ => None,
        }
    }

    pub(crate) const fn codec(self) -> CodecDispatch {
        match self {
            Self::Base64 => CodecDispatch::StandardUnpadded(STRICT_STANDARD_UNPADDED),
            Self::Base64Pad => CodecDispatch::StandardPadded(STRICT_STANDARD_PADDED),
            Self::Base64Url => CodecDispatch::UrlSafeUnpadded(STRICT_URL_SAFE_UNPADDED),
            Self::Base64UrlPad => CodecDispatch::UrlSafePadded(STRICT_URL_SAFE_PADDED),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CodecDispatch {
    StandardUnpadded(Base64<StrictStandardUnpadded>),
    StandardPadded(Base64<StrictStandardPadded>),
    UrlSafeUnpadded(Base64<StrictUrlSafeUnpadded>),
    UrlSafePadded(Base64<StrictUrlSafePadded>),
}

impl CodecDispatch {
    pub(crate) fn encoded_len(self, input_len: usize) -> Result<usize, base64_ng::OneShotError> {
        match self {
            Self::StandardUnpadded(codec) => codec.encoded_len(input_len),
            Self::StandardPadded(codec) => codec.encoded_len(input_len),
            Self::UrlSafeUnpadded(codec) => codec.encoded_len(input_len),
            Self::UrlSafePadded(codec) => codec.encoded_len(input_len),
        }
    }

    pub(crate) fn decoded_len(self, input: &[u8]) -> Result<usize, base64_ng::OneShotError> {
        match self {
            Self::StandardUnpadded(codec) => codec.decoded_len(input),
            Self::StandardPadded(codec) => codec.decoded_len(input),
            Self::UrlSafeUnpadded(codec) => codec.decoded_len(input),
            Self::UrlSafePadded(codec) => codec.decoded_len(input),
        }
    }

    pub(crate) fn encode_into(
        self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, base64_ng::OneShotError> {
        match self {
            Self::StandardUnpadded(codec) => codec.encode_into(input, output),
            Self::StandardPadded(codec) => codec.encode_into(input, output),
            Self::UrlSafeUnpadded(codec) => codec.encode_into(input, output),
            Self::UrlSafePadded(codec) => codec.encode_into(input, output),
        }
    }

    pub(crate) fn decode_into(
        self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, base64_ng::OneShotError> {
        match self {
            Self::StandardUnpadded(codec) => codec.decode_into(input, output),
            Self::StandardPadded(codec) => codec.decode_into(input, output),
            Self::UrlSafeUnpadded(codec) => codec.decode_into(input, output),
            Self::UrlSafePadded(codec) => codec.decode_into(input, output),
        }
    }

    pub(crate) fn encoder(self) -> base64_ng::EncoderState {
        match self {
            Self::StandardUnpadded(codec) => codec.encoder(),
            Self::StandardPadded(codec) => codec.encoder(),
            Self::UrlSafeUnpadded(codec) => codec.encoder(),
            Self::UrlSafePadded(codec) => codec.encoder(),
        }
    }

    pub(crate) fn decoder(self) -> base64_ng::DecoderState {
        match self {
            Self::StandardUnpadded(codec) => codec.decoder(),
            Self::StandardPadded(codec) => codec.decoder(),
            Self::UrlSafeUnpadded(codec) => codec.decoder(),
            Self::UrlSafePadded(codec) => codec.decoder(),
        }
    }
}
