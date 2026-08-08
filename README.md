<p align="center">
  <b>Secure, no_std-first Base64 for Rust.</b><br>
  Strict RFC 4648 codecs, caller-owned buffers, optional SIMD, and zero core dependencies.
</p>

<div align="center">
  <a href="https://docs.rs/base64-ng">API docs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/MIGRATION.md">Migration</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/TRUST.md">Trust</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/SECURITY.md">Security</a>
</div>

<br>

<p align="center">
  <img src="https://raw.githubusercontent.com/valkyoth/base64-ng/main/.github/images/base64-ng.webp" alt="base64-ng Rust crate overview">
</p>

# base64-ng

`base64-ng` provides strict RFC 4648 Base64, `no_std` operation, caller-owned
and allocating APIs, incremental and in-place transforms, optional admitted
SIMD backends, and separately named compatibility policies.
Zero external runtime or development dependencies in `Cargo.toml`.

This source tree defines the synchronized `2.0.1` package family. This patch
reduces published package and README size. Runtime behavior
and the public 2.0 API remain unchanged from `2.0.0`; repository-only release
scripts, evidence, API snapshots, and engineering ledgers remain available on
GitHub instead of being copied into the crates.io archive.

## Quick Start

```toml
[dependencies]
base64-ng = "2.0.1"
```

For ordinary Standard Base64 with canonical padding:

```rust
let encoded = base64_ng::encode(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");

let decoded = base64_ng::decode(encoded.as_bytes()).unwrap();
assert_eq!(decoded, b"hello");
```

Use an explicit preset when the alphabet and padding policy should be visible:

```rust
use base64_ng::STRICT_URL_SAFE_UNPADDED;

let encoded = STRICT_URL_SAFE_UNPADDED
    .encode_to_string(b"hello")
    .unwrap();
assert_eq!(encoded, "aGVsbG8");
assert_eq!(
    STRICT_URL_SAFE_UNPADDED.decode_to_vec(encoded.as_bytes()).unwrap(),
    b"hello"
);
```

The strict presets are:

- `STRICT_STANDARD_PADDED`
- `STRICT_STANDARD_UNPADDED`
- `STRICT_URL_SAFE_PADDED`
- `STRICT_URL_SAFE_UNPADDED`

Strict decoders reject whitespace, mixed alphabets, impossible lengths,
malformed padding, trailing data after padding, and non-canonical unused bits.
WHATWG-forgiving, legacy-whitespace, wrapped, MIME, PEM, and other protocol
behavior is available only through explicitly named APIs or companion crates.

## Choosing An API

| Need | API |
| --- | --- |
| Ordinary owned Standard Base64 | `encode` and `decode` |
| Explicit alphabet and padding | A `STRICT_*` preset |
| Transactional caller-owned output | `encode_into` and `decode_into` |
| Heapless incremental processing | `encoder()` and `decoder()` |
| In-place transformation | `encode_in_place` and `decode_in_place` |
| Policy-carrying encoded text | `Base64String<S>` |
| Compile-time fixed data | `encode_array` and `decode_array` |
| Fixed-capacity runtime output | `encode_bounded` and `decode_bounded` |
| Secret-bearing frames | `secret::{SecretArrayFrame, SecretVecFrame}` |
| Synchronous I/O | Enable `stream` |
| Async I/O | `base64-ng-tokio` |
| Serde fields | `base64-ng-serde` |

`base64_ng::prelude` contains the focused ordinary API imports. It deliberately
does not import secret, compatibility, protocol, or historical surfaces.

## Caller-Owned Buffers

One-shot `*_into` operations are transactional: an error leaves the complete
destination unchanged.

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let mut encoded = [0u8; 8];
let encoded_len = STRICT_STANDARD_PADDED
    .encode_into(b"hello", &mut encoded)
    .unwrap();
assert_eq!(&encoded[..encoded_len], b"aGVsbG8=");

let mut decoded = [0u8; 5];
let decoded_len = STRICT_STANDARD_PADDED
    .decode_into(&encoded[..encoded_len], &mut decoded)
    .unwrap();
assert_eq!(&decoded[..decoded_len], b"hello");
```

For in-place encoding, reserve the encoded capacity first:

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let mut buffer = [0u8; 8];
buffer[..5].copy_from_slice(b"hello");
let written = STRICT_STANDARD_PADDED
    .encode_in_place(&mut buffer, 5)
    .unwrap();
assert_eq!(&buffer[..written], b"aGVsbG8=");
```

## Incremental Processing

Incremental states retain at most partial Base64 quanta and report exact input
and output progress. Call `finish` to validate or emit the final tail.

```rust
use base64_ng::{Status, STRICT_STANDARD_PADDED};

let mut encoder = STRICT_STANDARD_PADDED.encoder();
let mut output = [0u8; 8];
let step = encoder.update(b"hello", &mut output).unwrap();
let mut written = step.progress().output_produced();
let final_step = encoder.finish(&mut output[written..]).unwrap();
written += final_step.progress().output_produced();

assert_eq!(final_step.status(), Status::Complete);
assert_eq!(&output[..written], b"aGVsbG8=");
```

