# base64-ng

`base64-ng` is a `no_std`-first Base64 crate focused on correctness, strict decoding, caller-owned buffers, and a security-heavy release process. The long-term goal is to provide modern hardware acceleration without making unsafe SIMD the foundation of trust.

The crate starts conservative: a small scalar implementation, strict RFC 4648 behavior, and a test/release system modeled after hardened Rust service projects. SIMD, streaming, Kani proofs, and fuzzing are planned behind explicit gates.

## Current Status

This repository is at the initial `0.1.0` scaffold stage.

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
- `std::io` streaming encoders and decoders behind the `stream` feature.
- Focused unit and integration tests.
- Local check scripts, release gate, dependency policy, audit config, CI, SBOM script, and reproducible build check.

Planned:

- Constant-time-focused scalar decoder mode.
- Legacy compatibility profile for explicitly non-canonical inputs.
- AVX2, AVX-512, and ARM NEON fast paths.
- Async streaming wrappers.
- Miri, cargo-fuzz, and Kani proof harnesses.
- Criterion benchmarks against the established `base64` crate.

## Install

```toml
[dependencies]
base64-ng = "0.1"
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
| `fuzzing` | no | Future fuzz target support. |

Disable defaults for embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "0.1", default-features = false }
```

## Example

```rust
use base64_ng::{STANDARD, encoded_len};

let input = b"hello";
let mut encoded = [0u8; encoded_len(5, true)];
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
- No unsafe code in scalar code.
- Future unsafe SIMD isolated under `src/simd/`.
- Strict decoding rejects malformed padding and trailing data.
- Alphabet decode uses branch-minimized scalar arithmetic, but strict decoding is not documented as a cryptographic constant-time API.
- Legacy compatibility must be opt-in.
- Release gates include formatting, clippy, tests, Miri when installed, docs, dependency policy, audit, license review, SBOM, and reproducible build checks.
- Future Kani proofs target in-place decoding bounds and scalar decoder invariants.

See [docs/PLAN.md](docs/PLAN.md) and [SECURITY.md](SECURITY.md).

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

Miri is installed as a nightly Rust component, not as a Cargo package:

```sh
rustup toolchain install nightly --component miri
cargo +nightly miri setup
cargo +nightly miri test --no-default-features
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
