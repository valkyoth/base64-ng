<p align="center">
  <b>Secure, no_std-first Base64 for Rust.</b><br>
  Strict decoding, caller-owned buffers, zero runtime dependencies, and release-gated security evidence.
</p>

<div align="center">
  <a href="https://docs.rs/base64-ng">Docs.rs</a>
  |
  <a href="docs/TRUST.md">Trust Dashboard</a>
  |
  <a href="docs/SECURITY_CONTROLS.md">Security Controls</a>
  |
  <a href="docs/PLAN.md">Roadmap</a>
  |
  <a href="SECURITY.md">Security</a>
</div>

<br>

<p align="center">
  <img src="https://raw.githubusercontent.com/valkyoth/base64-ng/main/.github/images/base64-ng.webp" alt="base64-ng Rust crate overview">
</p>

# base64-ng

`base64-ng` is a `no_std`-first Base64 crate focused on correctness, strict decoding, caller-owned buffers, and a security-heavy release process. The long-term goal is to provide modern hardware acceleration without making unsafe SIMD the foundation of trust.

The crate starts conservative: a small scalar implementation, strict RFC 4648 behavior, and a test/release system modeled after hardened Rust service projects. Streaming is available behind an explicit feature, fuzz harnesses are isolated from the published crate, and SIMD and broader Kani work remain gated until they have evidence.

## Current Status

The current public release is `1.3.8`.

`1.3.8` is a security-hardening and crate-family synchronization patch on top
of the `1.3.0` implementation-completion release, the `1.3.1` Tokio
writer patch, the `1.3.2` non-standard SIMD surface review, the `1.3.3`
wasm SIMD runtime-dispatch and profile-ergonomics patch, the `1.3.4`
big-endian QEMU evidence patch, the `1.3.5` RISC-V QEMU evidence patch, and
the `1.3.6` documentation refresh and `1.3.7` maintenance patch.
It keeps the admitted `1.2.x` native encode acceleration posture and the
admitted `1.3.0` normal strict SIMD decode scope for Standard and URL-safe
alphabet families on std `x86`/`x86_64` AVX-512 VBMI, AVX2, SSSE3/SSE4.1,
and little-endian std `aarch64` NEON after whole-input scalar validation.

Encode acceleration remains active only for admitted Standard and URL-safe
fixed-block surfaces. Decode acceleration remains limited to normal strict
decode, including the compacted strict stage used after wrapped line-profile
validation and strict in-place decode after stack staging. Legacy whitespace
compaction, custom alphabets, bcrypt-style and `crypt(3)` profiles, `no_std`,
and constant-time-oriented secret decode remain scalar. In-place encode may
use admitted encode backends only after stack staging protects unread input
bytes. Wasm `simd128` is admitted only for Standard and URL-safe public encode
and normal strict decode when the binary is compiled with
`target-feature=+simd128`, the `simd` feature, and the explicit
`allow-wasm32-best-effort-wipe` feature.

The latest patch in this line is `1.3.8`, which keeps all workspace crate
versions aligned and closes two low-severity evidence and cleanup gaps. Tokio
read-all helpers now wipe replaced vector allocations before deallocation, and
the Chromium wasm smoke gate requires a success attribute created only by
runtime execution rather than matching a token present in static HTML.
Runtime encoding also rejects custom `Alphabet::encode` overrides that
disagree with their authoritative `Alphabet::ENCODE` table, and a dedicated
daily workflow checks all maintained lockfiles for newly published advisories.
The stronger RISC-V RVV proof and admission review is scheduled for `1.3.9`;
until then, RISC-V remains QEMU-tested scalar/fallback-only. The workspace
crate family stays version-aligned at `1.3.8`.

Implemented on this branch now:

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
- Validation-only APIs for strict and legacy profiles when callers need to
  reject malformed input without materializing decoded bytes.
- Line-wrapped encoding for MIME/PEM-style output and caller-selected wrapping
  policies.
- Strict line-wrapped validation and decoding profiles for MIME/PEM-style
  input.
- Custom alphabet validation helpers for user-defined 64-byte alphabets.
- Named dependency-free profiles for MIME, PEM, bcrypt-style, and
  `crypt(3)`-style Base64.
- Stack-backed encoded output buffers for short values without `alloc`.
- Redacted secret owned buffers for sensitive encoded or decoded bytes when
  `alloc` is enabled.
- Separate `ct` scalar validation and decode module for sensitive payloads
  that avoids secret-indexed lookup tables during Base64 symbol mapping.
- `std::io` streaming encoders and decoders behind the `stream` feature.
- Focused unit and integration tests.
- Isolated `cargo-fuzz` harnesses for decode, in-place decode, and stream
  chunk-boundary behavior.
- Isolated dudect-style timing harness for the constant-time-oriented scalar
  decoder.
- Bounded Kani proof harnesses that run with the documented Rust `1.90.0`
  verifier pairing and `cargo-kani 0.67.0`.
- Constant-time assembly evidence generation for reviewer inspection.
- Runtime-dispatched std `x86`/`x86_64` AVX-512 VBMI fixed-block encode,
  falling back to AVX2, then SSSE3/SSE4.1, and then scalar, plus
  little-endian std `aarch64` NEON fixed-block encode, for Standard and
  URL-safe alphabets behind the SIMD admission boundary. Public slice,
  clear-tail, alloc, and wrapped encode helpers can use admitted fixed-block
  encode for their unwrapped encoding step. All input lengths are supported:
  fixed blocks may be accelerated, while the final tail and padding are
  completed by the scalar encoder. In-place encode may use admitted encode
  backends only after stack staging protects unread input bytes. Unsupported
  CPUs, `no_std`, custom alphabets, and line-ending insertion stay scalar.
- Runtime-dispatched std `x86`/`x86_64` AVX-512 VBMI fixed-block strict decode
  in the `1.3.0` line, falling back to AVX2, then
  SSSE3/SSE4.1, and then scalar, plus little-endian std `aarch64` NEON
  fixed-block strict decode, limited to Standard and URL-safe alphabets after
  whole-input scalar validation. Public strict decode supports every valid
  encoded length: fixed blocks may be accelerated, while short inputs and
  non-block tails are decoded by scalar code. Wrapped decode may use admitted
  strict decode after scalar line-profile validation and line-ending
  compaction; legacy whitespace decode may use admitted strict decode after
  scalar whitespace compaction. Strict in-place decode may use admitted strict
  decode backends only after stack staging. Unsupported CPUs, big-endian
  AArch64, `no_std`, custom alphabets, and CT secret decode stay scalar.
- Runtime-dispatched wasm `simd128` fixed-block encode and normal strict
  decode for Standard and URL-safe alphabets when built for `wasm32` with
  `target-feature=+simd128`, `simd`, and
  `allow-wasm32-best-effort-wipe`. The admission is backed by Node/V8,
  Wasmtime, Chromium-family browser, Firefox/SpiderMonkey, and Safari/WebKit runtime smoke evidence; unsupported
  wasm builds stay scalar by compiling without `+simd128`.
