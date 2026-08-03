use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

const PREFIX: &str = "Base64Secret:";
const MAX_SECRET_LENGTH: usize = 1_024;

#[derive(Clone, Copy)]
pub(crate) enum Alphabet {
    Standard,
    UrlSafe,
}

#[derive(Clone, Copy)]
pub(crate) enum Padding {
    Padded,
    Unpadded,
}

#[derive(Clone, Copy)]
pub(crate) enum Exposure {
    None,
    Read,
    ReadWrite,
}

pub(crate) struct Declaration {
    pub(crate) name: String,
    pub(crate) length: usize,
    pub(crate) alphabet: Alphabet,
    pub(crate) padding: Padding,
    pub(crate) exposure: Exposure,
}

impl Declaration {
    pub(crate) fn parse(input: TokenStream) -> Result<Self, String> {
        let tokens = input.into_iter().collect::<Vec<_>>();
        let policy = Policy::parse(&tokens)?;
        let struct_index = find_struct_index(&tokens)?;
        let name = struct_name(&tokens, struct_index)?;
        let field_group = tuple_field_group(&tokens, struct_index)?;
        let length = secret_array_length(&field_group)?;

        if length != policy.exact_length {
            return Err(format!(
                "{PREFIX} `exact_length = {}` does not match `SecretArray<{length}>`",
                policy.exact_length
            ));
        }
        if !(1..=MAX_SECRET_LENGTH).contains(&length) {
            return Err(format!(
                "{PREFIX} exact length must be between 1 and {MAX_SECRET_LENGTH} bytes"
            ));
        }

        Ok(Self {
            name,
            length,
            alphabet: policy.alphabet,
            padding: policy.padding,
            exposure: policy.exposure,
        })
    }
}

struct Policy {
    alphabet: Alphabet,
    padding: Padding,
    exact_length: usize,
    exposure: Exposure,
}

