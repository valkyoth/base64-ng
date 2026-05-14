# base64-ng

`base64-ng` is a `no_std`-first Base64 crate focused on correctness, strict decoding, caller-owned buffers, and a security-heavy release process. The long-term goal is to provide modern hardware acceleration without making unsafe SIMD the foundation of trust.

The crate starts conservative: a small scalar implementation, strict RFC 4648 behavior, and a test/release system modeled after hardened Rust service projects. Streaming is available behind an explicit feature, fuzz harnesses are isolated from the published crate, and future SIMD and Kani work remain gated until they have evidence.

## Current Status

The current public release is `0.4.1`. The `main` branch now tracks the
`0.5.0-alpha.0` development cycle.

Implemented now:

- `no_std` core with optional `alloc` and `std` features.
- Zero external runtime or development dependencies in `Cargo.toml`.
- Standard and URL-safe alphabets.
- Padded and unpadded encoding into caller-provided output buffers.
- Stable compile-time encoding into caller-sized arrays.
- Strict decoding into caller-provided output buffers.
- In-place encoding when the caller provides enough spare capacity.
- Optional `alloc` vector and string helpers.
- In-place decode API built on the same strict scalar decoder.
- Explicit legacy decode APIs that ignore ASCII transport whitespace while
  keeping alphabet and padding validation strict.
- Separate `ct` scalar decode module for sensitive payloads that avoids
  secret-indexed lookup tables during Base64 symbol mapping.
- `std::io` streaming encoders and decoders behind the `stream` feature.
- Focused unit and integration tests.
- Isolated `cargo-fuzz` harnesses for decode, in-place decode, and stream
  chunk-boundary behavior.
- Local check scripts, release gate, dependency policy, audit config, CI, SBOM script, and reproducible build check.

Planned:

- AVX2, AVX-512, and ARM NEON fast paths.
- Async streaming wrappers.
- Kani proof harnesses.
- Criterion benchmarks against the established `base64` crate.

## Install

```toml
[dependencies]
base64-ng = "0.4.1"
```

The crate is dual-licensed:

```toml
license = "MIT OR Apache-2.0"
```

## Features

| Feature | Default | Purpose |
| --- | --- | --- |
| `alloc` | yes | `Vec` and encoded `String` convenience APIs. |
| `std` | yes | `std::error::Error` support and feature base for I/O. |
| `simd` | no | Future hardware acceleration. |
| `stream` | no | `std::io` streaming wrappers. |
| `tokio` | no | Future async streaming wrappers. |
| `kani` | no | Future verifier harnesses. |
| `fuzzing` | no | Reserved for verifier and fuzz harness integration; published crate stays dependency-free. |

Disable defaults for embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "0.4.1", default-features = false }
```

## Example

```rust
use base64_ng::{STANDARD, checked_encoded_len};

let input = b"hello";
const ENCODED_CAPACITY: usize = match checked_encoded_len(5, true) {
    Some(len) => len,
    None => panic!("encoded length overflow"),
};
let mut encoded = [0u8; ENCODED_CAPACITY];
let written = STANDARD.encode_slice(input, &mut encoded).unwrap();
assert_eq!(&encoded[..written], b"aGVsbG8=");

let mut decoded = [0u8; 5];
let written = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
assert_eq!(&decoded[..written], input);
```

In-place encoding:

```rust
use base64_ng::STANDARD;

let mut buffer = [0u8; 8];
buffer[..5].copy_from_slice(b"hello");
let encoded = STANDARD.encode_in_place(&mut buffer, 5).unwrap();
assert_eq!(encoded, b"aGVsbG8=");
```

For sensitive payloads, `encode_slice_clear_tail` and
`encode_in_place_clear_tail` clear unused bytes after the encoded prefix and
clear the caller-owned output buffer on encode error.

Compile-time encoding:

```rust
use base64_ng::{STANDARD, URL_SAFE_NO_PAD};

const HELLO: [u8; 8] = STANDARD.encode_array(b"hello");
const URL_BYTES: [u8; 3] = URL_SAFE_NO_PAD.encode_array(b"\xfb\xff");

assert_eq!(&HELLO, b"aGVsbG8=");
assert_eq!(&URL_BYTES, b"-_8");
```

Stable Rust cannot yet express the encoded length as the return array length
directly, so `encode_array` uses the destination array type supplied by the
caller. A wrong output length fails during const evaluation.

For untrusted length metadata, use checked length calculation:

```rust
use base64_ng::{checked_encoded_len, decoded_len};