- Optional `base64-ng-sanitization` companion crate for applications that
  already admit `sanitization` and want direct CT decode helpers into
  clear-on-drop secret containers.
- Optional `base64-ng-derive` companion crate for fixed-size byte newtypes that
  need narrow Base64 parsing and encoding helpers.
- Optional `base64-ng-serde`, `base64-ng-bytes`, `base64-ng-subtle`, and
  `base64-ng-tokio` companion crates for projects that explicitly admit those
  ecosystem dependencies.
- Local check scripts, release gate, dependency policy, audit config, CI, SBOM script, reproducible build check, and a 500-line production-source budget guard.

Planned behind admission evidence:

- Additional SIMD decode acceleration beyond the `1.3.0`
  strict Standard and URL-safe decode scope only after separate admission
  evidence is complete. The frozen first scope remains padded and unpadded
  strict decode only after any scalar line or whitespace compaction: no
  vectorized line wrapping, no vectorized legacy whitespace, no custom
  alphabets, no bcrypt/crypt profiles, and no constant-time-oriented secret
  decode. Default builds, unsupported runtime CPUs, `no_std`, and all
  out-of-scope decode surfaces remain scalar.
- Additional custom alphabet and broader wasm
  runtime/browser fast paths only after separate SIMD admission evidence is
  complete. Default builds and unsupported runtime CPUs remain scalar.
- Async reader and writer streaming is available through the optional
  `base64-ng-tokio` companion crate after cancellation-safety, accepted-byte,
  and backpressure review. The core `tokio` feature remains inert; use the
  companion crate for admitted async integration.
- Additional Kani harnesses beyond the current bounded no-default-features
  proof set. A clean Kani run proves only the scoped harness properties, not a
  whole-crate or cryptographic constant-time guarantee.
- Broader benchmark evidence against the established `base64` crate.

## Trust Dashboard

| Area | Status |
| --- | --- |
| License | `MIT OR Apache-2.0` |
| MSRV | Rust `1.90.0` |
| Active release toolchain | Rust `1.97.0` |
| Runtime dependencies | Zero external crates |
| Unsafe policy | Scalar encode/decode remains safe Rust; audited unsafe is limited to volatile wiping, CT comparison/barrier helpers, and the reviewed SIMD boundary |
| Active backend | Scalar by default; std x86/x86_64 AVX-512 VBMI preferred, then AVX2, then SSSE3/SSE4.1, plus little-endian std aarch64 NEON, and wasm `simd128` when the admitted feature/runtime profile is present |
| Strict decoding | Default, canonical, no whitespace |
| Legacy compatibility | Explicit opt-in APIs |
| Constant-time posture | Constant-time-oriented scalar validation/decode with isolated dudect-style timing evidence; no formal cryptographic guarantee |
| Cleanup posture | Best-effort initialized-byte cleanup and redacted secret wrappers |
| Kani | 18 bounded no-default-features harnesses verified with the Rust `1.90.0` Kani toolchain and `cargo-kani 0.67.0`; not a whole-crate formal-verification claim |
| Release evidence | fmt, clippy, tests, docs, deny, audit, license, SBOM, reproducibility |

Full adoption details live in [docs/TRUST.md](docs/TRUST.md). Security-control
and CWE mapping lives in [docs/SECURITY_CONTROLS.md](docs/SECURITY_CONTROLS.md).

## Rust Version Support

The minimum supported Rust version is Rust `1.90.0`. New deployments should
prefer the latest tested stable Rust; as of July 15, 2026, this project tests
through Rust `1.97.0`.

The active release toolchain is Rust `1.97.0`. MSRV remains Rust `1.90.0` and
is checked separately in CI so the project can build and test with the latest
stable compiler without dropping older supported users.

Compatibility evidence for the `1.3.8` workspace:

| Rust | Local Evidence |
| --- | --- |
| `1.90.0` | ✓ MSRV compatibility check |
| `1.91.0` | ✓ `cargo check --all-features` |
| `1.92.0` | ✓ `cargo check --all-features` |
| `1.93.0` | ✓ `cargo check --all-features` |
| `1.94.0` | ✓ `cargo check --all-features` |
| `1.95.0` | ✓ `cargo check --all-features` |
| `1.96.0` | ✓ `cargo check --all-features` |
| `1.96.1` | ✓ `cargo check --all-features` |
| `1.97.0` | ✓ active release toolchain and `cargo check --all-features` |

## Install

```toml
[dependencies]
base64-ng = "1.3.8"
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
| `simd` | no | Admitted std runtime-dispatched encode and normal strict decode acceleration for Standard and URL-safe alphabets, with scalar fallback for unsupported surfaces. |
| `stream` | no | `std::io` streaming wrappers. |
| `allow-wasm32-best-effort-wipe` | no | Explicitly allow `wasm32` builds with compiler-fence-only cleanup. |
| `allow-compiler-fence-only-wipe` | no | Explicitly allow unsupported native architectures to build with compiler-fence-only cleanup after platform review. |
| `tokio` | no | Reserved placeholder in the core crate; currently inert and dependency-free. Use `base64-ng-tokio` for the admitted async helper and streaming adapter surface. |
| `kani` | no | Reserved for verifier harnesses; normal builds do not require Kani. |
| `fuzzing` | no | Reserved for verifier and fuzz harness integration; published crate stays dependency-free. |

High-assurance deployments that must fail closed when SIMD is enabled can build
with `RUSTFLAGS="--cfg base64_ng_require_high_assurance"`. This custom cfg is
not a Cargo feature, so normal `--all-features` release evidence and docs.rs
builds remain usable. When the cfg is set, `base64-ng` refuses to compile if
the `simd` feature is enabled.

## Companion Crates

The core `base64-ng` crate keeps its zero-runtime-dependency policy. Optional
ecosystem integrations live as separate crates so applications can opt into
their own approved dependency set without changing the base package.

The `1.3.0` family syncs all companion crates to the same version so docs.rs
and crates.io examples resolve consistently across the workspace.

| Crate | Purpose |
| --- | --- |
| `base64-ng` | Stable zero-runtime-dependency facade crate and primary user entry point. |
| `base64-ng-sanitization` | Optional `sanitization` integration with native `Choice` comparison helpers and opt-in locked secret decode helpers. |
| `base64-ng-derive` | Dependency-free `Base64Secret` derive for fixed-size secret newtypes. |
| `base64-ng-serde` | Optional `serde` wrappers for projects that already admit `serde`. |
| `base64-ng-bytes` | Optional `bytes` helpers for `Bytes`, `Buf`, and `BufMut` users. |
| `base64-ng-subtle` | Optional `subtle::ConstantTimeEq` helpers for token/MAC comparison boundaries. |
| `base64-ng-tokio` | Optional Tokio read-all/write-all helpers and async reader/writer streaming adapters. |

Subcrates are documented so crate pages are readable, but they belong to the
main `base64-ng` crate family and are not intended as independent protocol
products. Package versions and crates.io links are tracked in
[Crate Version Matrix](docs/CRATE_VERSION_MATRIX.md) so releases can publish
only the crates that changed instead of republishing the whole ecosystem.

`base64-ng-sanitization` provides extension helpers for
`base64_ng::ct::CtEngine` that decode directly into
`sanitization::SecretBytes<N>` in `no_std`, with `SecretVec` helpers behind its
own `alloc` feature. The `1.3.8` companion uses exact-pinned
`sanitization` `=1.2.4` and exposes `sanitization::ct::Choice` comparison
helpers through `SanitizationCtEqExt`. Enable the companion's
`high-assurance` feature to
decode directly into `sanitization::LockedSecretBytes` or
`sanitization::LockedSecretVec` on supported native targets, using
`sanitization` memory locking plus canary checking and random canaries:

```toml
[dependencies]
base64-ng = { version = "1.3.8", default-features = false }
base64-ng-sanitization = { version = "1.3.8", default-features = false }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

