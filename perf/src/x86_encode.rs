use std::hint::black_box;
use std::time::Instant;

use base64_ng::perf_evidence::EvidenceBackend;

use crate::codec::{self, Profile};

const LENGTHS: &[usize] = &[12, 24, 48, 64, 96, 192, 384, 768, 1024, 64 * 1024];
const DEFAULT_SAMPLES: usize = 7;
const DEFAULT_TARGET_BYTES: usize = 4 * 1024 * 1024;

pub fn run() {
    let samples = env_usize("BASE64_NG_PERF_SAMPLES", DEFAULT_SAMPLES);
    let target_bytes = env_usize("BASE64_NG_PERF_TARGET_BYTES", DEFAULT_TARGET_BYTES);
    assert!(samples > 0, "sample count must be non-zero");
    assert!(target_bytes > 0, "target bytes must be non-zero");
    println!(
        "backend,alphabet,padding,input_len,sample_index,iterations,elapsed_ns,throughput_mib_s"
    );
    for profile in Profile::ALL {
        for &input_len in LENGTHS {
            let input = codec::make_input(input_len);
            for backend in [
                EvidenceBackend::Scalar,
                EvidenceBackend::Ssse3Sse41,
                EvidenceBackend::Avx2,
                EvidenceBackend::Avx512Vbmi,
            ] {
                if !backend.is_available() || input_len < minimum_input(backend) {
                    continue;
                }
                benchmark(
                    backend,
                    profile,
                    &input,
                    samples,
                    target_bytes,
                );
            }
        }
    }
}

fn benchmark(
    backend: EvidenceBackend,
    profile: Profile,
    input: &[u8],
    samples: usize,
    target_bytes: usize,
) {
    let iterations = (target_bytes / input.len()).max(1);
    let mut output = vec![0u8; profile.encoded_len(input.len())];
    for _ in 0..(iterations / 10).max(1) {
        encode_once(backend, profile, input, &mut output);
    }
    for sample_index in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations {
            encode_once(
                backend,
                profile,
                black_box(input),
                black_box(&mut output),
            );
        }
        let elapsed = start.elapsed();
        let throughput = input.len() as f64 * iterations as f64
            / (1024.0 * 1024.0)
            / elapsed.as_secs_f64();
        println!(
            "{},{},{},{},{sample_index},{iterations},{},{throughput:.6}",
            backend.as_str(),
            profile.alphabet(),
            profile.padding(),
            input.len(),
            elapsed.as_nanos(),
        );
    }
}

fn encode_once(backend: EvidenceBackend, profile: Profile, input: &[u8], output: &mut [u8]) {
    black_box(codec::encode(
        "base64-ng",
        Some(backend),
        profile,
        input,
        output,
    ));
}

const fn minimum_input(backend: EvidenceBackend) -> usize {
    match backend {
        EvidenceBackend::Avx512Vbmi => 48,
        EvidenceBackend::Avx2 => 24,
        EvidenceBackend::Ssse3Sse41 => 12,
        _ => 0,
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .map(|value| value.parse().expect("performance integer is valid"))
        .unwrap_or(default)
}