The decoder uses the same lifecycle and progress contract. See the
[incremental encoder](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_INCREMENTAL_ENCODER.md)
and [decoder finalization](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_INCREMENTAL_DECODER_FINALIZATION.md)
guides for fragmented-input examples.

## Encoded Strings And Formatting

`Base64String<S>` stores validated ordinary Base64 together with its exact
codec policy:

```rust
use base64_ng::{Base64String, STRICT_STANDARD_PADDED};

let stored = Base64String::encode(STRICT_STANDARD_PADDED, b"hello").unwrap();
assert_eq!(stored.as_str(), "aGVsbG8=");
assert_eq!(stored.decode().unwrap(), b"hello");
```

It is printable and non-wiping; it is not a secret container. Allocation-free
`display`, rollback-safe `encode_append`, and `encoded_chunks` are available
from the same strict presets.

## Validation And Compatibility

Validate without producing decoded output:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};

STRICT_STANDARD_PADDED.validate(b"aGVsbG8=").unwrap();
assert!(STRICT_STANDARD_PADDED.validate(b"aGVsbG8").is_err());
STRICT_URL_SAFE_UNPADDED.validate(b"-_8").unwrap();
```

Exact WHATWG forgiving decode is separate from strict RFC 4648:

```rust
use base64_ng::{web, STRICT_STANDARD_PADDED};

assert_eq!(web::FORGIVING.decode_to_vec(" Z h = = ").unwrap(), b"f");
assert!(STRICT_STANDARD_PADDED.decode_to_vec(b" Z h = = ").is_err());
```

Legacy ASCII-whitespace decode is also explicit:

```rust
use base64_ng::{legacy, STRICT_STANDARD_PADDED};

let mut output = [0u8; 5];
let written = legacy::ASCII_WHITESPACE
    .decode_into(&STRICT_STANDARD_PADDED, b" aG\r\nVs\tbG8= ", &mut output)
    .unwrap();
assert_eq!(&output[..written], b"hello");
```

Forgiving and legacy policies are ordinary-data APIs and are unavailable to
secret frames.

## Features

| Feature | Default | Purpose |
| --- | --- | --- |
| `alloc` | yes | `Vec`, `String`, and owned convenience APIs |
| `std` | yes | Standard error and runtime support |
| `simd` | no | Admitted Standard/URL-safe encode and strict-decode acceleration |
| `stream` | no | Synchronous `std::io` adapters |
| `secrets` | no | Bounded secret storage, fixed-work transforms, and explicit exposure |
| `checked-backend` | no | SIMD plus bounded scalar verification, quarantine, and scalar retry |
| `allow-wasm32-best-effort-wipe` | no | Acknowledge wasm cleanup limitations for `secrets` builds |
| `allow-compiler-fence-only-wipe` | no | Acknowledge unsupported-native cleanup limitations |
| `tokio` | no | Reserved, currently inert and dependency-free; use `base64-ng-tokio` |
| `kani` | no | Reserved verifier integration |
| `fuzzing` | no | Reserved fuzz integration |

Disable defaults for core-only embedded use:

```toml
[dependencies]
base64-ng = { version = "2.0.1", default-features = false }
```

Enable ordinary SIMD dispatch without changing the public codec API:

```toml
[dependencies]
base64-ng = { version = "2.0.1", features = ["simd"] }
```

Scalar by default; std x86/x86_64 encode selects SSSE3/SSE4.1, AVX2, or AVX-512 VBMI by length, strict decode selects SSSE3/SSE4.1 or AVX2. Admitted
little-endian AArch64 NEON, wasm `simd128`, and exact-profile Linux/SpacemiT
X60 RVV paths are selected only inside their documented scopes. Unsupported
CPUs, custom alphabets, compatibility policies, and secret operations retain
scalar behavior.

Runtime selection can be inspected with `runtime::backend_report()`.

## Secret-Bearing Data

Ordinary strict decoding returns detailed errors and may exit early. For keys,
tokens, passwords, or other secret-bearing frames, enable `secrets` and use the
bounded fixed-work API:

```toml
[dependencies]
base64-ng = { version = "2.0.1", default-features = false, features = ["secrets"] }
```

```rust
use base64_ng::{secret::{SecretArrayFrame, SecretInput}, STRICT_STANDARD_PADDED};