assert!(secret.sanitization_verify(
    b"hello",
    "example compares public expected bytes"
));
```

```toml
[dependencies]
base64-ng-sanitization = { version = "1.3.8", features = ["high-assurance"] }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationExt;

let locked = ct::STANDARD
    .decode_locked_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

locked.with_secret(|bytes| assert_eq!(bytes, b"hello"));
```

`base64-ng-derive` provides a dependency-free `Base64Secret` derive for tuple
newtypes around fixed byte arrays:

```toml
[dependencies]
base64-ng = { version = "1.3.8", default-features = false }
base64-ng-derive = "1.3.8"
```

```rust
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
struct ApiKey([u8; 5]);

let key = ApiKey::from_base64(b"aGVsbG8=").unwrap();
assert_eq!(key.as_bytes(), b"hello");
assert_eq!(key.encode_base64::<8>().unwrap().as_str(), "aGVsbG8=");
```

`base64-ng-serde` provides explicit serialization wrappers without admitting
`serde` into the core package:

```toml
[dependencies]
base64-ng-serde = "1.3.8"
serde = { version = "1.0.228", features = ["derive"] }
```

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct Message {
    #[serde(with = "base64_ng_serde::standard")]
    payload: Vec<u8>,
}
```

Field-level modules are available for `standard`, `standard_no_pad`,
`url_safe`, `url_safe_no_pad`, `mime`, and `pem`.

`base64-ng-bytes` provides `Bytes`, `Buf`, and `BufMut` helpers:

```toml
[dependencies]
base64-ng = "1.3.8"
base64-ng-bytes = "1.3.8"
bytes = "1.12.1"
```

```rust
use base64_ng::STANDARD;
use base64_ng_bytes::EngineBytesExt;

let encoded = STANDARD.encode_bytes(b"hello").unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");
```

`base64-ng-subtle` provides explicit `subtle::ConstantTimeEq` integration for
projects that already admit `subtle`:

```toml
[dependencies]
base64-ng = "1.3.8"
base64-ng-subtle = "1.3.8"
```

```rust
use base64_ng::ct;
use base64_ng_subtle::SubtleEqExt;

let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
assert!(decoded.subtle_verify(b"hello"));
```

`base64-ng-tokio` provides read-all/write-all async helpers and manual
`AsyncRead`/`AsyncWrite` streaming adapters for applications that already use
Tokio. Prefer the `*_limited` helper variants for request or frame boundaries
controlled by a peer. Read-all temporary allocations are RAII-wiped on return,
error, and cancellation. Limited helpers may consume one lookahead byte to
detect overflow, so use an already bounded reader when adjacent input must
remain unread:

```toml
[dependencies]
base64-ng = "1.3.8"
base64-ng-tokio = "1.3.8"
tokio = { version = "1.52.3", features = ["io-util"] }
```

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader, EncoderWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

# async fn example() -> std::io::Result<()> {
let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 1024).await?;
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await?;
assert_eq!(streamed, b"aGVsbG8=");

let mut writer = EncoderWriter::new(Vec::new(), STANDARD);
writer.write_all(b"hello").await?;
writer.shutdown().await?;
let encoded = writer.into_inner()?;
assert_eq!(encoded, b"aGVsbG8=");
# Ok(())
# }
```

Disable defaults for embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "1.3.8", default-features = false }
```

Enable admitted encode acceleration on supported `std` targets with the
`simd` feature. The public encode APIs do not change; runtime dispatch selects
an admitted backend only when the CPU and input shape match the admission
scope, otherwise scalar encode is used. In the `1.3.0` line,
the same feature also enables admitted strict decode acceleration for Standard
and URL-safe alphabets after whole-input scalar validation:

```toml
[dependencies]
base64-ng = { version = "1.3.8", features = ["simd"] }
```

```rust
use base64_ng::{runtime, STANDARD};

let encoded = STANDARD.encode_string(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");

let decoded = STANDARD.decode_vec(encoded.as_bytes()).unwrap();
assert_eq!(decoded, b"hello");

let report = runtime::backend_report();
// With `simd` enabled this reports the active admitted encode backend, or
// scalar when the current platform/input is outside the admitted scope. The
// strict decode backend is reported separately.
println!("{}", report);
println!("{}", report.active_decode_backend());
```

## Convenience API

For strict standard padded Base64 on `alloc` targets:

```rust
assert_eq!(base64_ng::encode(b"hello").unwrap(), "aGVsbG8=");
assert_eq!(base64_ng::decode("aGVsbG8=").unwrap(), b"hello");
```

These helpers are intended for ordinary data and migration from simpler
Base64 APIs. For secret-bearing payloads, use the explicit engine, profile, or
`ct` APIs below so the security contract is visible at the call site.

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
Use `encode_array` for fixed-size static values, not for runtime data whose
size is controlled by an attacker.

Compile-time strict decoding:

```rust
use base64_ng::{DecodeError, STANDARD, URL_SAFE_NO_PAD};

const HELLO: Result<([u8; 5], usize), DecodeError> = STANDARD.decode_array(b"aGVsbG8=");
const URL_BYTES: Result<([u8; 2], usize), DecodeError> =
    URL_SAFE_NO_PAD.decode_array(b"-_8");

let (hello, hello_len) = HELLO.unwrap();
let (url_bytes, url_len) = URL_BYTES.unwrap();

assert_eq!(&hello[..hello_len], b"hello");
assert_eq!(&url_bytes[..url_len], b"\xfb\xff");
```

`decode_array` is strict and returns `Result` for malformed input, padding
errors, and undersized output arrays. It is useful for fixed static Base64
literals and does not replace the `ct` APIs for secret-bearing decode.