impl Policy {
    fn parse(tokens: &[TokenTree]) -> Result<Self, String> {
        let attributes = tokens
            .windows(2)
            .filter_map(|pair| match pair {
                [TokenTree::Punct(hash), TokenTree::Group(group)]
                    if hash.as_char() == '#'
                        && group.delimiter() == Delimiter::Bracket
                        && attribute_name(group).as_deref() == Some("base64_ng") =>
                {
                    Some(group.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        let attribute = match attributes.as_slice() {
            [] => {
                return Err(format!(
                    "{PREFIX} requires exactly one `#[base64_ng(alphabet = ..., padding = ..., exact_length = ..., exposure = ...)]` attribute"
                ));
            }
            [attribute] => attribute,
            _ => return Err(format!("{PREFIX} duplicate `base64_ng` attribute")),
        };

        let arguments = attribute_arguments(attribute)?;
        let mut alphabet = None;
        let mut padding = None;
        let mut exact_length = None;
        let mut exposure = None;

        for entry in split_entries(arguments)? {
            let (key, value) = parse_entry(&entry)?;
            match key.as_str() {
                "alphabet" => {
                    set_once(&mut alphabet, parse_alphabet(&value)?, "alphabet")?;
                }
                "padding" => {
                    set_once(&mut padding, parse_padding(&value)?, "padding")?;
                }
                "exact_length" => {
                    set_once(&mut exact_length, parse_length(&value)?, "exact_length")?;
                }
                "exposure" => {
                    set_once(&mut exposure, parse_exposure(&value)?, "exposure")?;
                }
                _ => {
                    return Err(format!(
                        "{PREFIX} unknown policy key `{key}`; expected `alphabet`, `padding`, `exact_length`, or `exposure`"
                    ));
                }
            }
        }

        Ok(Self {
            alphabet: alphabet.ok_or_else(|| missing("alphabet"))?,
            padding: padding.ok_or_else(|| missing("padding"))?,
            exact_length: exact_length.ok_or_else(|| missing("exact_length"))?,
            exposure: exposure.ok_or_else(|| missing("exposure"))?,
        })
    }
}

fn attribute_name(group: &Group) -> Option<String> {
    group
        .stream()
        .into_iter()
        .next()
        .and_then(|token| match token {
            TokenTree::Ident(ident) => Some(ident.to_string()),
            _ => None,
        })
}

fn attribute_arguments(group: &Group) -> Result<TokenStream, String> {
    let tokens = group.stream().into_iter().collect::<Vec<_>>();
    match tokens.as_slice() {
        [TokenTree::Ident(name), TokenTree::Group(arguments)]
            if name.to_string() == "base64_ng"
                && arguments.delimiter() == Delimiter::Parenthesis =>
        {
            Ok(arguments.stream())
        }
        _ => Err(format!(
            "{PREFIX} expected `#[base64_ng(alphabet = ..., padding = ..., exact_length = ..., exposure = ...)]`"
        )),
    }
}

fn split_entries(arguments: TokenStream) -> Result<Vec<Vec<TokenTree>>, String> {
    let mut entries = Vec::new();
    let mut current = Vec::new();
    for token in arguments {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            if current.is_empty() {
                return Err(format!("{PREFIX} empty policy entry"));
            }
            entries.push(core::mem::take(&mut current));
        } else {
            current.push(token);
        }
    }
    if !current.is_empty() {
        entries.push(current);
    }
    if entries.is_empty() {
        return Err(format!("{PREFIX} policy attribute may not be empty"));
    }
    Ok(entries)
}

fn parse_entry(tokens: &[TokenTree]) -> Result<(String, TokenTree), String> {
    match tokens {
        [TokenTree::Ident(key), TokenTree::Punct(equal), value] if equal.as_char() == '=' => {
            Ok((key.to_string(), value.clone()))
        }
        _ => Err(format!(
            "{PREFIX} every policy entry must use `key = value` syntax"
        )),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        return Err(format!("{PREFIX} duplicate policy key `{key}`"));
    }
    Ok(())
}

fn parse_alphabet(token: &TokenTree) -> Result<Alphabet, String> {
    match string_literal(token).as_deref() {
        Some("standard") => Ok(Alphabet::Standard),
        Some("url_safe") => Ok(Alphabet::UrlSafe),
        _ => Err(format!(
            "{PREFIX} `alphabet` must be \"standard\" or \"url_safe\""
        )),
    }
}

fn parse_padding(token: &TokenTree) -> Result<Padding, String> {
    match string_literal(token).as_deref() {
        Some("padded") => Ok(Padding::Padded),
        Some("unpadded") => Ok(Padding::Unpadded),
        _ => Err(format!(
            "{PREFIX} `padding` must be \"padded\" or \"unpadded\""
        )),
    }
}

fn parse_exposure(token: &TokenTree) -> Result<Exposure, String> {
    match string_literal(token).as_deref() {
        Some("none") => Ok(Exposure::None),
        Some("read") => Ok(Exposure::Read),
        Some("read_write") => Ok(Exposure::ReadWrite),
        _ => Err(format!(
            "{PREFIX} `exposure` must be \"none\", \"read\", or \"read_write\""
        )),
    }
}

fn string_literal(token: &TokenTree) -> Option<String> {
    let token = ungroup(token);
    let TokenTree::Literal(literal) = &token else {
        return None;
    };
    let value = literal.to_string();
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .map(str::to_owned)
}

fn parse_length(token: &TokenTree) -> Result<usize, String> {
    let token = ungroup(token);
    let TokenTree::Literal(literal) = &token else {
        return Err(format!(
            "{PREFIX} `exact_length` must be an unsuffixed decimal integer"
        ));
    };
    parse_decimal(&literal.to_string())
        .ok_or_else(|| format!("{PREFIX} `exact_length` must be an unsuffixed decimal integer"))
}

fn ungroup(token: &TokenTree) -> TokenTree {
    let mut current = token.clone();
    loop {
        let TokenTree::Group(group) = &current else {
            return current;
        };
        if group.delimiter() != Delimiter::None {
            return current;
        }
        let mut tokens = group.stream().into_iter();
        let Some(inner) = tokens.next() else {
            return current;
        };
        if tokens.next().is_some() {
            return current;
        }
        current = inner;
    }
}

fn missing(key: &str) -> String {
    format!("{PREFIX} missing required policy key `{key}`")
}

fn find_struct_index(tokens: &[TokenTree]) -> Result<usize, String> {
    tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(ident) if ident.to_string() == "struct"))
        .ok_or_else(|| format!("{PREFIX} can only be derived for a tuple struct"))
}

fn struct_name(tokens: &[TokenTree], struct_index: usize) -> Result<String, String> {
    tokens
        .get(struct_index + 1)
        .and_then(|token| match token {
            TokenTree::Ident(ident) => Some(ident.to_string()),
            _ => None,
        })
        .ok_or_else(|| format!("{PREFIX} could not find the struct name"))
}

fn tuple_field_group(tokens: &[TokenTree], struct_index: usize) -> Result<Group, String> {
    match tokens.get(struct_index + 2) {
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
            Ok(group.clone())
        }
        Some(TokenTree::Punct(punct)) if punct.as_char() == '<' => Err(format!(
            "{PREFIX} generic structs are not supported; use a concrete `SecretArray<N>` field"
        )),
        _ => Err(format!(
            "{PREFIX} supports only tuple structs like `struct Key(base64_ng::secret::SecretArray<32>);`"
        )),
    }
}

fn secret_array_length(group: &Group) -> Result<usize, String> {
    let mut tokens = group.stream().into_iter().collect::<Vec<_>>();
    if matches!(tokens.last(), Some(TokenTree::Punct(punct)) if punct.as_char() == ',') {
        tokens.pop();
    }
    if tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ','))
    {
        return Err(format!("{PREFIX} supports exactly one tuple field"));
    }
    if matches!(tokens.first(), Some(TokenTree::Ident(ident)) if ident.to_string() == "pub") {
        return Err(format!(
            "{PREFIX} the `SecretArray<N>` tuple field must be private"
        ));
    }
    if tokens
        .iter()
        .any(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == '#'))
    {
        return Err(format!("{PREFIX} tuple field attributes are not supported"));
    }

    let normalized = tokens.iter().map(TokenTree::to_string).collect::<String>();
    let marker = "base64_ng::secret::SecretArray<";
    let without_root = normalized.strip_prefix("::").unwrap_or(&normalized);
    if !without_root.starts_with(marker) || !without_root.ends_with('>') {
        return Err(format!(
            "{PREFIX} field must be `base64_ng::secret::SecretArray<N>`"
        ));
    }
    let length = &without_root[marker.len()..without_root.len() - 1];
    parse_decimal(length).ok_or_else(|| {
        format!("{PREFIX} `SecretArray<N>` length must be an unsuffixed decimal integer")
    })
}

fn parse_decimal(value: &str) -> Option<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}
