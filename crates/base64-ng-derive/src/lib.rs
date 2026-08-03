#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

//! Derive support for fixed-size 2.0 `base64-ng` secret owners.
//!
//! [`Base64Secret`] accepts one private
//! `base64_ng::secret::SecretArray<N>` tuple field. A required
//! `#[base64_ng(...)]` attribute selects one of the four sealed strict codecs,
//! repeats the exact decoded length, and states whether generated explicit
//! exposure methods are absent, read-only, or read-write.

mod expand;
mod parse;

use proc_macro::TokenStream;

/// Derives fixed-size Base64 secret decoding and encoding methods.
///
/// ```
/// use base64_ng::secret::SecretInput;
/// use base64_ng_derive::Base64Secret;
///
/// #[derive(Base64Secret)]
/// #[base64_ng(
///     alphabet = "url_safe",
///     padding = "unpadded",
///     exact_length = 32,
///     exposure = "read"
/// )]
/// struct ApiKey(base64_ng::secret::SecretArray<32>);
///
/// let input = SecretInput::new(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
/// let key = ApiKey::decode_base64(&input)?;
/// assert_eq!(key.expose_secret().len(), 32);
/// # Ok::<(), base64_ng::secret::SecretDecodeError>(())
/// ```
///
/// The declaration must name every policy explicitly. Supported values are:
///
/// - `alphabet = "standard" | "url_safe"`;
/// - `padding = "padded" | "unpadded"`;
/// - `exact_length = N`, where `1 <= N <= 1024` and the field is
///   `SecretArray<N>`; and
/// - `exposure = "none" | "read" | "read_write"`.
///
/// The expansion uses `SecretArrayFrame` for staged fixed-work decode and
/// `SecretArrayEncoder` for secret-preserving encode. It generates no
/// `AsRef`, `AsMut`, `Deref`, `Clone`, `Copy`, `PartialEq`, `FromStr`, or
/// ordinary slice conversion implementation.
#[proc_macro_derive(Base64Secret, attributes(base64_ng))]
pub fn derive_base64_secret(input: TokenStream) -> TokenStream {
    match parse::Declaration::parse(input).and_then(expand::expand) {
        Ok(tokens) => tokens,
        Err(error) => compile_error(&error),
    }
}

fn compile_error(message: &str) -> TokenStream {
    let source = format!("compile_error!({message:?});");
    match source.parse() {
        Ok(tokens) => tokens,
        Err(_) => TokenStream::new(),
    }
}