For untrusted length metadata, use checked length calculation:

```rust
use base64_ng::{
    LineEnding, LineWrap, checked_encoded_len, checked_wrapped_encoded_len, decoded_len,
};

assert_eq!(checked_encoded_len(5, true), Some(8));
assert_eq!(
    checked_wrapped_encoded_len(5, true, LineWrap::new(4, LineEnding::Lf)),
    Some(9)
);
assert_eq!(decoded_len(b"aGVsbG8=", true).unwrap(), 5);
```

## Validation Without Decoding

Use validation-only APIs when a protocol needs to sanitize input before storing,
routing, or accounting for it:

```rust
use base64_ng::{STANDARD, URL_SAFE_NO_PAD};

assert!(STANDARD.validate(b"aGVsbG8="));
assert!(!STANDARD.validate(b"aGVsbG8"));

STANDARD.validate_result(b"aGVsbG8=").unwrap();

assert!(URL_SAFE_NO_PAD.validate(b"-_8"));
assert!(!URL_SAFE_NO_PAD.validate(b"+/8"));
```

For line-wrapped or spaced legacy inputs, use the explicit legacy profile:

```rust
use base64_ng::STANDARD;

assert!(STANDARD.validate_legacy(b" aG\r\nVsbG8= "));
assert!(!STANDARD.validate_legacy(b" aG-V "));

let decoded = STANDARD
    .decode_buffer_legacy::<5>(b" aG\r\nVs\tbG8= ")
    .unwrap();
assert_eq!(decoded.as_bytes(), b"hello");
```

## Line-Wrapped Encoding

Use `LineWrap` when a protocol needs MIME/PEM-style line lengths:

```rust
use base64_ng::{LineEnding, LineWrap, STANDARD};

let wrap = LineWrap::new(4, LineEnding::Lf);
let mut output = [0u8; 9];
let written = STANDARD
    .encode_slice_wrapped(b"hello", &mut output, wrap)
    .unwrap();

assert_eq!(&output[..written], b"aGVs\nbG8=");
```

Built-in policies include `LineWrap::MIME`, `LineWrap::PEM`, and
`LineWrap::PEM_CRLF`. Wrapping inserts line endings between encoded lines and
does not append a trailing line ending after the final line. `LineEnding`
exposes `name()`, `Display`, `as_str()`, `as_bytes()`, and `byte_len()` for
allocation-free policy inspection. `name()` and `Display` return printable
identifiers such as `LF` and `CRLF`; `as_str()` returns the literal line-ending
bytes. `LineWrap` exposes `line_len()`, `line_ending()`, and `is_valid()` for
const-friendly policy checks and implements `Display` as `line_len:name`, for
example `76:CRLF`. `LineWrap::new` rejects zero line lengths; use
`LineWrap::checked_new` when wrapping policy comes from configuration.

Named profiles carry the wrapping policy for common protocols:

```rust
use base64_ng::{LineEnding, MIME, PEM};

assert_eq!(MIME.line_wrap().unwrap().line_len, 76);
assert_eq!(MIME.line_len(), Some(76));
assert_eq!(MIME.line_ending(), Some(LineEnding::CrLf));
assert_eq!(MIME.to_string(), "padded=true wrap=76:CRLF");
assert_eq!(PEM.line_wrap().unwrap().line_len, 64);
assert_eq!(PEM.line_len(), Some(64));

let mut encoded = [0u8; 82];
let written = MIME.encode_slice(&[0x5a; 58], &mut encoded).unwrap();
assert_eq!(&encoded[76..78], b"\r\n");
assert!(MIME.validate(&encoded[..written]));
```

An engine can also be promoted explicitly to an unwrapped profile when a common
configuration path expects profile values, or to the matching
constant-time-oriented decoder when sensitive decode policy is required:

```rust
use base64_ng::STANDARD;

let profile = STANDARD.profile();
let ct_decoder = STANDARD.ct_decoder();

assert!(profile.is_padded());
assert!(!profile.is_wrapped());
assert_eq!(ct_decoder.decoded_len(b"aGVsbG8=").unwrap(), 5);
```

When wrapping policy comes from configuration, prefer checked construction.
Use `Engine::checked_profile_with_wrap` when the profile should use the same
engine and only the wrapping policy is dynamic:

```rust
use base64_ng::{LineEnding, LineWrap, STANDARD};

let wrap = LineWrap::checked_new(76, LineEnding::CrLf).unwrap();
let profile = STANDARD.checked_profile_with_wrap(wrap).unwrap();

assert!(profile.is_valid());
assert!(profile.is_wrapped());
```

The same policy can be used for strict wrapped decoding. Unlike legacy
whitespace decoding, this accepts only the configured line ending and requires
every non-final line to have the configured encoded length:

```rust
use base64_ng::{LineEnding, LineWrap, STANDARD};

let wrap = LineWrap::new(4, LineEnding::Lf);
let mut output = [0u8; 5];
let written = STANDARD
    .decode_slice_wrapped(b"aGVs\nbG8=", &mut output, wrap)
    .unwrap();

assert_eq!(&output[..written], b"hello");

let encoded = STANDARD.encode_wrapped_buffer::<9>(b"hello", wrap).unwrap();
assert_eq!(encoded.as_bytes(), b"aGVs\nbG8=");

let decoded = STANDARD
    .decode_wrapped_buffer::<5>(encoded.as_bytes(), wrap)
    .unwrap();
assert_eq!(decoded.as_bytes(), b"hello");
```

## Custom Alphabets

User-defined alphabets can be generated and validated at compile time:

```rust
base64_ng::define_alphabet! {
    struct DotSlash = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
}

use base64_ng::Alphabet;

assert_eq!(DotSlash::decode(b'.'), Some(0));
```

The generated alphabet uses the deliberately conservative default
`Alphabet::encode` implementation: it performs a fixed 64-entry scan for every
emitted Base64 byte to avoid secret-indexed table lookups. The built-in
alphabets override this with optimized arithmetic mappers. For very large
payloads and custom alphabets, benchmark this tradeoff before using them on
untrusted high-volume traffic.

If you implement `Alphabet` manually, overriding `encode` with
`ENCODE[value as usize]` makes normal `Engine` encoding timing-sensitive with
respect to the 6-bit value. Similarly, a custom `decode` implementation affects
the normal strict decoder. The `ct` module does not call `Alphabet::decode`; it
scans `Alphabet::ENCODE` directly with its own fixed 64-entry mapper.

Built-in non-RFC alphabets are available for explicit interoperability:

```rust
use base64_ng::{BCRYPT, CRYPT};

let mut bcrypt = [0u8; 4];
let written = BCRYPT.encode_slice(&[0xff, 0xff, 0xff], &mut bcrypt).unwrap();
assert_eq!(&bcrypt[..written], b"9999");

let mut crypt = [0u8; 4];
let written = CRYPT.encode_slice(&[0xff, 0xff, 0xff], &mut crypt).unwrap();
assert_eq!(&crypt[..written], b"zzzz");
```

