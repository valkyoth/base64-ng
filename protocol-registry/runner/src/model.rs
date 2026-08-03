use crate::Case;

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const IMAP: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,";

pub(crate) fn evaluate(case: &Case) -> Option<Vec<u8>> {
    match case.registry_id.as_str() {
        "mime-body" => mime(&case.wire),
        "pem-textual" => pem(&case.wire),
        "multibase-base64" => multibase(&case.wire),
        "imap-mutf7-payload" => imap(&case.wire),
        "passlib-pbkdf2" => pbkdf2(&case.wire).then(Vec::new),
        "sha-crypt" => sha_crypt(&case.wire).then(Vec::new),
        "openpgp-armor" => openpgp(&case.wire),
        other => panic!("unregistered model {other}"),
    }
}

fn mime(input: &[u8]) -> Option<Vec<u8>> {
    if input.is_empty() {
        return Some(Vec::new());
    }
    if !input.ends_with(b"\r\n") {
        return None;
    }
    let lines = split_exact(&input[..input.len() - 2], b"\r\n");
    if lines.is_empty()
        || lines.iter().any(|line| line.is_empty() || line.len() > 76)
        || lines
            .iter()
            .take(lines.len() - 1)
            .any(|line| line.len() != 76)
    {
        return None;
    }
    decode(&lines.concat(), STANDARD, Padding::Required)
}

fn pem(input: &[u8]) -> Option<Vec<u8>> {
    if !input.ends_with(b"\r\n") {
        return None;
    }
    let lines = split_exact(&input[..input.len() - 2], b"\r\n");
    if lines.len() < 3 {
        return None;
    }
    let label = boundary(lines[0], b"-----BEGIN ")?;
    if boundary(lines[lines.len() - 1], b"-----END ")? != label {
        return None;
    }
    let body = &lines[1..lines.len() - 1];
    if body.iter().any(|line| line.is_empty() || line.len() > 64)
        || body
            .iter()
            .take(body.len().saturating_sub(1))
            .any(|line| line.len() != 64)
    {
        return None;
    }
    decode(&body.concat(), STANDARD, Padding::Required)
}

fn multibase(input: &[u8]) -> Option<Vec<u8>> {
    let (prefix, body) = input.split_first()?;
    match prefix {
        b'm' => decode(body, STANDARD, Padding::Forbidden),
        b'M' => decode(body, STANDARD, Padding::Required),
        b'u' => decode(
            body,
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
            Padding::Forbidden,
        ),
        b'U' => decode(
            body,
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
            Padding::Required,
        ),
        _ => None,
    }
}

fn imap(input: &[u8]) -> Option<Vec<u8>> {
    let decoded = decode(input, IMAP, Padding::Forbidden)?;
    (decoded.len() % 2 == 0).then_some(decoded)
}

fn pbkdf2(input: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(input) else {
        return false;
    };
    let fields: Vec<_> = text.split('$').collect();
    if fields.len() != 5 || !fields[0].is_empty() {
        return false;
    }
    let checksum_len = match fields[1] {
        "pbkdf2" => 27,
        "pbkdf2-sha256" => 43,
        "pbkdf2-sha512" => 86,
        _ => return false,
    };
    canonical_decimal(fields[2])
        && !fields[3].is_empty()
        && fields[4].len() == checksum_len
        && [fields[3], fields[4]].iter().all(|field| {
            field
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/'))
                && decode(
                    field.as_bytes(),
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./",
                    Padding::Forbidden,
                )
                .is_some()
        })
}

fn sha_crypt(input: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(input) else {
        return false;
    };
    let fields: Vec<_> = text.split('$').collect();
    if fields.len() != 4 || !fields[0].is_empty() {
        return false;
    }
    let checksum_len = match fields[1] {
        "5" => 43,
        "6" => 86,
        _ => return false,
    };
    !fields[2].is_empty()
        && fields[2].len() <= 16
        && fields[2]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/'))
        && fields[3].len() == checksum_len
        && fields[3]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/'))
}

fn openpgp(input: &[u8]) -> Option<Vec<u8>> {
    let normalized = input.strip_suffix(b"\n")?;
    let lines = split_exact(normalized, b"\n");
    if lines.len() < 4 {
        return None;
    }
    let label = boundary(lines[0], b"-----BEGIN ")?;
    if !matches!(
        label,
        b"PGP MESSAGE" | b"PGP PUBLIC KEY BLOCK" | b"PGP PRIVATE KEY BLOCK" | b"PGP SIGNATURE"
    ) {
        return None;
    }
    let blank = lines.iter().position(|line| line.is_empty())?;
    if blank == 0 || boundary(lines[lines.len() - 1], b"-----END ")? != label {
        return None;
    }
    let body = &lines[blank + 1..lines.len() - 1];
    if body.is_empty() || body.iter().any(|line| line.is_empty() || line.len() > 76) {
        return None;
    }
    decode(&body.concat(), STANDARD, Padding::Required)
}

fn boundary<'a>(line: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    line.strip_prefix(prefix)?.strip_suffix(b"-----")
}

fn canonical_decimal(value: &str) -> bool {
    !value.is_empty()
        && value != "0"
        && !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u32>().is_ok()
}

#[derive(Clone, Copy)]
enum Padding {
    Required,
    Forbidden,
}

fn decode(input: &[u8], alphabet: &[u8; 64], padding: Padding) -> Option<Vec<u8>> {
    let pads = input.iter().rev().take_while(|byte| **byte == b'=').count();
    if pads > 2 || input[..input.len().saturating_sub(pads)].contains(&b'=') {
        return None;
    }
    let significant = input.len().checked_sub(pads)?;
    if significant % 4 == 1 {
        return None;
    }
    match padding {
        Padding::Required
            if !input.len().is_multiple_of(4) || pads != (4 - significant % 4) % 4 =>
        {
            return None;
        }
        Padding::Forbidden if pads != 0 => return None,
        _ => {}
    }
    let mut values = Vec::with_capacity(significant);
    for byte in &input[..significant] {
        values.push(u8::try_from(alphabet.iter().position(|entry| entry == byte)?).ok()?);
    }
    if significant % 4 == 2 && values.last()? & 0x0f != 0
        || significant % 4 == 3 && values.last()? & 0x03 != 0
    {
        return None;
    }
    let mut output = Vec::with_capacity(significant * 3 / 4);
    for chunk in values.chunks(4) {
        if chunk.len() >= 2 {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
        }
        if chunk.len() >= 3 {
            output.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if chunk.len() == 4 {
            output.push((chunk[2] << 6) | chunk[3]);
        }
    }
    Some(output)
}

fn split_exact<'a>(input: &'a [u8], separator: &[u8]) -> Vec<&'a [u8]> {
    let mut result = Vec::new();
    let mut start = 0;
    while let Some(offset) = input[start..]
        .windows(separator.len())
        .position(|window| window == separator)
    {
        let end = start + offset;
        result.push(&input[start..end]);
        start = end + separator.len();
    }
    result.push(&input[start..]);
    result
}
