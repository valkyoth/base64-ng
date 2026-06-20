#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Derive helpers for fixed-size `base64-ng` byte newtypes.
//!
//! This proc-macro crate is intentionally narrow and dependency-free. The
//! [`Base64Secret`] derive supports tuple structs with exactly one `[u8; N]`
//! field and generates Base64 parsing/encoding helpers around that field.

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

/// Derives fixed-size Base64 helper methods for a tuple newtype.
///
/// Supported input:
///
/// ```
/// use base64_ng_derive::Base64Secret;
///
/// #[derive(Base64Secret)]
/// struct ApiKey([u8; 32]);
/// ```
///
/// Generated behavior:
///
/// - `from_base64(&[u8])` and `from_base64_str(&str)` decode with
///   `base64_ng::ct::STANDARD.decode_slice_staged_clear_tail`.
/// - `encode_base64::<CAP>()` encodes with `base64_ng::STANDARD`.
/// - `Debug` is redacted.
/// - `Drop` clears the wrapped bytes with `base64_ng::clear_bytes`.
#[proc_macro_derive(Base64Secret)]
pub fn derive_base64_secret(input: TokenStream) -> TokenStream {
    match expand_base64_secret(input) {
        Ok(tokens) => tokens,
        Err(error) => compile_error(&error),
    }
}

fn expand_base64_secret(input: TokenStream) -> Result<TokenStream, String> {
    let tokens = input.into_iter().collect::<Vec<_>>();
    let struct_index = find_struct_index(&tokens)?;
    let name = struct_name(&tokens, struct_index)?;
    let field_group = tuple_field_group(&tokens, struct_index)?;
    let length = array_field_length(&field_group)?;

    let expanded = format!(
        r#"
impl {name} {{
    /// Parses strict standard padded Base64 into this fixed-size newtype.
    ///
    /// Decoding uses `base64_ng::ct::STANDARD` and stages plaintext privately
    /// until the full input has been accepted.
    pub fn from_base64(input: &[u8]) -> Result<Self, ::base64_ng::DecodeError> {{
        let required = ::base64_ng::ct::STANDARD.decoded_len(input)?;
        if required != {length} {{
            return Err(::base64_ng::DecodeError::InvalidLength);
        }}

        let mut output: [u8; {length}] = [0u8; {length}];
        let mut staging: [u8; {length}] = [0u8; {length}];
        let written = match ::base64_ng::ct::STANDARD
            .decode_slice_staged_clear_tail(input, &mut output, &mut staging)
        {{
            Ok(written) => written,
            Err(error) => {{
                ::base64_ng::clear_bytes(&mut output);
                ::base64_ng::clear_bytes(&mut staging);
                return Err(error);
            }}
        }};

        if written != {length} {{
            ::base64_ng::clear_bytes(&mut output);
            ::base64_ng::clear_bytes(&mut staging);
            return Err(::base64_ng::DecodeError::InvalidLength);
        }}

        ::base64_ng::clear_bytes(&mut staging);
        Ok(Self(output))
    }}

    /// Parses strict standard padded Base64 text into this fixed-size newtype.
    pub fn from_base64_str(input: &str) -> Result<Self, ::base64_ng::DecodeError> {{
        Self::from_base64(input.as_bytes())
    }}

    /// Encodes this value as strict standard padded Base64.
    pub fn encode_base64<const CAP: usize>(
        &self,
    ) -> Result<::base64_ng::EncodedBuffer<CAP>, ::base64_ng::EncodeError> {{
        ::base64_ng::STANDARD.encode_buffer::<CAP>(&self.0)
    }}

    /// Returns the wrapped bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {{
        &self.0
    }}

    /// Returns the wrapped bytes mutably.
    ///
    /// Callers that mutate secret bytes are responsible for preserving any
    /// application-level invariants attached to this newtype.
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {{
        &mut self.0
    }}

    /// Compares two values with a fixed-width, constant-time-oriented scan.
    ///
    /// The final equality result is public. This inherits the caveats of
    /// `base64_ng::constant_time_eq_fixed_width`.
    #[must_use]
    pub fn constant_time_eq(&self, other: &Self) -> bool {{
        ::base64_ng::constant_time_eq_fixed_width(&self.0, &other.0)
    }}
}}

impl ::core::convert::TryFrom<&[u8]> for {name} {{
    type Error = ::base64_ng::DecodeError;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {{
        Self::from_base64(input)
    }}
}}