The bcrypt and `crypt(3)` profiles provide alphabets and no-padding behavior
only. They do not parse or verify complete password-hash strings.

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
`decode_slice_legacy_clear_tail`, `decode_in_place_legacy_clear_tail`, and
`decode_buffer_legacy`. Strict line-wrapped profiles provide
`decode_in_place_wrapped`, `decode_in_place_wrapped_clear_tail`, and the same
in-place behavior through `Profile::decode_in_place`. The `ct` module provides
the same clear-tail decode variants for callers using the constant-time-oriented
scalar decoder, `ct::CtEngine::decoded_len` for sizing caller-owned buffers
under the same opaque malformed-input policy, plus
`ct::CtEngine::decode_buffer` for stack-backed no-alloc decoded output.
For constant-time-oriented in-place decode, prefer
`ct::CtEngine::decode_in_place_clear_tail`. The non-clear-tail CT in-place API
was removed before the `1.0` stable boundary because failed in-place decode can
partially destroy the encoded input and retain decoded plaintext in the same
buffer. If the encoded token must be logged or retried after failure, keep a
separate copy before any in-place decode.

The default strict decoders are not constant-time decoders: they preserve exact
error indexes and may return early for malformed input, padding, length, or
output-size errors. Use `base64_ng::ct` for secret-bearing payloads where decode
timing posture matters more than localized error diagnostics.
Do not use `STANDARD`, `STANDARD_NO_PAD`, `URL_SAFE`, `URL_SAFE_NO_PAD`,
`MIME`, `PEM`, `BCRYPT`, or `CRYPT` as token-comparison or key-material decode
APIs when the encoded bytes or rejection reason are sensitive. Use
`ct::STANDARD`, `ct::URL_SAFE_NO_PAD`, or `STANDARD.ct_decoder()` instead and
perform any final token comparison with a constant-time-oriented comparison
appropriate for the protocol.
For reusable secret output buffers, use `ct::CtEngine::decode_slice_clear_tail`
or `ct::CtEngine::decode_buffer`. The non-clear-tail CT slice API was removed
before the `1.0` stable boundary because it can leave real decoded plaintext
from valid leading quanta in `output` when later malformed input is rejected
after the fixed-shape decode pass.
For shared-memory, HSM-adjacent, sandboxed, or other multi-principal threat
models where even transient writes to caller-owned output are unacceptable, use
`ct::CtEngine::decode_slice_staged_clear_tail` with a private staging buffer.
This staged API should be the default for enclave-adjacent code, shared memory,
or any service where another principal could observe the public output buffer
during the decode call. `decode_slice_clear_tail` wipes on error before it
returns, but the CT loop still writes decoded bytes before the final error gate.

For short values, `encode_buffer` returns a stack-backed `EncodedBuffer`
and `decode_buffer` returns a stack-backed `DecodedBuffer` without requiring
the `alloc` feature:

```rust
use base64_ng::{BCRYPT, MIME, STANDARD};

let encoded = STANDARD.encode_buffer::<8>(b"hello").unwrap();
assert_eq!(encoded.as_str(), "aGVsbG8=");
assert_eq!(encoded.as_utf8().unwrap(), "aGVsbG8=");
assert_eq!(encoded.to_string(), "aGVsbG8=");

let decoded = STANDARD.decode_buffer::<5>(encoded.as_bytes()).unwrap();
assert_eq!(decoded.as_bytes(), b"hello");

let bcrypt = BCRYPT.encode_buffer::<4>(&[0xff, 0xff, 0xff]).unwrap();
assert_eq!(bcrypt.as_bytes(), b"9999");

let wrapped = MIME.encode_buffer::<82>(&[0x5a; 58]).unwrap();
let decoded = MIME.decode_buffer::<58>(wrapped.as_bytes()).unwrap();
assert_eq!(decoded.as_bytes(), &[0x5a; 58]);
```

`EncodedBuffer` exposes bytes only through `as_bytes`, fallible `as_utf8`, and
`as_str`, and implements `Display` for allocation-free formatting of encoded
Base64 text. That `Display` implementation emits the full Base64 payload; do
not use `EncodedBuffer` for encoded secrets that may reach logs or error
messages.
`DecodedBuffer` exposes bytes through `as_bytes` and provides a fallible
`as_utf8` view for decoded text. Both expose `is_full()` and
`remaining_capacity()` for no-alloc sizing checks, redact the payload from
`Debug`, clear their backing arrays when dropped as best-effort data-retention
reduction, and provide explicit equal-length comparison through
`constant_time_eq_public_len`. They intentionally do not
implement `PartialEq`/`==`: the helper is a dependency-free best-effort
comparison, not a formal cryptographic token/MAC comparison primitive. Length
mismatch returns immediately and must be treated as public protocol
information. Applications that require a formally audited comparison should
admit that dependency at the application boundary, for example by comparing
exposed bytes with `subtle`. Do not use these helpers as the sole MAC,
bearer-token, password-hash, or authentication-secret comparison primitive in
high-assurance systems.

`into_exposed_array` is the explicit no-alloc ownership escape hatch for both
stack-backed buffers. It returns `ExposedEncodedArray` or
`ExposedDecodedArray`, keeping redacted formatting and best-effort drop-time
cleanup after ownership transfer. If a bare array is unavoidable, call
`into_exposed_unprotected_array_caller_must_zeroize`; cleanup then becomes the
caller responsibility.

Stack-backed buffers clear their backing arrays when dropped, but they cannot
clear historical stack-frame copies made by the compiler, caller code, panic
machinery, or operating system crash capture. For highly sensitive payloads,
prefer the clear-tail APIs as soon as the value is no longer needed, keep
secret lifetimes short, and combine crate-level cleanup with process policies
for locked memory, encrypted or disabled swap and hibernation, core dumps,
crash reporting, and allocator isolation for secret regions.
Cloning `EncodedBuffer` or `DecodedBuffer` creates a second live copy; avoid
cloning secret material unless the duplicate lifetime is explicitly accounted
for.
On `wasm32`, the wipe barrier uses only a compiler fence; the wasm runtime JIT
may still optimize or retain cleared bytes outside the crate's control.
For that reason, `wasm32` builds fail closed by default. Enable
`allow-wasm32-best-effort-wipe` only when the deployment explicitly accepts the
limitation and applies its own approved memory strategy around stack-backed
buffers.
Other native architectures without an implemented hardware wipe barrier also
fail closed by default. Enable `allow-compiler-fence-only-wipe` only after
reviewing `docs/UNSAFE.md` and applying platform memory controls appropriate
for that deployment.

