use proc_macro::TokenStream;

use crate::parse::{Alphabet, Declaration, Exposure, Padding};

pub(crate) fn expand(declaration: Declaration) -> Result<TokenStream, String> {
    let codec = match (declaration.alphabet, declaration.padding) {
        (Alphabet::Standard, Padding::Padded) => "STRICT_STANDARD_PADDED",
        (Alphabet::Standard, Padding::Unpadded) => "STRICT_STANDARD_UNPADDED",
        (Alphabet::UrlSafe, Padding::Padded) => "STRICT_URL_SAFE_PADDED",
        (Alphabet::UrlSafe, Padding::Unpadded) => "STRICT_URL_SAFE_UNPADDED",
    };
    let encoded_length = encoded_length(declaration.length, declaration.padding)?;
    let exposure = exposure_methods(declaration.exposure);
    let name = declaration.name;
    let length = declaration.length;

    let expanded = format!(
        r#"
impl {name} {{
    /// Exact decoded byte length enforced by this type.
    pub const EXACT_LENGTH: usize = {length};

    /// Exact encoded byte length for this type's admitted codec.
    pub const ENCODED_LENGTH: usize = {encoded_length};

    /// Decodes one classified Base64 value through bounded staged storage.
    pub fn decode_base64(
        input: &::base64_ng::secret::SecretInput<'_>,
    ) -> ::core::result::Result<Self, ::base64_ng::secret::SecretDecodeError> {{
        let mut frame = ::base64_ng::secret::SecretArrayFrame::<{length}>::new(
            &::base64_ng::{codec},
        )?;
        let _ = frame.update(input)?;
        let secret = frame.finish()?;
        if secret.len() != {length} {{
            return ::core::result::Result::Err(
                ::base64_ng::secret::SecretDecodeError::InvalidInput,
            );
        }}
        ::core::result::Result::Ok(Self(secret))
    }}

    /// Encodes this value into a fixed-capacity wiping secret owner.
    pub fn encode_base64(
        &self,
    ) -> ::core::result::Result<
        ::base64_ng::secret::SecretArray<{encoded_length}>,
        ::base64_ng::secret::SecretEncodeError,
    > {{
        let exposed = self.0.expose_secret();
        let input = ::base64_ng::secret::SecretInput::new(exposed.as_bytes());
        ::base64_ng::secret::SecretArrayEncoder::<{encoded_length}>::encode(
            &::base64_ng::{codec},
            &input,
        )
    }}

{exposure}
}}

impl ::core::fmt::Debug for {name} {{
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
        formatter
            .debug_struct(::core::stringify!({name}))
            .field("secret", &"<redacted>")
            .field("len", &{length})
            .finish()
    }}
}}

impl ::core::fmt::Display for {name} {{
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
        formatter.write_str("<redacted secret>")
    }}
}}
"#
    );

    expanded
        .parse()
        .map_err(|error| format!("Base64Secret: failed to generate implementation: {error}"))
}

fn exposure_methods(exposure: Exposure) -> &'static str {
    match exposure {
        Exposure::None => "",
        Exposure::Read => {
            r"    /// Explicitly exposes the secret bytes through a redacted view.
    #[must_use]
    pub fn expose_secret(&self) -> ::base64_ng::secret::ExposedSecret<'_> {
        self.0.expose_secret()
    }
"
        }
        Exposure::ReadWrite => {
            r"    /// Explicitly exposes the secret bytes through a redacted view.
    #[must_use]
    pub fn expose_secret(&self) -> ::base64_ng::secret::ExposedSecret<'_> {
        self.0.expose_secret()
    }

    /// Explicitly exposes the secret bytes through a mutable redacted view.
    #[must_use]
    pub fn expose_secret_mut(&mut self) -> ::base64_ng::secret::ExposedSecretMut<'_> {
        self.0.expose_secret_mut()
    }
"
        }
    }
}

fn encoded_length(length: usize, padding: Padding) -> Result<usize, String> {
    let full = length
        .checked_div(3)
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| "Base64Secret: encoded length overflow".to_owned())?;
    let tail = match (padding, length % 3) {
        (_, 0) => 0,
        (Padding::Padded, _) => 4,
        (Padding::Unpadded, 1) => 2,
        (Padding::Unpadded, 2) => 3,
        (Padding::Unpadded, _) => {
            return Err("Base64Secret: invalid encoded-length remainder".to_owned());
        }
    };
    full.checked_add(tail)
        .ok_or_else(|| "Base64Secret: encoded length overflow".to_owned())
}
