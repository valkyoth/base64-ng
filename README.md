# base64-ng

`base64-ng` is a `no_std`-first Base64 crate focused on correctness, strict decoding, caller-owned buffers, and a security-heavy release process. The long-term goal is to provide modern hardware acceleration without making unsafe SIMD the foundation of trust.

The crate starts conservative: a small scalar implementation, strict RFC 4648 behavior, and a test/release system modeled after hardened Rust service projects. SIMD, streaming, Kani proofs, and fuzzing are planned behind explicit gates.

## Current Status

This repository is at the initial `0.1.0` scaffold stage.

Implemented now:

- `no_std` core with optional `alloc` and `std` features.
- Standard and URL-safe alphabets.
- Padded and unpadded encoding into caller-provided output buffers.
- Strict decoding into caller-provided output buffers.
- In-place decode API built on the same strict scalar decoder.
- Focused unit and integration tests.
- Local check scripts, release gate, dependency policy, audit config, CI, SBOM script, and reproducible build check.

Planned:

- Constant-time-focused scalar decoder mode.
- Legacy compatibility profile for explicitly non-canonical inputs.
- AVX2, AVX-512, and ARM NEON fast paths.
- Sync and async streaming wrappers.
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
| `alloc` | yes | Future `Vec`/`String` convenience APIs. |
| `std` | yes | Future `std::error::Error` and I/O support. |
| `simd` | no | Future hardware acceleration. |
| `stream` | no | Future sync streaming wrappers. |
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
- Legacy compatibility must be opt-in.
- Release gates include formatting, clippy, tests, docs, dependency policy, audit, license review, SBOM, and reproducible build checks.
- Future Kani proofs target in-place decoding bounds and scalar decoder invariants.

See [docs/PLAN.md](docs/PLAN.md) and [SECURITY.md](SECURITY.md).

## Local Checks

Run the standard gate:

```sh
scripts/checks.sh
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

## Project Principles

- Correctness first, speed second, unsafe last.
- The scalar implementation is the reference behavior.
- SIMD must prove equivalence to scalar behavior across fuzzed and deterministic inputs.
- Compatibility modes must be visible in the type/API surface.
- Release evidence belongs in the repository and CI, not in memory.