When an owned heap buffer is acceptable but accidental logging is not, use
`encode_secret` and `decode_secret`:

```rust
use base64_ng::STANDARD;

let encoded = STANDARD.encode_secret(b"hello").unwrap();
assert_eq!(encoded.expose_secret(), b"aGVsbG8=");
assert_eq!(format!("{encoded:?}"), r#"SecretBuffer { bytes: "<redacted>", len: 8 }"#);

let decoded = STANDARD.decode_secret(encoded.expose_secret()).unwrap();
assert_eq!(decoded.expose_secret(), b"hello");
assert!(decoded.constant_time_eq_public_len(b"hello"));
assert_eq!(format!("{decoded}"), "<redacted>");

let wrapped = STANDARD
    .encode_wrapped_secret(b"hello", base64_ng::LineWrap::PEM)
    .unwrap();
let unwrapped = STANDARD
    .decode_wrapped_secret(wrapped.expose_secret(), base64_ng::LineWrap::PEM)
    .unwrap();
assert_eq!(unwrapped.expose_secret(), b"hello");

let legacy = STANDARD
    .decode_secret_legacy(b" aG\r\nVs\tbG8= ")
    .unwrap();
assert_eq!(legacy.expose_secret(), b"hello");

let decoded = base64_ng::SecretBuffer::try_from("aGVsbG8=").unwrap();
assert_eq!(decoded.expose_secret(), b"hello");
```

`SecretBuffer` conversion traits use the normal strict `STANDARD` decoder.
They provide redacted owned storage and best-effort cleanup, not
constant-time-oriented decoding. Use `base64_ng::ct` or the
`base64-ng-derive`/`base64-ng-sanitization` companions for secret-bearing
protocol inputs where malformed-input timing matters.

For malformed-input timing-sensitive payloads, prefer the `ct` owned secret
helper:

```rust
use base64_ng::ct;

let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
assert!(decoded.constant_time_eq_public_len(b"hello"));
```

For shared-memory, enclave-adjacent, HSM-style, or multi-principal deployments
where even transient writes into the final heap allocation are unacceptable,
use stack-staged owned decode:

```rust
use base64_ng::ct;

let decoded = ct::STANDARD
    .decode_secret_staged::<5>(b"aGVsbG8=")
    .unwrap();
assert!(decoded.constant_time_eq_public_len(b"hello"));
```

`SecretBuffer` clears vector spare capacity when a vector is wrapped, and clears
initialized bytes plus spare capacity when dropped. It does not claim formal
zeroization and cannot clean historical copies outside the wrapper or make
guarantees about allocator behavior. `SecretBuffer` intentionally does not
implement `PartialEq`/`==`; use the explicit
`constant_time_eq_public_len` helper only when its best-effort, public-length
security contract is sufficient. Length mismatch returns immediately and must
be treated as public protocol information. Applications that require a
formally audited comparison should admit that dependency at the application
boundary, for example by comparing exposed bytes with `subtle`.
`SecretBuffer` does not lock memory; high-assurance deployments should pair it
with OS memory-locking, encrypted or disabled swap, crash-dump suppression, and
allocator isolation where those controls are required.
On `wasm32`, the same compiler-fence-only wipe-barrier caveat applies to owned
secret buffers. `wasm32` builds fail closed by default; enable
`allow-wasm32-best-effort-wipe` only when the deployment explicitly accepts the
limitation and applies its own approved cleanup strategy.
`expose_secret_utf8` provides an explicit borrowed text view when the secret
bytes are valid UTF-8.

`into_exposed_vec` consumes the wrapper and returns an `ExposedSecretVec`, which
keeps redacted formatting and best-effort drop-time cleanup. If a raw `Vec<u8>`
is unavoidable, call
`into_exposed_unprotected_vec_caller_must_zeroize`; that method name is
intentionally loud because cleanup becomes the caller's responsibility.
`try_into_exposed_string` provides an explicit escape hatch for UTF-8 text and
returns an `ExposedSecretString`, which keeps redacted formatting and
best-effort drop-time cleanup. If a raw `String` is unavoidable, call
`into_exposed_unprotected_string_caller_must_zeroize`; cleanup then becomes the
caller responsibility. Invalid UTF-8 returns the redacted wrapper unchanged.

`SecretBuffer` also implements `From<Vec<u8>>` and `From<String>` for callers
that already own sensitive bytes or text and want to move them into the
redacted wrapper without copying initialized bytes. With `alloc` enabled,
stack-backed `EncodedBuffer` and `DecodedBuffer` values can also be consumed
into `SecretBuffer`; the stack backing array is cleared when the consumed
buffer drops at the end of the conversion.

`TryFrom<&str>`, `TryFrom<&[u8]>`, and `TryFrom<&[u8; N]>` for
`EncodedBuffer<CAP>` encode raw input bytes with strict standard padded Base64.
The same byte and text conversions for `DecodedBuffer<CAP>` and `SecretBuffer`
decode strict standard padded Base64.
`DecodedBuffer<CAP>` and `SecretBuffer` also implement `FromStr` with the same
strict standard padded decode policy. Use explicit engine or profile methods
for URL-safe, no-padding, MIME/PEM, bcrypt-style, or custom alphabets.

With the default `alloc` feature, vector and string helpers are available:

```rust
use base64_ng::STANDARD;

let encoded = STANDARD.encode_vec(b"hello").unwrap();
assert_eq!(encoded, b"aGVsbG8=");

let encoded_string = STANDARD.encode_string(b"hello").unwrap();
assert_eq!(encoded_string, "aGVsbG8=");

let infallible = STANDARD.encode_string_infallible(b"hello");
assert_eq!(infallible, "aGVsbG8=");

let decoded = STANDARD.decode_vec(&encoded).unwrap();
assert_eq!(decoded, b"hello");
```

The infallible encode helpers are for ordinary trusted byte buffers where
failure would indicate an internal length/allocation invariant break rather
than invalid input. Use the fallible helpers when input length comes from
untrusted metadata, allocation pressure must be reported, or the caller needs a
recoverable error. On 32-bit targets, very large inputs can overflow the
encoded length calculation, so services should keep externally sized buffers on
the fallible `encode_*` APIs.

With the `stream` feature, `std::io` encoders are available:

```rust
use std::io::{Read, Write};
use base64_ng::STANDARD;

let mut encoder = STANDARD.encoder_writer(Vec::new());
encoder.write_all(b"he").unwrap();
encoder.write_all(b"llo").unwrap();
assert!(encoder.has_pending_input());
encoder.try_finish().unwrap();
assert_eq!(encoder.get_ref(), b"aGVsbG8=");
let encoded = encoder.finish().unwrap();
assert_eq!(encoded, b"aGVsbG8=");

let mut reader = STANDARD.encoder_reader(&b"hello"[..]);
let mut encoded = String::new();
reader.read_to_string(&mut encoded).unwrap();
assert_eq!(encoded, "aGVsbG8=");

let mut decoder = STANDARD.decoder_writer(Vec::new());
decoder.write_all(b"aGVs").unwrap();
decoder.write_all(b"bG8=").unwrap();
assert!(decoder.has_terminal_padding());
let decoded = decoder.finish().unwrap();
assert_eq!(decoded, b"hello");

let mut reader = STANDARD.decoder_reader(&b"aGVsbG8="[..]);
let mut decoded = Vec::new();
reader.read_to_end(&mut decoded).unwrap();
assert_eq!(decoded, b"hello");
assert!(reader.has_terminal_padding());
assert!(reader.is_finished());
```

