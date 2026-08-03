#![allow(missing_docs)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use base64_ng_openpgp::{ChecksumPolicy, OpenPgpLimits, parse_armor_document};

const RFC9580_INLINE_SIGNED: &[u8] = include_bytes!("fixtures/rfc9580-inline-signed.asc");

#[test]
fn rfc9580_fixture_matches_gnupg_and_sequoia() {
    let required = env::var_os("BASE64_NG_REQUIRE_OPENPGP_INTEROP").is_some();
    let expected = parse_armor_document(
        RFC9580_INLINE_SIGNED,
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap()
    .into_blocks()
    .remove(0)
    .into_contents();
    let directory = temporary_directory();
    fs::create_dir(&directory).unwrap();
    let input = directory.join("rfc9580.asc");
    fs::write(&input, RFC9580_INLINE_SIGNED).unwrap();

    let gpg = env::var("GPG").unwrap_or_else(|_| "gpg".into());
    let gpg_output = directory.join("gnupg.bin");
    match Command::new(&gpg)
        .args(["--batch", "--yes", "--dearmor", "--output"])
        .arg(&gpg_output)
        .arg(&input)
        .status()
    {
        Ok(status) if status.success() => assert_eq!(fs::read(&gpg_output).unwrap(), expected),
        Ok(status) => panic!("GnuPG dearmor failed with {status}"),
        Err(error) if required => panic!("required GnuPG is unavailable: {error}"),
        Err(_) => eprintln!("OpenPGP interoperability: skipping unavailable GnuPG"),
    }

    let sq = env::var("SQ").unwrap_or_else(|_| "sq".into());
    let sq_output = directory.join("sequoia.bin");
    match run_sq(&sq, &input, &sq_output) {
        Ok(status) if status.success() => assert_eq!(fs::read(&sq_output).unwrap(), expected),
        Ok(status) => panic!("Sequoia sq dearmor failed with {status}"),
        Err(error) if required => panic!("required Sequoia sq is unavailable: {error}"),
        Err(_) => eprintln!("OpenPGP interoperability: skipping unavailable Sequoia sq"),
    }

    fs::remove_dir_all(directory).unwrap();
}

fn run_sq(sq: &str, input: &Path, output: &Path) -> std::io::Result<ExitStatus> {
    let modern = Command::new(sq)
        .args(["packet", "dearmor", "--output"])
        .arg(output)
        .arg(input)
        .status()?;
    if modern.success() {
        return Ok(modern);
    }
    let _ = fs::remove_file(output);
    Command::new(sq)
        .args(["dearmor", "--output"])
        .arg(output)
        .arg(input)
        .status()
}

fn temporary_directory() -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "base64-ng-openpgp-interop-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    path
}
