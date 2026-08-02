use std::hint::black_box;
use std::time::Instant;

use base64_ng::perf_evidence::EvidenceBackend;

use crate::codec::{self, Profile};

const LENGTHS: &[usize] = &[12, 24, 48, 64, 96, 192, 384, 768, 1024, 4096, 64 * 1024];
const DEFAULT_SAMPLES: usize = 15;
const DEFAULT_TARGET_BYTES: usize = 4 * 1024 * 1024;

pub fn run() {
    assert!(
        EvidenceBackend::Neon.is_available(),
        "NEON evidence must run on little-endian AArch64"
    );
    let samples = env_usize("BASE64_NG_PERF_SAMPLES", DEFAULT_SAMPLES);
    let target_bytes = env_usize("BASE64_NG_PERF_TARGET_BYTES", DEFAULT_TARGET_BYTES);
    println!(
        "backend,operation,alphabet,padding,input_len,sample_index,iterations,elapsed_ns,throughput_mib_s"
    );
    for profile in Profile::ALL {
        for &input_len in LENGTHS {
            let raw = codec::make_input(input_len);
            let encoded = codec::canonical_encoded(profile, &raw);
            for backend in [EvidenceBackend::Scalar, EvidenceBackend::Neon] {
                benchmark_encode(backend, profile, &raw, samples, target_bytes);
                benchmark_decode(
                    backend,
                    profile,
                    &encoded,
                    input_len,
                    samples,
                    target_bytes,
                );
            }
        }
    }
}

fn benchmark_encode(
    backend: EvidenceBackend,
    profile: Profile,
    input: &[u8],
    samples: usize,
    target_bytes: usize,
) {
    let iterations = (target_bytes / input.len()).max(1);
    let mut output = vec![0u8; profile.encoded_len(input.len())];
    benchmark(
        backend,
        "encode",
        profile,
        input.len(),
        samples,
        iterations,
        || {
            black_box(codec::encode(
                "base64-ng",
                Some(backend),
                profile,
                black_box(input),
                black_box(&mut output),
            ));
        },
    );
}

fn benchmark_decode(
    backend: EvidenceBackend,
    profile: Profile,
    input: &[u8],
    raw_input_len: usize,
    samples: usize,
    target_bytes: usize,
) {
    let iterations = (target_bytes / raw_input_len).max(1);
    let mut output = vec![0u8; raw_input_len];
    benchmark(
        backend,
        "decode",
        profile,
        raw_input_len,
        samples,
        iterations,
        || {
            black_box(codec::decode(
                "base64-ng",
                Some(backend),
                profile,
                black_box(input),
                black_box(&mut output),
            ));
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn benchmark(
    backend: EvidenceBackend,
    operation: &str,
    profile: Profile,
    input_len: usize,
    samples: usize,
    iterations: usize,
    mut operation_once: impl FnMut(),
) {
    for _ in 0..(iterations / 10).max(1) {
        operation_once();
    }
    for sample_index in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations {
            operation_once();
        }
        let elapsed = start.elapsed();
        let throughput = input_len as f64 * iterations as f64
            / (1024.0 * 1024.0)
            / elapsed.as_secs_f64();
        println!(
            "{},{operation},{},{},{input_len},{sample_index},{iterations},{},{throughput:.6}",
            backend.as_str(),
            profile.alphabet(),
            profile.padding(),
            elapsed.as_nanos(),
        );
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|value| value.parse().expect("performance integer is valid"))
        .unwrap_or(default)
}