The explicit adapter constructors remain available when the engine should be
passed separately:

```rust
use base64_ng::{STANDARD, stream::Encoder};

let encoder = Encoder::new(Vec::new(), STANDARD);
assert_eq!(encoder.engine(), STANDARD);
```

The stream adapters expose `engine()` and `is_padded()` for policy inspection,
plus `pending_len()` and `has_pending_input()` for partial Base64 quantum
visibility, plus `pending_input_needed_len()` for the number of bytes needed to
complete the partial quantum. Reader adapters also expose
`buffered_output_len()`, `buffered_output_capacity()`,
`buffered_output_remaining_capacity()`, and `has_buffered_output()` for bytes
already decoded or encoded but not yet returned to the caller. Decoders
additionally expose `has_terminal_padding()` so framed protocols can tell when
a padded payload has ended and leave adjacent bytes for the next protocol
layer. Reader adapters also expose `is_finished()` once EOF or terminal padding
has been reached and all buffered output has been drained, and
`has_finished_input()` when the wrapped reader has reached EOF or terminal
padding but buffered output may still remain. Writer adapters expose
`try_finish()` to finalize pending input and flush the wrapped writer without
consuming the adapter, plus `is_finalized()` for explicit state inspection;
after successful finalization, later writes are rejected. Writer adapters also
expose `buffered_output_len()`, `buffered_output_capacity()`,
`buffered_output_remaining_capacity()`, and `has_buffered_output()` for encoded
or decoded bytes accepted by the adapter but not yet drained into the wrapped
writer. If a wrapped writer fails, retrying `flush()` or `try_finish()` drains
the buffered output without re-encoding or re-decoding the accepted input. All
stream adapters also expose `can_into_inner()` and `try_into_inner()` as
checked recovery paths that refuse to return the wrapped reader or writer while
doing so would discard pending input or buffered output. Their `Debug` output
reports adapter state without formatting the wrapped reader or writer,
including recovery readiness, pending quantum state, and fixed output queue
capacity. As with other `std::io::Write` implementations, direct `write()`
calls may accept only part of the provided input while buffering encoded or
decoded output; use `write_all()` when the whole input slice must be consumed.
Decoder writer and reader adapters fail closed after malformed Base64 input;
`is_failed()` exposes that state, while unchecked `into_inner()` remains
available for explicit recovery of the wrapped object.

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

- Stable Rust first. MSRV remains Rust `1.90.0`; the active release toolchain
  is Rust `1.97.0`. New deployments should prefer the latest tested stable
  Rust, currently Rust `1.97.0`.
- `no_std` core by default.
- Scalar encode/decode remains safe Rust.
- Audited unsafe helpers in `src/cleanup.rs` perform volatile best-effort
  wiping plus architecture-gated inline assembly and hardware store-ordering
  fences where stable Rust supports them, so cleanup writes resist common
  dead-store elimination and are ordered before the cleanup boundary on
  supported native architectures. Constant-time comparison, byte accumulation,
  CT scan, and CT result-gate hardening remain audited in `src/ct/`.
- Unsafe SIMD remains isolated under `src/simd/`; admitted AVX-512 VBMI,
  AVX2, SSSE3/SSE4.1, NEON, and narrow wasm `simd128` encode and strict decode
  paths are gated by their documented runtime profiles, and all non-admitted
  backends and API surfaces remain prototype-only or scalar.
- Local checks verify that `allow(unsafe_code)` is confined to the volatile
  wipe helpers and SIMD boundary, every unsafe function is inventoried, and
  every unsafe block has a nearby `SAFETY:` explanation. Architecture intrinsics,
  CPU feature detection, and target-feature gates are checked against the same
  boundary.
- [docs/UNSAFE.md](docs/UNSAFE.md) inventories every current unsafe site and
  its safety invariants.
- [docs/ASYNC.md](docs/ASYNC.md) defines the admission bar for async/Tokio
  APIs. The optional companion crate now admits read-all/write-all helpers and
  manual `AsyncRead`/`AsyncWrite` streaming adapters; the core `tokio` feature
  remains reserved and inert.
- [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) defines the dependency
  admission bar for any future external crate.
- `runtime::backend_report()` exposes the active admitted backend, detected
  candidate, candidate detection mode, SIMD feature status, security posture,
  and a conservative unsafe-boundary posture flag for audit logging. In the
  `1.2.x` line, non-scalar active values describe admitted encode dispatch.
  The `1.3.0` line exposes strict decode dispatch separately through
  `BackendReport::active_decode_backend()`. The
  unsafe-boundary flag is true only when the reserved `simd` feature is
  disabled; SIMD-enabled builds must rely on the release evidence scripts for
  boundary validation. On `no_std` and non-x86 targets, candidate detection is
  compile-time target-feature reporting, not runtime CPU probing.
- `runtime::require_backend_policy()` lets deployments assert scalar execution,
  disabled SIMD features, or no detected SIMD candidate.
- `BackendPolicy::HighAssuranceScalarOnly` combines the scalar/no-SIMD
  deployment checks into one assertion and rejects CT gate postures that are
  ordering-only, compiler-fence-only, or hardware-barrier-unattested. AArch64
  deployments that have platform evidence for CSDB may compile with
  `--cfg base64_ng_aarch64_csdb_attested`; that cfg is an operator
  attestation, not an automatic CPU probe. It reports
  `hardware-speculation-barrier-build-asserted` so logs distinguish a
  deployment assertion from a native target guarantee, and it is intentionally
  not a Cargo feature so `--all-features` cannot enable it by accident.
- Runtime backend, posture, and policy enums expose stable string identifiers
  for CI artifacts, audit logs, and deployment evidence.
- Runtime backend reports and policy failures use stable key/value display
  output for log ingestion.
- `Engine`, `ct::CtEngine`, `LineEnding`, `LineWrap`, and `Profile` implement
  printable `Display` output for policy logging without payload
  materialization.
- CI runs platform tests on Linux, Windows, pinned macOS ARM images, pinned
  Intel macOS, and `macos-latest` so the GitHub-hosted macOS migration remains
  visible without hiding compatibility regressions behind the moving label.
- Strict decoding rejects malformed padding and trailing data.
- Runtime scalar APIs are expected to return `Result` or `Option` for malformed
  input and size errors instead of panicking.