assert_eq!(checked_encoded_len(5, true), Some(8));
assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
```

## Legacy Whitespace Decoding

Strict decoding rejects whitespace. If an existing protocol allows line-wrapped
or spaced Base64, use the explicit legacy APIs:

```rust
use base64_ng::STANDARD;

let mut output = [0u8; 5];
let written = STANDARD
    .decode_slice_legacy(b" aG\r\nVs\tbG8= ", &mut output)
    .unwrap();

assert_eq!(&output[..written], b"hello");
```

Legacy decoding only ignores ASCII space, tab, carriage return, and line feed.
Alphabet selection, padding placement, trailing data after padding, and
non-canonical trailing bits remain strict.

## Bounded Memory Use

For untrusted payloads, size buffers before decoding or encoding. The checked
helpers let callers reject impossible or oversized metadata before allocating:

```rust
use base64_ng::{STANDARD, checked_encoded_len, decoded_capacity};

let input = b"hello";
let encoded_len = checked_encoded_len(input.len(), true).unwrap();
assert_eq!(encoded_len, 8);

let mut encoded = vec![0u8; encoded_len];
let written = STANDARD.encode_slice(input, &mut encoded).unwrap();
encoded.truncate(written);

let max_decoded = decoded_capacity(encoded.len());
let mut decoded = vec![0u8; max_decoded];
let written = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
decoded.truncate(written);

assert_eq!(decoded, input);
```

`decode_vec` validates the complete input before allocating decoded output.
Use `decode_slice` or `decode_in_place` when the caller needs hard memory
limits and owns the output buffer.

For sensitive payloads, use `decode_slice_clear_tail` or
`decode_in_place_clear_tail` to clear unused bytes after the decoded prefix. On
decode error these variants clear the caller-owned output buffer before
returning the error. The legacy whitespace profile also provides
`decode_slice_legacy_clear_tail` and `decode_in_place_legacy_clear_tail`.
The `ct` module provides the same clear-tail decode variants for callers using
the constant-time-oriented scalar decoder.

With the default `alloc` feature, vector and string helpers are available:

```rust
use base64_ng::STANDARD;

let encoded = STANDARD.encode_vec(b"hello").unwrap();
assert_eq!(encoded, b"aGVsbG8=");

let encoded_string = STANDARD.encode_string(b"hello").unwrap();
assert_eq!(encoded_string, "aGVsbG8=");

let decoded = STANDARD.decode_vec(&encoded).unwrap();
assert_eq!(decoded, b"hello");
```

With the `stream` feature, `std::io` encoders are available:

```rust
use std::io::{Read, Write};
use base64_ng::{STANDARD, stream::{Decoder, DecoderReader, Encoder, EncoderReader}};

let mut encoder = Encoder::new(Vec::new(), STANDARD);
encoder.write_all(b"he").unwrap();
encoder.write_all(b"llo").unwrap();
let encoded = encoder.finish().unwrap();
assert_eq!(encoded, b"aGVsbG8=");

let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
let mut encoded = String::new();
reader.read_to_string(&mut encoded).unwrap();
assert_eq!(encoded, "aGVsbG8=");

let mut decoder = Decoder::new(Vec::new(), STANDARD);
decoder.write_all(b"aGVs").unwrap();
decoder.write_all(b"bG8=").unwrap();
let decoded = decoder.finish().unwrap();
assert_eq!(decoded, b"hello");

let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
let mut decoded = Vec::new();
reader.read_to_end(&mut decoded).unwrap();
assert_eq!(decoded, b"hello");
```

URL-safe, no-padding encoding:

```rust
use base64_ng::URL_SAFE_NO_PAD;

