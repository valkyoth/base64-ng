//! Independent MIME body interoperability checks.

#![cfg(feature = "std")]

use std::{
    io::Write,
    process::{Command, Stdio},
};

use base64_ng_mime::{
    MimeBodyDecodePolicy, MimeBodyLimits, MimeBodyTerminalLineEnding,
    decode_mime_content_transfer_body_to_vec, encode_mime_content_transfer_body_to_string,
};

#[test]
fn canonical_output_matches_python_email_and_openssl() {
    for plain in [
        b"f".as_slice(),
        b"interoperable MIME body".as_slice(),
        &[0x5a; 58],
        &[0xa5; 257],
    ] {
        let ours = encode_mime_content_transfer_body_to_string(
            plain,
            MimeBodyLimits::DEFAULT,
            MimeBodyTerminalLineEnding::IncludeCrLf,
        )
        .unwrap();
        assert_eq!(ours.as_bytes(), python_email_encode(plain));

        let openssl = openssl_base64(plain, false);
        assert_eq!(without_line_endings(ours.as_bytes()), openssl);

        let (decoded, _) = decode_mime_content_transfer_body_to_vec(
            ours.as_bytes(),
            MimeBodyDecodePolicy::Rfc2045Compatible,
            MimeBodyLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(decoded, python_email_decode(ours.as_bytes()));
        assert_eq!(decoded, openssl_base64(ours.as_bytes(), true));
    }
}

fn python_email_encode(input: &[u8]) -> Vec<u8> {
    let script = concat!(
        "import email.base64mime,sys; ",
        "sys.stdout.buffer.write(email.base64mime.body_encode(",
        "sys.stdin.buffer.read(), maxlinelen=76, eol='\\r\\n').encode('ascii'))"
    );
    run_filter("python3", &["-c", script], input)
}

fn python_email_decode(input: &[u8]) -> Vec<u8> {
    let script = concat!(
        "import email.base64mime,sys; ",
        "sys.stdout.buffer.write(email.base64mime.decode(sys.stdin.buffer.read()))"
    );
    run_filter("python3", &["-c", script], input)
}

fn openssl_base64(input: &[u8], decode: bool) -> Vec<u8> {
    let arguments: &[&str] = if decode {
        // `-A` declares single-line input and some OpenSSL releases silently
        // produce no output when a valid MIME body contains line wrapping.
        &["base64", "-d"]
    } else {
        &["base64", "-A"]
    };
    run_filter("openssl", arguments, input)
}

fn run_filter(program: &str, arguments: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("could not start {program}: {error}"));
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn without_line_endings(input: &[u8]) -> Vec<u8> {
    input
        .iter()
        .copied()
        .filter(|byte| !matches!(byte, b'\r' | b'\n'))
        .collect()
}
