mod allocation;
mod codec;
mod evidence;
mod resources;

use std::hint::black_box;
use std::time::Instant;

use allocation::{allocation_count, reset_allocation_count};
use base64_ng::perf_evidence::EvidenceBackend;
use base64_ng::runtime::backend_report;
use codec::{Profile, decode, encode};

const CASES: &[usize] = &[
    1,
    2,
    3,
    11,
    12,
    15,
    16,
    23,
    24,
    31,
    32,
    47,
    48,
    63,
    64,
    1024,
    64 * 1024,
];
const DEFAULT_SAMPLES: usize = 5;
const DEFAULT_TARGET_BYTES: usize = 4 * 1024 * 1024;
const SCHEMA_VERSION: &str = "1";

#[derive(Clone, Copy)]
enum Engine {
    Base64Ng(EvidenceBackend),
    Base64,
    Base64Ct,
}

impl Engine {
    fn name(self) -> &'static str {
        match self {
            Self::Base64Ng(_) => "base64-ng",
            Self::Base64 => "base64-0.23.0",
            Self::Base64Ct => "base64ct-1.8.3",
        }
    }

    fn backend(self) -> &'static str {
        match self {
            Self::Base64Ng(backend) => backend.as_str(),
            Self::Base64 | Self::Base64Ct => "external",
        }
    }
}

struct Config {
    campaign_id: String,
    run_id: String,
    samples: usize,
    target_bytes: usize,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("availability") => print_availability(),
        Some("resources") => resources::print(),
        Some("correctness") => verify_correctness(),
        None | Some("benchmark") => run_benchmark(),
        Some(mode) => panic!("unknown performance harness mode: {mode}"),
    }
}

fn run_benchmark() {
    let config = Config {
        campaign_id: evidence::env_id("BASE64_NG_PERF_CAMPAIGN_ID", "manual"),
        run_id: evidence::env_id("BASE64_NG_PERF_RUN_ID", "run-1"),
        samples: env_usize("BASE64_NG_PERF_SAMPLES", DEFAULT_SAMPLES),
        target_bytes: env_usize("BASE64_NG_PERF_TARGET_BYTES", DEFAULT_TARGET_BYTES),
    };
    assert!(config.samples > 0, "sample count must be non-zero");
    assert!(config.target_bytes > 0, "target bytes must be non-zero");

    verify_correctness();
    let report = backend_report();
    let snapshot = report.snapshot();
    println!(
        "schema_version,campaign_id,run_id,sample_index,engine,operation,alphabet,padding,input_len,encoded_len,iterations,elapsed_ns,throughput_mib_s,backend,active_encode_backend,active_decode_backend,target_arch,target_os,allocation_count"
    );

    for profile in Profile::ALL {
        for &input_len in CASES {
            let input = codec::make_input(input_len);
            let encoded = codec::canonical_encoded(profile, &input);
            for engine in engines() {
                benchmark_operation(
                    &config,
                    profile,
                    engine,
                    "encode",
                    &input,
                    input_len,
                    snapshot.active,
                    report.active_decode_backend().as_str(),
                );
                benchmark_operation(
                    &config,
                    profile,
                    engine,
                    "decode",
                    &encoded,
                    input_len,
                    snapshot.active,
                    report.active_decode_backend().as_str(),
                );
            }
        }
    }
    verify_correctness();
}

#[allow(clippy::too_many_arguments)]
fn benchmark_operation(
    config: &Config,
    profile: Profile,
    engine: Engine,
    operation: &'static str,
    operation_input: &[u8],
    raw_input_len: usize,
    active_encode: &'static str,
    active_decode: &'static str,
) {
    let iterations = (config.target_bytes / raw_input_len.max(1)).max(1);
    let output_len = if operation == "encode" {
        profile.encoded_len(raw_input_len)
    } else {
        raw_input_len
    };
    let mut output = vec![0u8; output_len];

    run_once(engine, profile, operation, operation_input, &mut output);
    reset_allocation_count();
    run_once(engine, profile, operation, operation_input, &mut output);
    let allocations = allocation_count();

    let warmup = (iterations / 10).max(1);
    for _ in 0..warmup {
        run_once(engine, profile, operation, operation_input, &mut output);
    }

    for sample_index in 0..config.samples {
        let start = Instant::now();
        for _ in 0..iterations {
            run_once(
                engine,
                profile,
                operation,
                black_box(operation_input),
                black_box(&mut output),
            );
        }
        let elapsed = start.elapsed();
        let elapsed_ns = elapsed.as_nanos();
        let throughput =
            raw_input_len as f64 * iterations as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64();
        println!(
            "{SCHEMA_VERSION},{},{},{sample_index},{},{operation},{},{},{raw_input_len},{},{iterations},{elapsed_ns},{throughput:.6},{},{active_encode},{active_decode},{},{},{allocations}",
            config.campaign_id,
            config.run_id,
            engine.name(),
            profile.alphabet(),
            profile.padding(),
            profile.encoded_len(raw_input_len),
            engine.backend(),
            std::env::consts::ARCH,
            std::env::consts::OS,
        );
    }
}

fn run_once(engine: Engine, profile: Profile, operation: &str, input: &[u8], output: &mut [u8]) {
    let written = if operation == "encode" {
        encode(engine.name(), backend(engine), profile, input, output)
    } else {
        decode(engine.name(), backend(engine), profile, input, output)
    };
    black_box(written);
}

fn backend(engine: Engine) -> Option<EvidenceBackend> {
    match engine {
        Engine::Base64Ng(backend) => Some(backend),
        Engine::Base64 | Engine::Base64Ct => None,
    }
}

fn engines() -> Vec<Engine> {
    let mut engines = vec![Engine::Base64Ng(EvidenceBackend::Auto)];
    for backend in EvidenceBackend::ALL {
        if backend != EvidenceBackend::Auto && backend.is_available() {
            engines.push(Engine::Base64Ng(backend));
        }
    }
    engines.push(Engine::Base64);
    engines.push(Engine::Base64Ct);
    engines
}

fn print_availability() {
    println!("schema_version,backend,available,target_arch,target_os");
    for backend in EvidenceBackend::ALL {
        println!(
            "{SCHEMA_VERSION},{},{},{},{}",
            backend.as_str(),
            backend.is_available(),
            std::env::consts::ARCH,
            std::env::consts::OS
        );
    }
}

fn verify_correctness() {
    for profile in Profile::ALL {
        for &len in CASES {
            let input = codec::make_input(len);
            let expected = codec::canonical_encoded(profile, &input);
            for engine in engines() {
                let mut encoded = vec![0u8; expected.len()];
                let written = encode(
                    engine.name(),
                    backend(engine),
                    profile,
                    &input,
                    &mut encoded,
                );
                assert_eq!(&encoded[..written], expected);

                let mut decoded = vec![0u8; input.len()];
                let written = decode(
                    engine.name(),
                    backend(engine),
                    profile,
                    &expected,
                    &mut decoded,
                );
                assert_eq!(&decoded[..written], input);
            }
        }
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|value| value.parse().expect("performance integer is valid"))
        .unwrap_or(default)
}
