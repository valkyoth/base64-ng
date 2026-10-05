//! Encode an ordinary file with bounded memory and explicit finalization.
//! Usage: cargo run --example stream_file --features stream -- INPUT OUTPUT
//! Use a trusted output directory; on Windows, restrict its inherited DACL.
use base64_ng::{STANDARD, stream::Encoder};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, Read},
};

const INPUT_LIMIT: u64 = 64 * 1024 * 1024;

fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected INPUT OUTPUT",
        ));
    }
    let mut input = BufReader::new(File::open(&args[0])?).take(INPUT_LIMIT + 1);
    // Set Unix permissions at creation, without an exposure window before chmod.
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let output = options.open(&args[1])?;
    let mut encoder = Encoder::new(output, STANDARD);
    let copied = io::copy(&mut input, &mut encoder)?;
    if copied > INPUT_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input limit exceeded; discard partial output",
        ));
    }
    encoder.finish()?.sync_all()
}