let mut encoded = [0u8; 7];
let written = URL_SAFE_NO_PAD.encode_slice(b"hello", &mut encoded).unwrap();
assert_eq!(&encoded[..written], b"aGVsbG8");
```

## Security Model

`base64-ng` treats Base64 as infrastructure code. Fast paths are never allowed to outrun evidence.

Security commitments:

- Stable Rust first. Current toolchain pin: Rust `1.95.0`.
- `no_std` core by default.
- Scalar code denies unsafe.
- Future unsafe SIMD isolated under `src/simd.rs`.
- Local checks verify that `allow(unsafe_code)` is confined to the SIMD
  boundary.
- [docs/UNSAFE.md](docs/UNSAFE.md) inventories every current unsafe site and
  its safety invariants.
- `runtime::backend_report()` exposes the active backend, detected candidate,
  SIMD feature status, and scalar-only security posture for audit logging.
- `runtime::require_backend_policy()` lets deployments assert scalar execution,
  disabled SIMD features, or no detected SIMD candidate.
- `BackendPolicy::HighAssuranceScalarOnly` combines the scalar/no-SIMD
  deployment checks into one assertion.
- Runtime backend, posture, and policy enums expose stable string identifiers
  for CI artifacts, audit logs, and deployment evidence.
- Runtime backend reports and policy failures use stable key/value display
  output for log ingestion.
- Strict decoding rejects malformed padding and trailing data.
- Runtime scalar APIs are expected to return `Result` or `Option` for malformed
  input and size errors instead of panicking.
- Public encoded-length overflow is recoverable through `Result` or `Option`;
  untrusted length metadata should never require a panic.
- Scalar encode avoids input-derived alphabet table indexes, and scalar decode
  uses branch-minimized arithmetic. A separate `ct` module provides a
  constant-time-oriented scalar decode path for callers that need a narrower
  timing target. Its malformed-input errors are intentionally non-localized,
  clear-tail variants clear caller-owned buffers on error, and it is not
  documented as a formally verified cryptographic constant-time API.
- Clear-tail encode/decode variants are available for callers that want
  best-effort cleanup of unused caller-owned buffers without adding a runtime
  dependency.
- Streaming wrappers clear internal pending and queued byte buffers on drop and
  as buffered bytes are consumed, as best-effort retention reduction.
- Legacy compatibility must be opt-in.
- Release gates include formatting, clippy, tests, Miri when installed, docs, dependency policy, audit, license review, isolated fuzz/perf dependency checks, SBOM, and reproducible build checks.
- Future Kani proofs target in-place decoding bounds and scalar decoder invariants.

See [docs/PLAN.md](docs/PLAN.md), [SECURITY.md](SECURITY.md),
[docs/RELEASE_EVIDENCE.md](docs/RELEASE_EVIDENCE.md), and
[docs/CONSTANT_TIME.md](docs/CONSTANT_TIME.md). For the unsafe hardware
acceleration gate, see [docs/SIMD.md](docs/SIMD.md).
For adoption guidance from the established `base64` crate, see
[docs/MIGRATION.md](docs/MIGRATION.md).
For performance evidence guidance, see [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

## Local Checks

Run the standard gate:

```sh
scripts/checks.sh
```

Check the zero-external-crate policy directly:

```sh
scripts/validate-dependencies.sh
```

Run the release gate:

```sh
scripts/stable_release_gate.sh
```

Install cross-compilation targets used by the local and CI target checks:

```sh
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
```

Required security tools:

```sh
cargo install --locked cargo-audit
cargo install --locked cargo-license
cargo install --locked cargo-deny
cargo install --locked cargo-sbom --version 0.10.0
```

Optional deep tools:

```sh
cargo install --locked cargo-nextest
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier
```

Verify optional tool installation:

```sh
cargo nextest --version
cargo fuzz --version
cargo kani --version
```

Compile fuzz targets without running a campaign:

```sh
scripts/check_fuzz.sh
```

Compile and audit the isolated performance harness:

```sh
scripts/check_perf.sh
```

Run the scalar comparison benchmark:

```sh
cargo run --release --manifest-path perf/Cargo.toml
```

Run a target with `cargo-fuzz`:

```sh
cargo +nightly fuzz run decode
cargo +nightly fuzz run in_place
cargo +nightly fuzz run stream_chunks
cargo +nightly fuzz run differential
```

Miri is installed as a nightly Rust component, not as a Cargo package:

```sh
rustup toolchain install nightly --component miri
cargo +nightly miri setup
cargo +nightly miri test --no-default-features
```

Kani may need a one-time setup after installation:

```sh
cargo kani setup
```

On openSUSE Tumbleweed, install `rustup` first if it is not already present:

```sh
sudo zypper install rustup
```

The local release gate runs Miri automatically when `cargo +nightly miri` is
available. The large deterministic sweep tests are ignored only under Miri
because they are already covered by the normal release gate and are too slow for
an interpreter.

## Project Principles

- Keep external crates to the absolute minimum. The current crate dependency graph is only `base64-ng`.
- Correctness first, speed second, unsafe last.
- The scalar implementation is the reference behavior.
- SIMD must prove equivalence to scalar behavior across fuzzed and deterministic inputs.
- Compatibility modes must be visible in the type/API surface.
- Release evidence belongs in the repository and CI, not in memory.

## Contributing And Releases

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution rules and [docs/RELEASE.md](docs/RELEASE.md) for the maintainer release checklist.
