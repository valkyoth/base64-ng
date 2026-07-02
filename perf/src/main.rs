use std::hint::black_box;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose;
use base64_ng::{STANDARD, checked_encoded_len, decoded_capacity};
use base64_ng::runtime::{BackendSnapshot, backend_report};

const CASES: &[usize] = &[1, 2, 3, 32, 1024, 64 * 1024, 1024 * 1024];
const TARGET_BYTES: usize = 64 * 1024 * 1024;

struct BackendContext {
    snapshot: BackendSnapshot,
    active_decode: &'static str,
}

fn main() {
    let report = backend_report();
    let backend = BackendContext {
        snapshot: report.snapshot(),
        active_decode: report.active_decode_backend().as_str(),
    };
    println!(
        "engine,operation,input_len,iterations,elapsed_ms,throughput_mib_s,effective_backend,active_backend,active_decode_backend,candidate_backend,detection_mode,target_arch,target_os"
    );
    for &len in CASES {
        let input = make_input(len);
        let encoded_len = checked_encoded_len(input.len(), true).expect("encoded length fits");
        let iterations = (TARGET_BYTES / input.len()).max(1);

        let mut ng_encoded = vec![0u8; encoded_len];
        let ng_written = STANDARD
            .encode_slice(&input, &mut ng_encoded)
            .expect("base64-ng encode succeeds");
        ng_encoded.truncate(ng_written);

        let mut reference_encoded = vec![0u8; encoded_len];
        let reference_written = general_purpose::STANDARD
            .encode_slice(&input, &mut reference_encoded)
            .expect("base64 encode succeeds");
        reference_encoded.truncate(reference_written);
        assert_eq!(ng_encoded, reference_encoded);

        let mut decoded = vec![0u8; input.len()];
        let decoded_len = STANDARD
            .decode_slice(&ng_encoded, &mut decoded)
            .expect("base64-ng decode succeeds");
        assert_eq!(&decoded[..decoded_len], input.as_slice());

        let mut base64_ng_encode_output = vec![0u8; encoded_len];
        let base64_ng_encode = measure(iterations, input.len(), || {
            let written = STANDARD.encode_slice(
                black_box(&input),
                black_box(&mut base64_ng_encode_output),
            )?;
            black_box(written);
            Ok::<(), base64_ng::EncodeError>(())
        })
        .expect("base64-ng encode benchmark succeeds");

        let mut base64_encode_output = vec![0u8; encoded_len];
        let base64_encode = measure(iterations, input.len(), || {
            let written = general_purpose::STANDARD
                .encode_slice(black_box(&input), black_box(&mut base64_encode_output))
                .expect("base64 encode succeeds");
            black_box(written);
            Ok::<(), ()>(())
        });

        let decode_capacity = decoded_capacity(ng_encoded.len());
        let mut base64_ng_decode_output = vec![0u8; decode_capacity];
        let base64_ng_decode = measure(iterations, input.len(), || {
            let written = STANDARD.decode_slice(
                black_box(&ng_encoded),
                black_box(&mut base64_ng_decode_output),
            )?;
            black_box(written);
            Ok::<(), base64_ng::DecodeError>(())
        })
        .expect("base64-ng decode benchmark succeeds");

        let mut base64_decode_output = vec![0u8; decode_capacity];
        let base64_decode = measure(iterations, input.len(), || {
            let written = general_purpose::STANDARD
                .decode_slice(black_box(&ng_encoded), black_box(&mut base64_decode_output))
                .expect("base64 decode succeeds");
            black_box(written);
            Ok::<(), ()>(())
        });

        print_result(
            &backend,
            "base64-ng",
            "encode",
            effective_backend(&backend, "base64-ng", "encode", len),
            len,
            iterations,
            base64_ng_encode,
        );
        print_result(
            &backend,
            "base64",
            "encode",
            effective_backend(&backend, "base64", "encode", len),
            len,
            iterations,
            base64_encode.expect("base64 encode benchmark succeeds"),
        );
        print_result(
            &backend,
            "base64-ng",
            "decode",
            effective_backend(&backend, "base64-ng", "decode", ng_encoded.len()),
            len,
            iterations,
            base64_ng_decode,
        );
        print_result(
            &backend,
            "base64",
            "decode",
            effective_backend(&backend, "base64", "decode", len),
            len,
            iterations,
            base64_decode.expect("base64 decode benchmark succeeds"),
        );
    }
}

fn make_input(len: usize) -> Vec<u8> {
    let mut output = vec![0u8; len];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = index.wrapping_mul(37).wrapping_add(len) as u8;
    }
    output
}

fn measure<E>(
    iterations: usize,
    input_len: usize,
    mut operation: impl FnMut() -> Result<(), E>,
) -> Result<Duration, E> {
    let start = Instant::now();
    for _ in 0..iterations {
        operation()?;
    }
    let elapsed = start.elapsed();
    black_box(input_len);
    Ok(elapsed)
}

fn print_result(
    backend: &BackendContext,
    engine: &str,
    operation: &str,
    effective_backend: &str,
    len: usize,
    iterations: usize,
    elapsed: Duration,
) {
    let mib = len as f64 * iterations as f64 / 1024.0 / 1024.0;
    let seconds = elapsed.as_secs_f64();
    let throughput = if seconds == 0.0 {
        f64::INFINITY
    } else {
        mib / seconds
    };
    println!(
        "{engine},{operation},{len},{iterations},{:.3},{throughput:.2},{effective_backend},{},{},{},{},{},{}",
        elapsed.as_secs_f64() * 1000.0,
        backend.snapshot.active,
        backend.active_decode,
        backend.snapshot.candidate,
        backend.snapshot.candidate_detection_mode,
        std::env::consts::ARCH,
        std::env::consts::OS,
    );
}

fn effective_backend(
    backend: &BackendContext,
    engine: &str,
    operation: &str,
    operation_input_len: usize,
) -> &'static str {
    if engine != "base64-ng" {
        return "external";
    }

    if operation == "decode" {
        return match backend.active_decode {
            "avx512-vbmi" if operation_input_len >= 64 => "avx512-vbmi",
            "avx512-vbmi" | "avx2" if operation_input_len >= 32 => "avx2",
            "avx512-vbmi" | "avx2" | "ssse3-sse4.1" if operation_input_len >= 16 => {
                "ssse3-sse4.1"
            }
            "neon" if operation_input_len >= 16 => "neon",
            _ => "scalar",
        };
    }

    match backend.snapshot.active {
        "avx512-vbmi" if operation_input_len >= 48 => "avx512-vbmi",
        "avx512-vbmi" | "avx2" if operation_input_len >= 24 => "avx2",
        "avx512-vbmi" | "avx2" | "ssse3-sse4.1" if operation_input_len >= 12 => {
            "ssse3-sse4.1"
        }
        "neon" if operation_input_len >= 12 => "neon",
        _ => "scalar",
    }
}