impl ::core::convert::TryFrom<&str> for {name} {{
    type Error = ::base64_ng::DecodeError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {{
        Self::from_base64_str(input)
    }}
}}

impl ::core::str::FromStr for {name} {{
    type Err = ::base64_ng::DecodeError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {{
        Self::from_base64_str(input)
    }}
}}

impl ::core::convert::From<[u8; {length}]> for {name} {{
    fn from(bytes: [u8; {length}]) -> Self {{
        Self(bytes)
    }}
}}

impl ::core::convert::AsRef<[u8]> for {name} {{
    fn as_ref(&self) -> &[u8] {{
        self.as_bytes()
    }}
}}

impl ::core::fmt::Debug for {name} {{
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
        formatter
            .debug_struct(::core::stringify!({name}))
            .field("bytes", &"<redacted>")
            .field("len", &{length})
            .finish()
    }}
}}

impl ::core::ops::Drop for {name} {{
    fn drop(&mut self) {{
        ::base64_ng::clear_bytes(&mut self.0);
    }}
}}
"#
    );

    expanded
        .parse()
        .map_err(|error| format!("failed to generate Base64Secret impl: {error}"))
}

fn find_struct_index(tokens: &[TokenTree]) -> Result<usize, String> {
    tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(ident) if ident.to_string() == "struct"))
        .ok_or_else(|| "Base64Secret can only be derived for tuple structs".to_owned())
}

fn struct_name(tokens: &[TokenTree], struct_index: usize) -> Result<String, String> {
    tokens
        .get(struct_index + 1)
        .and_then(|token| match token {
            TokenTree::Ident(ident) => Some(ident.to_string()),
            _ => None,
        })
        .ok_or_else(|| "Base64Secret could not find a struct name".to_owned())
}

fn tuple_field_group(tokens: &[TokenTree], struct_index: usize) -> Result<Group, String> {
    tokens
        .iter()
        .skip(struct_index + 2)
        .find_map(|token| match token {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                Some(group.clone())
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => None,
            _ => None,
        })
        .ok_or_else(|| {
            "Base64Secret supports only tuple structs like `struct Key([u8; 32]);`".to_owned()
        })
}

fn array_field_length(group: &Group) -> Result<String, String> {
    let mut field_tokens = strip_field_visibility(group.stream().into_iter().collect::<Vec<_>>())?;
    if field_tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ','))
    {
        return Err("Base64Secret supports exactly one tuple field".to_owned());
    }

    if field_tokens.len() != 1 {
        return Err("Base64Secret field must be exactly `[u8; N]`".to_owned());
    }

    let Some(TokenTree::Group(array_group)) = field_tokens.pop() else {
        return Err("Base64Secret field must be a `[u8; N]` byte array".to_owned());
    };
    if array_group.delimiter() != Delimiter::Bracket {
        return Err("Base64Secret field must be a `[u8; N]` byte array".to_owned());
    }

    let array_tokens = array_group.stream().into_iter().collect::<Vec<_>>();
    parse_array_length(&array_tokens)
}

fn strip_field_visibility(mut tokens: Vec<TokenTree>) -> Result<Vec<TokenTree>, String> {
    if matches!(tokens.first(), Some(TokenTree::Ident(ident)) if ident.to_string() == "pub") {
        tokens.remove(0);
        if matches!(tokens.first(), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
        {
            tokens.remove(0);
        }
    }
    if tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#'))
    {
        return Err("Base64Secret tuple fields may not have field attributes".to_owned());
    }
    Ok(tokens)
}

fn parse_array_length(tokens: &[TokenTree]) -> Result<String, String> {
    let semicolon = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ';'))
        .ok_or_else(|| "Base64Secret field must be `[u8; N]`".to_owned())?;

    let element = tokens[..semicolon]
        .iter()
        .map(TokenTree::to_string)
        .collect::<String>();
    if element != "u8" {
        return Err("Base64Secret field element type must be `u8`".to_owned());
    }

    let length = tokens[semicolon + 1..]
        .iter()
        .map(TokenTree::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    if length.is_empty() {
        return Err("Base64Secret field length is missing".to_owned());
    }
    Ok(length)
}

fn compile_error(message: &str) -> TokenStream {
    let source = format!("compile_error!({message:?});");
    match source.parse() {
        Ok(tokens) => tokens,
        Err(_) => TokenStream::new(),
    }
}
