//! Encode an ordinary file with bounded memory and explicit finalization.
//! Usage: cargo run --example stream_file --features stream -- INPUT OUTPUT
use base64_ng::{STANDARD, stream::Encoder};
use std::{
    fs::File,
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
    // Avoid overwriting the input or an existing destination.
    let output = File::create_new(&args[1])?;
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
