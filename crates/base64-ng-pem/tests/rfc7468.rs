//! Locked RFC 7468 vectors and established-tool interoperability.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use base64_ng_pem::{
    PemGenerationOptions, PemLabel, PemLimits, PemParsePolicy, encode_pem_block_to_string,
    parse_pem_document,
};

const RFC_CERTIFICATE: &[u8] = include_bytes!("fixtures/rfc7468-certificate.pem");

#[test]
fn locked_rfc7468_certificate_example_parses_and_regenerates() {
    let parsed = parse_pem_document(
        RFC_CERTIFICATE,
        PemLimits::default(),
        PemParsePolicy::Rfc7468Compatible,
    )
    .unwrap();
    assert_eq!(parsed.blocks().len(), 1);
    assert_eq!(parsed.blocks()[0].label().as_str(), "CERTIFICATE");
    assert_eq!(parsed.blocks()[0].contents()[0], 0x30);
    assert_eq!(parsed.blocks()[0].contents().len(), 560);

    let generated = encode_pem_block_to_string(
        parsed.blocks()[0].label(),
        parsed.blocks()[0].contents(),
        PemLimits::default(),
        PemGenerationOptions::default(),
    )
    .unwrap();
    let strict = parse_pem_document(
        generated.as_bytes(),
        PemLimits::default(),
        PemParsePolicy::Strict,
    )
    .unwrap();
    assert_eq!(strict.blocks()[0].contents(), parsed.blocks()[0].contents());
}

#[test]
fn python_ssl_generation_and_parsing_agree() {
    if Command::new("python3")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        return;
    }
    let directory = temporary_directory("python");
    fs::create_dir_all(&directory).unwrap();
    let der = directory.join("input.der");
    let pem = directory.join("output.pem");
    let payload = [0x30, 0x03, 0x02, 0x01, 0x05];
    fs::write(&der, payload).unwrap();
    let script = "import pathlib,ssl,sys; pathlib.Path(sys.argv[2]).write_text(ssl.DER_cert_to_PEM_cert(pathlib.Path(sys.argv[1]).read_bytes()), encoding='ascii')";
    let status = Command::new("python3")
        .args(["-c", script])
        .arg(&der)
        .arg(&pem)
        .status()
        .unwrap();
    assert!(status.success());
    let python_pem = fs::read(&pem).unwrap();
    let parsed = parse_pem_document(
        &python_pem,
        PemLimits::default(),
        PemParsePolicy::Rfc7468Compatible,
    )
    .unwrap();
    assert_eq!(parsed.blocks()[0].contents(), payload);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn openssl_accepts_generated_asn1_pem() {
    if Command::new("openssl")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        return;
    }
    let directory = temporary_directory("openssl");
    fs::create_dir_all(&directory).unwrap();
    let pem = directory.join("asn1.pem");
    let generated = encode_pem_block_to_string(
        &PemLabel::new("CERTIFICATE").unwrap(),
        &[0x30, 0x03, 0x02, 0x01, 0x05],
        PemLimits::default(),
        PemGenerationOptions::default(),
    )
    .unwrap();
    fs::write(&pem, generated).unwrap();
    let output = Command::new("openssl")
        .args(["asn1parse", "-inform", "PEM", "-in"])
        .arg(&pem)
        .arg("-noout")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "openssl stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(directory);
}

fn temporary_directory(tool: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "base64-ng-pem-{tool}-{}-{nonce}",
        std::process::id()
    ))
}