let mut frame = SecretArrayFrame::<5>::new(&STRICT_STANDARD_PADDED).unwrap();
frame.update(&SecretInput::new(b"aGVsbG8=")).unwrap();
let secret = frame.finish().unwrap();
assert_eq!(secret.expose_secret().as_bytes(), b"hello");
```

This is a constant-time-oriented scalar boundary with bounded private staging,
opaque validity results, and best-effort cleanup. It has project timing,
assembly, Kani, Miri, and test evidence, but no formal cryptographic guarantee.
It must not be documented as a formally verified cryptographic constant-time API.
Protected-memory claims additionally require an admitted provider and runtime
assurance token. Read the [secret decoding](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_SECRET_DECODING.md)
and [assurance](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md)
guides before deployment.

## Companion Packages

Optional ecosystem and protocol dependencies remain outside the zero-dependency
core.

| Package | Purpose |
| --- | --- |
| `base64-ng-sanitization` | Protected and locked secret integration |
| `base64-ng-derive` | `Base64Secret` derive for fixed secret newtypes |
| `base64-ng-serde` | Explicit Serde field adapters |
| `base64-ng-bytes` | `Bytes`, `Buf`, and `BufMut` helpers |
| `base64-ng-subtle` | Sealed `subtle::ConstantTimeEq` integration |
| `base64-ng-tokio` | Async helpers and streaming adapters |
| `base64-ng-imap` | RFC 3501 modified-Base64 payload transforms |
| `base64-ng-mime` | RFC 2045 Base64 body transforms |
| `base64-ng-multibase` | Registered Base64-family multibase prefixes |
| `base64-ng-password` | Passlib PBKDF2 and SHA-crypt field transforms |
| `base64-ng-openpgp` | RFC 9580 ASCII armor |
| `base64-ng-pem` | RFC 7468 textual encoding |
| `@valkyoth/base64-ng-wasm-loader` | Scalar/`simd128` JavaScript loader |

Each Rust companion has its own crate README and examples. The
[package topology](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_PACKAGE_TOPOLOGY.md)
defines their boundaries.

Install the supported JavaScript loader with:

```sh
npm install @valkyoth/base64-ng-wasm-loader
```

```js
import { Codecs, createBase64Ng } from "@valkyoth/base64-ng-wasm-loader";

const base64 = await createBase64Ng();
const encoded = base64.encode(new TextEncoder().encode("hello"), Codecs.URL_SAFE_NO_PAD);
const decoded = base64.decode(encoded, Codecs.URL_SAFE_NO_PAD);
base64.dispose();
```

## Verification Status

"Project validated" means the named repository tests and evidence gates passed
on the stated systems. It does not mean certification, independent review,
whole-crate formal proof, or a portable performance guarantee.

| Backend | Project evidence | Dispatch status |
| --- | --- | --- |
| Portable scalar | Native x86-64, AArch64, RISC-V; QEMU big-endian targets | Admitted |
| x86 SSSE3/SSE4.1, AVX2, AVX-512 VBMI | Native differential, kernel, assembly, and benchmark evidence | Admitted within documented thresholds |
| AArch64 NEON | Apple Silicon and AWS Neoverse-N1 evidence | Admitted |
| wasm `simd128` | Node, Wasmtime, Chromium, Firefox, and Safari evidence | Admitted artifact |
| RISC-V RVV 1.0 | Native SpacemiT X60 plus QEMU vector-length evidence | Admitted only for the exact documented X60 profile |
| AArch64 SVE | QEMU and assembly evidence only | Not admitted |
| Big-endian SIMD | No accelerated backend | Scalar only |
| Secret encode/decode | Scalar fixed-work evidence | Separate from ordinary SIMD |

The full backend matrix, exact machines, thresholds, non-claims, and evidence
links live in the [Trust Dashboard](https://github.com/valkyoth/base64-ng/blob/main/docs/TRUST.md)
and [dispatch matrix](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md).

## Rust Support

MSRV remains Rust `1.90.0`. The active release toolchain is Rust `1.97.1`.

| Rust | Evidence |
| --- | --- |
| `1.90.0` | MSRV compatibility check |
| `1.91.0` - `1.97.0` | `cargo check --all-features` |
| `1.97.1` | Active release toolchain and full release checks |

New deployments should prefer the latest tested stable Rust.

## Security And Engineering Documentation

- [Migration guide](https://github.com/valkyoth/base64-ng/blob/main/docs/MIGRATION.md)
- [Codec and operation contracts](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_CODEC_SPECIFICATIONS.md)
- [Constant-time posture](https://github.com/valkyoth/base64-ng/blob/main/docs/CONSTANT_TIME.md)
- [Unsafe boundary](https://github.com/valkyoth/base64-ng/blob/main/docs/UNSAFE.md)
- [Trust Dashboard](https://github.com/valkyoth/base64-ng/blob/main/docs/TRUST.md)
- [Security controls](https://github.com/valkyoth/base64-ng/blob/main/docs/SECURITY_CONTROLS.md)
- [2.0 commit plan](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0.0-release-plan.md)
- [Governance decision](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_GOVERNANCE.md)
- [Release evidence](https://github.com/valkyoth/base64-ng/blob/main/docs/RELEASE_EVIDENCE.md)

Repository CI includes formatting, Clippy, tests, rustdoc, dependency policy,
fuzz and formal-verification harnesses, hardware-specific execution, SBOM, and reproducible build check. These engineering artifacts remain in the GitHub
repository and are intentionally excluded from the crates.io package.

## License

Licensed under either Apache-2.0 or MIT, at your option.

Contributions and release policy are documented in
[CONTRIBUTING.md](https://github.com/valkyoth/base64-ng/blob/main/CONTRIBUTING.md)
and [docs/RELEASE.md](https://github.com/valkyoth/base64-ng/blob/main/docs/RELEASE.md).
