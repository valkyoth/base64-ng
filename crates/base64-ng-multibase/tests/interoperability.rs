//! Independent Python interoperability checks.

#![cfg(feature = "std")]

use std::process::Command;

use base64_ng_multibase::{
    Base64MultibaseEncoding, Base64MultibaseLimits, decode_base64_multibase_to_vec,
    encode_base64_multibase_to_string,
};

const LIMITS: Base64MultibaseLimits = Base64MultibaseLimits::new(4_096, 4_096, 4_096);

#[test]
fn python_base64_agrees_with_every_admitted_prefix() {
    let mut payload = [0u8; 129];
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte = index.to_le_bytes()[0].wrapping_mul(131).wrapping_add(17);
    }
    for len in [0, 1, 2, 3, 31, 32, 63, 64, 127, 128, 129] {
        let expected = python_encodings(&payload[..len]);
        for (index, encoding) in Base64MultibaseEncoding::ALL.into_iter().enumerate() {
            let encoded =
                encode_base64_multibase_to_string(encoding, &payload[..len], LIMITS).unwrap();
            assert_eq!(encoded, expected[index]);
            let decoded = decode_base64_multibase_to_vec(encoded.as_bytes(), LIMITS).unwrap();
            assert_eq!(decoded.as_bytes(), &payload[..len]);
        }
    }
}

fn python_encodings(input: &[u8]) -> [String; 4] {
    let script = concat!(
        "import base64,sys\n",
        "x=bytes.fromhex(sys.argv[1])\n",
        "s=base64.b64encode(x).decode('ascii')\n",
        "u=base64.urlsafe_b64encode(x).decode('ascii')\n",
        "print('m'+s.rstrip('='))\n",
        "print('M'+s)\n",
        "print('u'+u.rstrip('='))\n",
        "print('U'+u)\n",
    );
    let output = Command::new("python3")
        .args(["-c", script, &hex(input)])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let mut lines = text.lines();
    core::array::from_fn(|_| lines.next().unwrap().to_owned())
}

fn hex(input: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}
