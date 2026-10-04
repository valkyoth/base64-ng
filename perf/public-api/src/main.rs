#[cfg(feature = "stream")]
mod adapters;
#[path = "../../src/allocation.rs"]
mod allocation;
mod diagnostics;
mod external;
mod operations;
#[path = "../../../src/v2/rfc4648_oracle.rs"]
mod oracle;
#[cfg(test)]
mod tests;

use base64_ng::{Alphabet, Base64, Codec, Engine};
use operations::Work;
use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("list") {
        for name in operations::names() {
            println!("{name}");
        }
        return;
    }
    if args.first().map(String::as_str) == Some("diagnostics") {
        diagnostics::run();
        return;
    }
    assert_eq!(
        args.len(),
        8,
        "usage: OP encode|decode PROFILE SIZE PATTERN FRAGMENT ITERATIONS warm|cold|invalid-position"
    );
    let operation = &args[0];
    let encode = match args[1].as_str() {
        "encode" => true,
        "decode" => false,
        _ => panic!("invalid direction"),
    };
    let size: usize = args[3].parse().unwrap();
    let fragment: usize = args[5].parse().unwrap();
    let iterations: usize = args[6].parse().unwrap();
    assert!(
        size <= 16 * 1024 * 1024
            && fragment > 0
            && fragment <= 16 * 1024 * 1024
            && iterations > 0
            && iterations <= 1_000_000
    );
    assert!(operation != "validate" || !encode);
    macro_rules! run {
        ($codec:expr, $engine:expr, $profile:expr) => {
            measure(
                &$codec, $engine, $profile, operation, encode, size, &args[4], fragment,
                iterations, &args[7],
            )
        };
    }
    match args[2].as_str() {
        "sp" => run!(
            base64_ng::STRICT_STANDARD_PADDED,
            base64_ng::STANDARD,
            oracle::Profile::StandardPadded
        ),
        "su" => run!(
            base64_ng::STRICT_STANDARD_UNPADDED,
            base64_ng::STANDARD_NO_PAD,
            oracle::Profile::StandardUnpadded
        ),
        "up" => run!(
            base64_ng::STRICT_URL_SAFE_PADDED,
            base64_ng::URL_SAFE,
            oracle::Profile::UrlSafePadded
        ),
        "uu" => run!(
            base64_ng::STRICT_URL_SAFE_UNPADDED,
            base64_ng::URL_SAFE_NO_PAD,
            oracle::Profile::UrlSafeUnpadded
        ),
        _ => panic!("invalid profile"),
    }
}

pub fn input(size: usize, pattern: &str) -> Vec<u8> {
    let mut state = 0xa341_316cu32;
    (0..size)
        .map(|index| match pattern {
            "zero" => 0,
            "structured" => index as u8,
            "random" => {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            }
            _ => panic!("invalid pattern"),
        })
        .collect()
}

fn verify(
    result: Result<usize, ()>,
    work: &Work,
    operation: &str,
    encode: bool,
    expected: &[u8],
    invalid: bool,
) {
    if invalid {
        assert!(result.is_err(), "malformed input accepted");
        return;
    }
    let written = result.expect("valid benchmark operation failed");
    if operation == "validate" {
        assert_eq!(written, 0);
    } else {
        assert_eq!(
            work.result(operation, encode, written),
            expected,
            "wrong benchmark output"
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn measure<S: Codec + Copy, A: Alphabet, const PAD: bool>(
    codec: &Base64<S>,
    engine: Engine<A, PAD>,
    profile: oracle::Profile,
    operation: &str,
    encode: bool,
    size: usize,
    pattern: &str,
    fragment: usize,
    iterations: usize,
    mode: &str,
) {
    let raw = input(size, pattern);
    let encoded = oracle::encode(profile, &raw);
    assert_eq!(oracle::decode(profile, &encoded).unwrap(), raw);
    let mut data = if encode { raw.clone() } else { encoded.clone() };
    let expected = if encode { &encoded } else { &raw };
    let invalid = mode.starts_with("invalid-");
    if invalid {
        assert!(!encode && !data.is_empty());
        assert!(
            matches!(operation, "canonical" | "historical" | "validate"),
            "unsupported rejection operation"
        );
        let position: usize = mode.strip_prefix("invalid-").unwrap().parse().unwrap();
        data[position] = b'!';
        assert!(oracle::decode(profile, &data).is_err());
    } else {
        assert!(mode == "warm" || mode == "cold");
    }
    if mode == "cold" {
        assert_eq!(
            iterations, 1,
            "cold measurement is one call in a fresh process"
        );
    }
    let mut work = Work::new(encoded.len() + 64);
    // No backend-report call or codec operation precedes the cold timer.
    if mode != "cold" {
        let result =
            operations::apply(codec, engine, operation, encode, &data, &mut work, fragment);
        verify(result, &work, operation, encode, expected, invalid);
    }
    allocation::reset_allocation_count();
    let mut successes = 0usize;
    let mut last = Err(());
    let start = Instant::now();
    for _ in 0..iterations {
        let result = operations::apply(
            codec,
            engine,
            operation,
            encode,
            black_box(&data),
            black_box(&mut work),
            fragment,
        );
        successes += usize::from(black_box(&result).is_ok());
        last = black_box(result);
    }
    let elapsed = start.elapsed().as_nanos();
    let allocations = allocation::allocation_count();
    assert_eq!(
        successes,
        if invalid { 0 } else { iterations },
        "error-only or unexpectedly successful timed operation"
    );
    verify(last, &work, operation, encode, expected, invalid);
    let result = operations::apply(codec, engine, operation, encode, &data, &mut work, fragment);
    verify(result, &work, operation, encode, expected, invalid);
    let report = base64_ng::runtime::backend_report().snapshot();
    println!(
        "elapsed_ns,iterations,raw_bytes,encoded_bytes,allocations,encode_capability,decode_capability"
    );
    println!(
        "{elapsed},{iterations},{size},{},{allocations},{},{}",
        encoded.len(),
        report.encode_backend.backend,
        report.strict_decode_backend.backend
    );
}