- Public encoded-length overflow is recoverable through `Result` or `Option`;
  untrusted length metadata should never require a panic.
- Scalar encode avoids input-derived alphabet table indexes, and scalar decode
  uses branch-minimized arithmetic. A separate `ct` module provides a
  constant-time-oriented scalar validation and decode path that scans the
  selected alphabet for every symbol so custom alphabets do not fall back to
  standard ASCII assumptions. Its malformed-input errors are intentionally
  non-localized, clear-tail variants clear caller-owned buffers on error, and
  it is not documented as a formally verified cryptographic constant-time API.
  Input length, padding length, decoded length, and final success/failure are
  public; callers that need protocol-level success/failure timing resistance
  should continue with fixed-shape dummy downstream work after decode failure.
- Clear-tail encode/decode variants are available for callers that want
  best-effort cleanup of unused caller-owned buffers without adding a runtime
  dependency.
- Streaming wrappers clear internal pending and queued byte buffers on drop and
  as buffered bytes are consumed, as best-effort retention reduction.
- Legacy compatibility must be opt-in.
- Release gates include formatting, clippy, tests, Miri when installed, docs,
  dependency policy, audit, license review, isolated fuzz/perf dependency
  checks, SBOM, and reproducible build checks.
- Kani harnesses stay in-tree and release-gated. The current
  no-default-features harness set verifies cleanly with the Rust `1.90.0`
  Kani toolchain and `cargo-kani 0.67.0`; this is scoped bounded evidence,
  not a whole-crate formal-verification claim.

See [docs/PLAN.md](docs/PLAN.md), [SECURITY.md](SECURITY.md),
[docs/RELEASE_EVIDENCE.md](docs/RELEASE_EVIDENCE.md), and
[docs/CONSTANT_TIME.md](docs/CONSTANT_TIME.md). For the unsafe hardware
acceleration gate, see [docs/SIMD.md](docs/SIMD.md).
For the trust dashboard and CWE/security-control mapping, see
[docs/TRUST.md](docs/TRUST.md) and
[docs/SECURITY_CONTROLS.md](docs/SECURITY_CONTROLS.md).
For panic-free public API policy, see
[docs/PANIC_POLICY.md](docs/PANIC_POLICY.md).
For constant-time-oriented decode verification requirements, see
[docs/CONSTANT_TIME.md](docs/CONSTANT_TIME.md).
For dependency admission rules, see [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
For adoption guidance from the established `base64` crate, see
[docs/MIGRATION.md](docs/MIGRATION.md).
For performance evidence guidance, see [docs/BENCHMARKS.md](docs/BENCHMARKS.md).
For fuzz target and corpus policy, see [docs/FUZZING.md](docs/FUZZING.md).

## Local Checks

Run the standard gate:

```sh
scripts/checks.sh
```

The standard gate includes isolated dudect, fuzz, and performance harness
compile/dependency checks. It does not run fuzz campaigns or benchmarks.

Check the zero-external-crate policy directly:

```sh
scripts/validate-dependencies.sh
```

Check release-facing documentation versions directly:

```sh
scripts/validate-doc-versions.sh
```

Check reserved feature placeholders directly:

```sh
scripts/check_reserved_features.sh
```

Check the wasm fail-closed cleanup policy directly:

```sh
scripts/check_wasm_wipe_policy.sh
```

Run the release gate:

```sh
scripts/stable_release_gate.sh
```

Install cross-compilation targets used by the local and CI target checks:

```sh
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
```

Run the dependency-free no-alloc portability smoke crate across the same
installed target list:

```sh
scripts/check_no_alloc_smoke.sh
```

Run the macOS host verification on an Apple Silicon or Intel Mac:

```sh
scripts/check_macos.sh
```

On an M2 MacBook Pro this runs the real host tests on
`aarch64-apple-darwin`, then compile-checks both `aarch64-apple-darwin` and
`x86_64-apple-darwin`.

Run the AArch64 Linux host verification on an ARM Linux machine, such as an
Amazon Graviton instance:

```sh
scripts/check_aarch64_linux.sh
```

This runs the host tests, all-feature tests, clippy, NEON encode block
evidence, backend evidence, SIMD feature-bundle checks, and SIMD admission
validators on the real AArch64 host.

Required security tools:

CI and local release scripts use `scripts/ci_install_rust.sh`; that script uses
`rust-toolchain.toml` as the single source of truth for the active release
toolchain. MSRV remains Rust `1.90.0` and is checked separately.

```sh
cargo install --locked cargo-audit --version 0.22.2
cargo install --locked cargo-license --version 0.7.0
cargo install --locked cargo-deny --version 0.20.2
cargo install --locked cargo-sbom --version 0.10.0
```

Optional deep tools:

```sh
cargo install --locked cargo-nextest --version 0.9.140
cargo install --locked cargo-fuzz --version 0.13.2
cargo install --locked kani-verifier --version 0.67.0
```

Verify optional tool installation:

```sh
cargo nextest --version
cargo fuzz --version
cargo kani --version
```

Compile and audit fuzz targets directly while iterating on fuzz harnesses:

```sh
scripts/check_fuzz.sh
```

Validate the committed fuzz corpus policy directly:

```sh
scripts/check_fuzz_corpus.sh
```

Compile and audit the isolated performance harness directly:

```sh
scripts/check_perf.sh
```

Run the default benchmark. The perf crate enables `base64-ng` SIMD by default
and records the active backend in each CSV row:

```sh
cargo run --release --manifest-path perf/Cargo.toml
```

Run the base64-ng scalar baseline explicitly:

```sh
cargo run --release --manifest-path perf/Cargo.toml --no-default-features
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
scripts/check_miri.sh
```

Kani may need a one-time setup after installation:

```sh
cargo kani setup
```

On openSUSE Tumbleweed, install `rustup` first if it is not already present:

```sh
sudo zypper install rustup
```

The local release gate runs Miri automatically when `rustup run nightly cargo
miri` is available. `scripts/check_miri.sh` covers no-default-features scalar
APIs and all-features alloc/stream APIs. The large deterministic sweep tests are
ignored only under Miri because they are already covered by the normal release
gate and are too slow for an interpreter.

## Project Principles

- Keep external crates to the absolute minimum. The current crate dependency graph is only `base64-ng`.
- Correctness first, speed second, unsafe last.
- The scalar implementation is the reference behavior.
- SIMD must prove equivalence to scalar behavior across fuzzed and deterministic inputs.
- Constant-time claims require empirical timing evidence, generated-code
  review, and explicit documented exclusions.
- Compatibility modes must be visible in the type/API surface.
- Release evidence belongs in the repository and CI, not in memory.

## Contributing And Releases

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution rules and [docs/RELEASE.md](docs/RELEASE.md) for the maintainer release checklist.
