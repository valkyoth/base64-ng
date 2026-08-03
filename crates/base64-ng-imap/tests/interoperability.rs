//! Differential checks against Python Base64 and established IMAP iconv.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use base64_ng_imap::{
    ImapPayloadLimits, decode_modified_utf7_payload_to_vec, encode_modified_utf7_payload_to_string,
};

const LIMITS: ImapPayloadLimits = ImapPayloadLimits::new(16_384, 32_768, 16_384);

#[test]
fn payload_transform_matches_python_standard_base64_mapping() {
    let mut input = vec![0u8; 514];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(101).wrapping_add(7);
    }
    for length in (0..=input.len()).step_by(2) {
        let expected = python_encode(&input[..length]);
        let encoded = encode_modified_utf7_payload_to_string(&input[..length], LIMITS).unwrap();
        assert_eq!(encoded.as_bytes(), expected);
        let decoded = decode_modified_utf7_payload_to_vec(encoded.as_bytes(), LIMITS).unwrap();
        assert_eq!(decoded, input[..length]);
    }
}

#[test]
fn rfc_mailbox_vector_matches_iconv_when_available() {
    let listing = Command::new("iconv").arg("-l").output();
    let Ok(listing) = listing else { return };
    let names = String::from_utf8_lossy(&listing.stdout).to_ascii_uppercase();
    if !names.contains("UTF-7-IMAP") {
        return;
    }

    let mut child = Command::new("iconv")
        .args(["-f", "UTF-8", "-t", "UTF-7-IMAP"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all("~peter/mail/台北/日本語".as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"~peter/mail/&U,BTFw-/&ZeVnLIqe-");
}

fn python_encode(input: &[u8]) -> Vec<u8> {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import base64,sys;sys.stdout.buffer.write(base64.b64encode(sys.stdin.buffer.read()).rstrip(b'=').replace(b'/',b','))",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    output.stdout
}
