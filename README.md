<p align="center">
  <b>Secure, no_std-first Base64 for Rust.</b><br>
  Strict RFC 4648 decoding, caller-owned buffers, zero runtime dependencies, and release-gated security evidence.
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

`base64-ng` is a `no_std`-first Base64 crate focused on correctness, strict decoding, caller-owned buffers, and a security-heavy release process. Its 2.0 family combines a safe scalar foundation with separately admitted hardware acceleration and optional integration and protocol companions.

Strict RFC 4648 behavior remains the default. Forgiving, wrapped, legacy, and protocol-specific behavior is explicitly named. Streaming is available through the core `stream` feature and the Tokio companion, fuzz and formal-verification harnesses are isolated from published packages, and SIMD execution is limited to backends with the documented admission evidence.

Zero external runtime or development dependencies in `Cargo.toml`. Optional
ecosystem dependencies remain isolated in companion packages.

## RFC 4648 Conformance

The current `STANDARD`, `STANDARD_NO_PAD`, `URL_SAFE`, and
`URL_SAFE_NO_PAD` engines implement strict RFC 4648 Base64 behavior.
`STANDARD` and `URL_SAFE` require and emit canonical padding;
the explicitly named `_NO_PAD` engines reject padding and emit none. Strict
decoders reject whitespace, mixed alphabets, impossible lengths, malformed or
forbidden padding, trailing data after padding, and non-canonical unused
trailing bits. Legacy whitespace and MIME/PEM-style line handling are available
only through separately named opt-in APIs.

The test suite includes the RFC 4648 Section 10 vectors. The exact RFC Editor
text, checksum, reviewed errata, and normative requirements mapping are locked
under [`rfc/`](https://github.com/valkyoth/base64-ng/tree/main/rfc) and verified
offline according to the
[RFC source policy](docs/rfc-source-policy.md).
The authoritative [2.0 commit plan](docs/2.0.0-release-plan.md) and supporting
[governance decision](docs/2.0_GOVERNANCE.md) define the numbered,
pentest-gated path from the signed 1.3.9 baseline to the single final 2.0.0
release.
The [2.0 API migration ledger](docs/2.0_API_MIGRATION_LEDGER.md) and
[package topology](docs/2.0_PACKAGE_TOPOLOGY.md) freeze public renames,
removals, capability edges, and companion boundaries before implementation.

## Current Status

This source tree defines the synchronized `2.0.0` package family. The
implementation and package topology are frozen. Publication is authorized only
from the signed `v2.0.0` tag after external review, required CI, the complete
release gate, and source-bound hardware evidence pass.

The 2.0 family includes the complete 2.0 API, synchronized companion crates,
the supported npm Wasm loader, and exact-profile RVV dispatch for the reviewed
Linux/SpacemiT X60 identity. See the
[release freeze](docs/2.0_RELEASE_FREEZE.md),
[release notes](release-notes/RELEASE_NOTES_2.0.0.md), and
[migration guide](docs/MIGRATION.md) for the frozen scope and adoption path.

Reviewers testing an untagged candidate should pin the exact reviewed Git
revision:

```toml
base64-ng = { git = "https://github.com/valkyoth/base64-ng", rev = "<reviewed-commit>" }
```

## Backend Verification Status

"Project validated" means the named repository tests and evidence gates pass
on the stated environments. It does not mean independent verification,
certification, formal proof, or a portable performance guarantee. A backend is
eligible for safe automatic dispatch only when its row says `admitted`;
included candidates with incomplete hardware evidence remain unreachable from
normal public dispatch.

| Surface | Implementation | Project execution evidence | Safe automatic dispatch | Independent verification |
| --- | --- | --- | --- | --- |
| Portable scalar encode and strict decode | Complete | Native x86-64, Apple/AWS AArch64, and RISC-V; QEMU s390x, PowerPC64, and RISC-V | `admitted` | Not independently verified |
| x86 SSSE3/SSE4.1 and AVX2 encode/decode | Complete | Native x86-64 differential, direct-kernel, assembly, and benchmark gates | `admitted` | Not independently verified |
| x86 AVX-512 VBMI encode/decode | Complete | Native AMD AVX-512 VBMI; second Intel performance corroboration is queued for 2.0.1, so no portable throughput claim is made | `admitted` with conservative exact-host thresholds; strict decode remains exact/static only | Not independently verified |
| little-endian AArch64 NEON encode/decode | Complete | Native Apple Silicon and AWS Neoverse-N1 correctness, direct-kernel, assembly, and retained 15-sample performance bundles accepted through the source-equivalence gate | `admitted` | Not independently verified |
| wasm `simd128` encode/decode | Complete | Node/V8, Wasmtime, Chromium/V8, Firefox/SpiderMonkey, and Safari/WebKit package/runtime gates | `admitted` for the documented SIMD artifact | Not independently verified |
| RISC-V RVV 1.0 encode/decode | Complete exact-profile backend | QEMU VLEN 128/256 fallback/direct evidence plus native Banana Pi BPI-F3 SpacemiT X60 VLEN 256 correctness, signal/thread, ABI, cleanup, and performance evidence; the release gate requires an exact integrated-source bundle | `admitted` only for the exact Linux/X60 identity at 192-byte encode/decode thresholds; all other RISC-V stays scalar | Not independently verified |
| AArch64 SVE encode/decode | Complete candidate | QEMU vector lengths 128/256/512 plus generated assembly; no accepted native SVE report | `not admitted`; public execution remains NEON or scalar | Not independently verified |
| Constant-time-oriented secret encode/decode | Complete scalar bounded path | Fixed-work tests, Kani, assembly review, and dudect-style project evidence | Separate scalar path; never ordinary SIMD dispatch | No formal or independent constant-time verification |
| Big-endian acceleration | No backend implemented | Complete scalar suites under s390x and PowerPC64 QEMU only | `not admitted`; scalar only | No native hardware verification |

The detailed evidence and non-claims are maintained in
[the Trust Dashboard](docs/TRUST.md), [SIMD policy](docs/SIMD.md),
[RISC-V review](docs/RISCV_QEMU_REVIEW.md), and
[SVE review](docs/SVE_QEMU_REVIEW.md). This table is updated whenever a backend
implementation, execution environment, or admission decision changes.

General 2.0 implementation is complete, including exact-profile native RVV
admission. Release assurance requires external review, exact-source native RVV
and NEON evidence, green required CI and CodeQL, candidate-local package
evidence, the report-only Commit 55 seal, and the authorized signed tag.

RVV dispatch is limited to the measured Linux/SpacemiT X60 identity; other
RISC-V profiles remain scalar. SVE remains non-dispatchable pending native
hardware evidence. Big-endian execution remains scalar. Secret operations
remain on the separate scalar fixed-work path. Project tests, Kani harnesses,
timing evidence, native runs, and QEMU runs are scoped evidence, not
certification or whole-crate formal proof.

## Trust Dashboard

| Area | Status |
| --- | --- |
| License | `MIT OR Apache-2.0` |
| MSRV | Rust `1.90.0` |
| Active release toolchain | Rust `1.97.1` |
| Runtime dependencies | Zero external crates |
| Unsafe policy | Scalar encode/decode remains safe Rust; audited unsafe is limited to volatile wiping, CT comparison/barrier helpers, and the reviewed SIMD boundary |
| Active backend | Scalar by default; std x86/x86_64 encode selects SSSE3/SSE4.1, AVX2, or AVX-512 VBMI by length, strict decode selects SSSE3/SSE4.1 or AVX2, plus little-endian std aarch64 NEON, wasm `simd128`, and exact Linux/SpacemiT X60 RVV under their admitted profiles; AVX-512 strict decode is exact/static only |
| Strict RFC 4648 decoding | Default, canonical, no whitespace |
| Legacy compatibility | Explicit opt-in APIs |
| Constant-time posture | Constant-time-oriented scalar validation/decode plus bounded 2.0 secret frames with private staging and isolated timing evidence; no formal cryptographic guarantee |
| Cleanup posture | Best-effort initialized-byte cleanup and redacted secret wrappers |
| Kani | 43 normal and 19 release-host advanced harnesses with explicit resource limits on Rust `1.90.0` and `cargo-kani 0.67.0`; 6 high-cost integrated/wrapped harnesses remain exploratory, and this is not a whole-crate formal-verification claim |
| Release evidence | fmt, clippy, tests, docs, deny, audit, license, SBOM, reproducibility |

Full adoption details live in [docs/TRUST.md](docs/TRUST.md). Security-control
and CWE mapping lives in [docs/SECURITY_CONTROLS.md](docs/SECURITY_CONTROLS.md).

## Rust Version Support

The minimum supported Rust version is Rust `1.90.0`. New deployments should
prefer the latest tested stable Rust; the 2.0 family is built and release-gated
with Rust `1.97.1` while retaining a separate MSRV gate.

The active release toolchain is Rust `1.97.1`. MSRV remains Rust `1.90.0` and
is checked separately in CI so the project can build and test with the latest
stable compiler without dropping older supported users.

Compatibility evidence for the `2.0.0` workspace:

| Rust | Local Evidence |
| --- | --- |
| `1.90.0` | ✓ MSRV compatibility check |
| `1.91.0` - `1.97.0` | ✓ `cargo check --all-features` |
| `1.97.1` | ✓ active release toolchain and `cargo check --all-features` |

## Install

```toml
[dependencies]
base64-ng = "2.0.0"
```

For ordinary public data, the shortest API uses strict RFC 4648 Standard
Base64 with canonical padding:

```rust
let encoded = base64_ng::encode(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");

let decoded = base64_ng::decode(encoded.as_bytes()).unwrap();
assert_eq!(decoded, b"hello");
```

These convenience functions use normal strict decoding with detailed errors.
They are not the secret-bearing, fixed-work path. For keys, tokens, passwords,
or other secrets, start with the
[bounded secret decoder](docs/2.0_SECRET_DECODING.md) and the `secrets`
feature. For Serde fields, use
[`base64-ng-serde`](https://crates.io/crates/base64-ng-serde) instead of writing
a custom serializer.

Choose the narrowest API matching the surrounding contract:

| Need | Start with |
| --- | --- |
| Ordinary owned Standard Base64 | `base64_ng::encode` and `base64_ng::decode` |
| Explicit alphabet or padding policy | A `STRICT_*` preset with `encode_to_string` and `decode_to_vec` |
| Transactional caller-owned buffers | `encode_into` and `decode_into` |
| Heapless incremental processing | `encoder()` and `decoder()` |
| Secret-bearing data | `secret::{SecretArrayFrame, SecretVecFrame}` |
| Serde fields | `base64-ng-serde` |

For example, an explicit strict preset makes the alphabet and padding policy
visible at the call site:

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let encoded = STRICT_STANDARD_PADDED.encode_to_string(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");

let decoded = STRICT_STANDARD_PADDED.decode_to_vec(encoded.as_bytes()).unwrap();
assert_eq!(decoded, b"hello");
```

When validated encoded text must carry its policy through storage or
transport, use the ordinary `Base64String<S>` owner:

```rust
use base64_ng::{Base64String, STRICT_STANDARD_PADDED};

let stored = Base64String::encode(STRICT_STANDARD_PADDED, b"hello").unwrap();
assert_eq!(stored.as_str(), "aGVsbG8=");
assert_eq!(stored.decode().unwrap(), b"hello");
```

`Base64String` is printable and non-wiping. It is not a secret container.
`base64_ng::prelude` provides a focused import set for ordinary 2.0 code; it
deliberately omits secret, compatibility, protocol, and historical APIs.

The historical `STANDARD` family remains as reviewed compatibility API.
Forgiving web decode, legacy whitespace, line wrapping, and protocol-specific
transforms require separately named opt-in APIs.

## 2.0 API Guide

The root crate documents and tests each 2.0 capability independently:

| Capability | Primary guide |
| --- | --- |
| Validated alphabets, sealed codecs, strict presets, and custom policy builders | [Codec specifications](docs/2.0_CODEC_SPECIFICATIONS.md) |
| Error, progress, lifecycle, atomicity, and rollback contracts | [Operation contracts](docs/2.0_OPERATION_CONTRACTS.md) |
| Transactional caller-owned and allocating one-shot operations, including policy-carrying `Base64String` storage | [Transactional one-shot](docs/2.0_TRANSACTIONAL_ONE_SHOT.md) |
| Heapless incremental encode and strict padded/unpadded decode | [Encoder](docs/2.0_INCREMENTAL_ENCODER.md), [padded decoder](docs/2.0_INCREMENTAL_PADDED_DECODER.md), [finalization](docs/2.0_INCREMENTAL_DECODER_FINALIZATION.md) |
| Const transforms and fixed-capacity ordinary buffers | [Const and bounded buffers](docs/2.0_CONST_AND_BOUNDED_BUFFERS.md) |
| Ordinary and staged secret-adjacent in-place transforms | [In-place operations](docs/2.0_IN_PLACE_OPERATIONS.md) |
| Allocation-free formatting, rollback-safe append, and encoded chunk iteration | [Formatting, append, and chunks](docs/2.0_FORMAT_APPEND_CHUNKS.md) |
| Validated line wrapping and accurately scoped body profiles | [Line wrapping](docs/2.0_LINE_WRAPPING.md), [profiles](docs/2.0_PROFILES_AND_TERMINOLOGY.md) |
| Exact WHATWG forgiving decode and explicitly scoped compatibility policies | [Web forgiving Base64](docs/2.0_WEB_FORGIVING_BASE64.md), [profiles](docs/2.0_PROFILES_AND_TERMINOLOGY.md) |
| Bounded secret owners, fixed-work encode/decode, and explicit exposure | [Secret storage](docs/2.0_SECRET_STORAGE_AND_EXPOSURE.md), [decode](docs/2.0_SECRET_DECODING.md), [encode](docs/2.0_SECRET_ENCODING.md) |
| Protected allocations, assurance tokens, operation reports, and teardown | [Assurance and protected memory](docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md), [reporting](docs/2.0_OPERATION_REPORTING.md) |
| Runtime backend health, checked execution, quarantine, and dispatch reporting | [Backend health](docs/2.0_BACKEND_HEALTH.md), [dispatch matrix](docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md) |
| Synchronous I/O and Tokio async I/O | [Synchronous I/O](docs/2.0_SYNCHRONOUS_IO.md), [async overview](docs/ASYNC.md) |
| MIME, PEM, OpenPGP, IMAP, multibase, and password-record protocols | [Protocol registry](docs/2.0_PROTOCOL_REGISTRY.md), [companion crates](#companion-crates) |
| Wasm package and runtime loading | [Wasm runtime review](docs/WASM_SIMD128_RUNTIME_REVIEW.md), [loader package](packages/base64-ng-wasm-loader/README.md) |
| Serde, bytes, derive, subtle, sanitization, and Tokio integrations | [Companion crates](#companion-crates) and each package-local README |

The [migration guide](docs/MIGRATION.md) contains compiled examples for the
canonical one-shot, incremental, in-place, compatibility, secret, streaming,
and companion boundaries. Each companion crate also carries a package-local
README and runnable examples for its complete public scope.

The crate is dual-licensed:

```toml
license = "MIT OR Apache-2.0"
```

## Features

| Feature | Default | Purpose |
| --- | --- | --- |
| `alloc` | yes | `Vec` and encoded `String` convenience APIs. |
| `std` | yes | `std::error::Error` support and feature base for I/O. |
| `simd` | no | Admitted `std` runtime-dispatched or compile-time-proven `no_std` encode and strict-decode acceleration for Standard and URL-safe alphabets, with KAT/quarantine and scalar fallback. |
| `stream` | no | `std::io` streaming wrappers. |
| `secrets` | no | Dependency-free 2.0 secret storage, explicit exposure/declassification, bounded constant-time-oriented transforms, and generation-bound assurance/protected-memory APIs. |
| `checked-backend` | no | Enables `simd` plus bounded scalar/SIMD comparison, permanent process quarantine on mismatch, and one scalar retry without exposing suspect chunks. |
| `allow-wasm32-best-effort-wipe` | no | Explicitly allow `wasm32` `secrets` builds with compiler-fence-only cleanup; ordinary codecs do not need it. |
| `allow-compiler-fence-only-wipe` | no | Explicitly allow `secrets` builds on unsupported native architectures with compiler-fence-only cleanup after platform review. |
| `tokio` | no | Reserved placeholder in the core crate; currently inert and dependency-free. Use `base64-ng-tokio` for the admitted async helper and streaming adapter surface. |
| `kani` | no | Reserved for verifier harnesses; normal builds do not require Kani. |
| `fuzzing` | no | Reserved for verifier and fuzz harness integration; published crate stays dependency-free. |

High-assurance deployments can build with
`RUSTFLAGS="--cfg base64_ng_require_high_assurance"`. This custom cfg is not a
Cargo feature, so normal `--all-features` evidence and docs.rs builds remain
usable. It establishes build eligibility only. Assured 2.0 operations also
require a runtime assurance token and an allocation-specific protected owner.
The `simd` feature may coexist for ordinary APIs; assured secret operations
remain scalar. Use `BackendPolicy::HighAssuranceScalarOnly` when the entire
process must reject ordinary SIMD.

## Companion Crates

The core `base64-ng` crate keeps its zero-runtime-dependency policy. Optional
ecosystem integrations live as separate crates so applications can opt into
their own approved dependency set without changing the base package.

The `2.0.0` family syncs all companion crates to the same version so docs.rs
and crates.io examples resolve consistently across the workspace.

| Crate | Purpose |
| --- | --- |
| `base64-ng` | Stable zero-runtime-dependency facade crate and primary user entry point. |
| `base64-ng-sanitization` | Optional `sanitization` integration with native `Choice` comparison helpers and opt-in locked secret decode helpers. |
| `base64-ng-derive` | Dependency-free `Base64Secret` derive with sealed codec, staged decode, exact length, and opt-in exposure policy. |
| `base64-ng-serde` | Optional `serde` wrappers for projects that already admit `serde`. |
| `base64-ng-bytes` | Optional `bytes` helpers for `Bytes`, `Buf`, and `BufMut` users. |
| `base64-ng-subtle` | Sealed `subtle::ConstantTimeEq` integration for final 2.0 secret owners and token/MAC comparison boundaries. |
| `base64-ng-tokio` | Optional Tokio read-all/write-all helpers and async reader/writer streaming adapters. |
| `base64-ng-imap` | Bounded legacy RFC 3501 Section 5.1.3 modified-Base64 payload transforms over already-converted UTF-16BE bytes; not a complete mailbox codec. |
| `base64-ng-mime` | Bounded RFC 2045 Section 6.8 Base64 content-transfer body encoding and decoding; not a MIME message or header parser. |
| `base64-ng-multibase` | Bounded strict support for the four registered Base64-family multibase prefixes; not a complete open-world multibase registry. |
| `base64-ng-password` | Bounded Passlib PBKDF2 and SHA-crypt field/record transforms with exact checksum permutations; never hashes or verifies passwords. |
| `base64-ng-openpgp` | Bounded complete RFC 9580 ordinary ASCII armor parser and generator with explicit CRC-24 policy and opt-in secret payload release. |
| `base64-ng-pem` | Bounded complete RFC 7468 textual encoding parser and generator with labels, boundaries, multiple blocks, and opt-in secret payload release. |
| `@valkyoth/base64-ng-wasm-loader` | Supported byte-only JavaScript/npm loader with separately selected scalar and `simd128` artifacts. |

Subcrates are documented so crate pages are readable, but they belong to the
main `base64-ng` crate family and are not intended as independent protocol
products. Package versions and crates.io links are tracked in
[Crate Version Matrix](docs/CRATE_VERSION_MATRIX.md) so releases can publish
only the crates that changed instead of republishing the whole ecosystem.

The 2.0 JavaScript companion selects an artifact with an embedded SIMD probe
before instantiation and reports the selected posture:

```sh
npm install @valkyoth/base64-ng-wasm-loader
```

```js
import { Codecs, createBase64Ng } from "@valkyoth/base64-ng-wasm-loader";

const base64 = await createBase64Ng();
const input = new TextEncoder().encode("hello");
const encoded = base64.encode(input, Codecs.URL_SAFE_NO_PAD);
const decoded = base64.decode(encoded, Codecs.URL_SAFE_NO_PAD);
base64.dispose();
```

It accepts bytes only, snapshots input, commits `*Into` destinations only after
success, rejects shared/resizable/detached/overlapping storage, and exposes no
secret API or wasm-memory views. See
[`packages/base64-ng-wasm-loader/README.md`](packages/base64-ng-wasm-loader/README.md).

`base64-ng-sanitization` provides extension helpers for
`base64_ng::ct::CtEngine` that decode directly into
`sanitization::SecretBytes<N>` in `no_std`, with `SecretVec` helpers behind its
own `alloc` feature. The `2.0.0` companion uses exact-pinned
`sanitization` `=2.0.3` and exposes `sanitization::ct::Choice` comparison
helpers through `SanitizationCtEqExt`. Locked containers additionally expose
fallible integrity-checked comparison through `LockedSanitizationCtEqExt`.
Heap-backed convenience decode has a 1 MiB default ceiling, reports reservation
failure, and offers const-generic bounded variants for protocol limits.
Stack-backed fixed and staged helpers reject capacities above 1,024 bytes at
compile time.
Built-in checked fixed-size and dynamic decode establish required memory-lock,
dump, and fork controls before plaintext materialization. The 2.0 trait has no
post-construction compatibility defaults: external implementations must define
every locked checked/fill method explicitly. The protected extension API
preserves protection versus canary-integrity failures and offers a bounded
dynamic helper that rejects oversized decoded capacity before mapping
allocation.
The 2.0 companion additionally implements
`SanitizationProtectedDecodeExt` for `Base64<S>`. Its fixed and bounded dynamic
methods establish separate protected staging and destination mappings before
the fixed-work `SecretFrame` sees input. The single-allocation no-copy route is
`Base64::decode_assured`; only that core provider path carries the quarantine,
generation, and fallible-teardown claims from the 2.0 assurance model. See
[`docs/2.0_SANITIZATION_PROTECTED_FILL.md`](docs/2.0_SANITIZATION_PROTECTED_FILL.md).
Enable the companion's
`high-assurance` feature to
decode directly into `sanitization::LockedSecretBytes` or
`sanitization::LockedSecretVec` on supported x86_64 or AArch64 native targets,
using
`sanitization` memory locking plus strict random-canary and assembly-comparison
checks:

```toml
[dependencies]
base64-ng = { version = "2.0.0", default-features = false }
base64-ng-sanitization = { version = "2.0.0", default-features = false }
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

For attacker-controlled dynamic input, enable `alloc` and make the public
decoded-output ceiling explicit:

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationBoundedExt;

let secret = ct::STANDARD
    .decode_secret_vec_bounded::<4096>(b"aGVsbG8=")
    .unwrap();
secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
```

```toml
[dependencies]
base64-ng-sanitization = { version = "2.0.0", features = ["high-assurance"] }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, LockedSanitizationCtEqExt};

let locked = ct::STANDARD
    .decode_locked_secret_bytes_checked::<5>(b"aGVsbG8=")
    .unwrap();

locked
    .try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))
    .unwrap();
assert!(locked
    .try_sanitization_verify(
        b"hello",
        "example authentication decision is public"
    )
    .unwrap());
```

`high-assurance` selects compiled hardening controls. Fixed-size checked decode
and the built-in dynamic checked decode require memory-lock, dump, and fork
controls before plaintext materialization. Inspect `protection_report()` before
relying on non-checked compatibility helpers. Import
`CtDecodeSanitizationProtectedExt` when incident handling must distinguish
protection setup from canary corruption, or when dynamic output needs a
compile-time decoded-capacity limit.

`base64-ng-derive` provides a dependency-free `Base64Secret` derive for tuple
newtypes around the final fixed-size 2.0 secret owner:

```toml
[dependencies]
base64-ng = { version = "2.0.0", default-features = false, features = ["secrets"] }
base64-ng-derive = "2.0.0"
```

```rust
use base64_ng::secret::SecretInput;
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "standard",
    padding = "padded",
    exact_length = 5,
    exposure = "read"
)]
struct ApiKey(base64_ng::secret::SecretArray<5>);

let input = SecretInput::new(b"aGVsbG8=");
let key = ApiKey::decode_base64(&input).unwrap();
assert_eq!(key.expose_secret().as_bytes(), b"hello");
assert_eq!(
    key.encode_base64().unwrap().expose_secret().as_bytes(),
    b"aGVsbG8="
);
```

The derive requires all codec, padding, exact-length, and exposure choices at
the declaration. It generates no ordinary string parsing, implicit slice
conversion, cloning, or equality traits. See
[`2.0_DERIVE_HARDENING.md`](docs/2.0_DERIVE_HARDENING.md).

`base64-ng-serde` provides explicit serialization wrappers without admitting
`serde` into the core package. Its 2.0 adapters use validated codec
specifications, borrowed encoded input where the format permits it, and
explicit string-versus-byte-string serializer behavior:

```toml
[dependencies]
base64-ng-serde = { version = "2.0.0", features = ["secrets"] }
serde = { version = "1.0.229", features = ["derive"] }
```

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct Message {
    #[serde(with = "base64_ng_serde::standard")]
    payload: Vec<u8>,
}
```

Field-level modules are available for `standard`, `standard_no_pad`,
`url_safe`, `url_safe_no_pad`, `mime`, and `pem`. Matching `bounded::*`
modules decode into fixed-capacity `DecodedArray<CAP>` values. Allocating
compatibility adapters enforce a 1 MiB decoded default; every ordinary field
module also offers `deserialize_with_limit::<D, MAX>`, while stack-backed
adapters cap `CAP` at 4096 bytes. Derived encoded-input ceilings run before
full validation. The optional `secret::*` modules decode strict Standard and
URL-safe fields through fixed-work frames into wiping `SecretArray<CAP>`
storage. General Serde format parsing and any parser-owned allocation remain
outside these boundaries.

`base64-ng-bytes` provides fragment-preserving `Bytes`, `Buf`, and `BufMut`
helpers over the sealed 2.0 codec. Owned results are transactional, while
stateful arbitrary-`BufMut` adapters report exact committed prefixes:

```toml
[dependencies]
base64-ng = "2.0.0"
base64-ng-bytes = "2.0.0"
bytes = "1.12.1"
```

```rust
use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_bytes::Base64BytesExt;
use bytes::Bytes;

let encoded = STRICT_STANDARD_PADDED
    .encode_buf(Bytes::from_static(b"hello"))
    .unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");
```

`base64-ng-subtle` provides a sealed `subtle::ConstantTimeEq` integration for
final 2.0 secret owners and views in projects that already admit `subtle`:

```toml
[dependencies]
base64-ng = { version = "2.0.0", features = ["secrets"] }
base64-ng-subtle = "2.0.0"
```

```rust
use base64_ng::{
    STRICT_STANDARD_PADDED,
    secret::{SecretArrayFrame, SecretInput},
};
use base64_ng_subtle::SubtleSecretEq;

let mut frame = SecretArrayFrame::<5>::new(&STRICT_STANDARD_PADDED).unwrap();
frame.update(&SecretInput::new(b"aGVsbG8=")).unwrap();
let decoded = frame.finish().unwrap();
assert!(bool::from(decoded.subtle_ct_eq_public_len(b"hello")));
```

Length mismatch is public and returns `Choice::from(0)` immediately. The
companion intentionally provides no boolean convenience method.

`base64-ng-tokio` provides read-all async helpers and fixed-buffer
`AsyncRead`/`AsyncWrite` adapters over the shared 2.0 incremental core. Prefer
`EncoderReader::new_exact` or `DecoderReader::new_exact` for a framed source
whose adjacent bytes must remain unread. The transactional `*_limited`
read-all helpers may consume one overflow lookahead byte and wipe their private
allocations on return, error, and cancellation. Writer adapters retain accepted
input across backpressure; call `shutdown` to finalize tails before checked
inner recovery. Read-all collection, incremental transformation, and output
delivery consume Tokio cooperative budget between bounded chunks so an
always-ready custom I/O object cannot indefinitely monopolize a runtime worker.
Unlimited helpers still require a trusted finite source:

```toml
[dependencies]
base64-ng = "2.0.0"
base64-ng-tokio = "2.0.0"
tokio = { version = "1.53.1", features = ["io-util"] }
```

```rust
use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader, EncoderWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

# async fn example() -> std::io::Result<()> {
let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(
    &STRICT_STANDARD_PADDED,
    &mut input,
    &mut output,
    1024,
).await?;
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new_exact(&b"helloNEXT"[..], &STRICT_STANDARD_PADDED, 5);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await?;
assert_eq!(streamed, b"aGVsbG8=");

let mut writer = EncoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
writer.write_all(b"hello").await?;
writer.shutdown().await?;
assert_eq!(writer.into_inner(), b"aGVsbG8=");

# Ok(())
# }
```

Disable defaults for embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "2.0.0", default-features = false }
```

Enable admitted encode acceleration on supported `std` targets with the
`simd` feature. The public encode APIs do not change; runtime dispatch selects
an admitted backend only when the CPU and input shape match the admission
scope, otherwise scalar encode is used. The same feature enables admitted
strict decode acceleration for Standard and URL-safe alphabets after
whole-input scalar validation:

```toml
[dependencies]
base64-ng = { version = "2.0.0", features = ["simd"] }
```

```rust
use base64_ng::{runtime, STANDARD};

let encoded = STANDARD.encode_string(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");

let decoded = STANDARD.decode_vec(encoded.as_bytes()).unwrap();
assert_eq!(decoded, b"hello");

let report = runtime::backend_report();
println!("encode: {}", report.encode_backend.backend);
println!("strict decode: {}", report.strict_decode_backend.backend);
println!("secret decode: {}", report.secret_decode_backend.backend);
println!("Wasm artifact: {}", report.wasm_artifact_posture.as_str());
println!("Wasm runtime: {}", report.wasm_runtime_posture.as_str());
```

## Canonical 2.0 Examples

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let input = b"hello";
let mut encoded = [0u8; 8];
let written = STRICT_STANDARD_PADDED
    .encode_into(input, &mut encoded)
    .unwrap();
assert_eq!(&encoded[..written], b"aGVsbG8=");

let mut decoded = [0u8; 5];
let written = STRICT_STANDARD_PADDED
    .decode_into(&encoded, &mut decoded)
    .unwrap();
assert_eq!(&decoded[..written], input);
```

In-place encoding:

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let mut buffer = [0u8; 8];
buffer[..5].copy_from_slice(b"hello");
let encoded_len = STRICT_STANDARD_PADDED
    .encode_in_place(&mut buffer, 5)
    .unwrap();
assert_eq!(&buffer[..encoded_len], b"aGVsbG8=");
```

Canonical `encode_into` and `decode_into` are transactional: every returned
error leaves the complete destination unchanged. Incremental and in-place APIs
instead report exact committed progress; choose the contract that matches the
surrounding protocol.

Heapless incremental encoding:

```rust
use base64_ng::{Status, STRICT_STANDARD_PADDED};

let mut state = STRICT_STANDARD_PADDED.encoder();
let mut output = [0u8; 8];
let step = state.update(b"hello", &mut output).unwrap();
let mut written = step.progress().output_produced();
let final_step = state.finish(&mut output[written..]).unwrap();
written += final_step.progress().output_produced();
assert_eq!(final_step.status(), Status::Complete);
assert_eq!(&output[..written], b"aGVsbG8=");
```

Heapless incremental strict decoding uses the same progress contract:

```rust
use base64_ng::{Status, STRICT_STANDARD_PADDED};

let mut state = STRICT_STANDARD_PADDED.decoder();
let mut output = [0u8; 5];
let step = state.update(b"aGVsbG8=", &mut output).unwrap();
let mut written = step.progress().output_produced();
let final_step = state.finish(&mut output[written..]).unwrap();
written += final_step.progress().output_produced();
assert_eq!(final_step.status(), Status::Complete);
assert_eq!(&output[..written], b"hello");
```

Allocation-free formatting, append rollback, and chunk iteration:

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let display = STRICT_STANDARD_PADDED.display(b"hello").unwrap();
assert_eq!(format!("{display}"), "aGVsbG8=");

let mut appended = String::from("prefix:");
STRICT_STANDARD_PADDED
    .encode_append(b"hello", &mut appended)
    .unwrap();
assert_eq!(appended, "prefix:aGVsbG8=");

let chunks = STRICT_STANDARD_PADDED
    .encoded_chunks(b"hello")
    .unwrap()
    .map(|chunk| chunk.as_bytes().to_vec())
    .collect::<Vec<_>>();
assert_eq!(chunks, [b"aGVs".as_slice(), b"bG8="].map(<[u8]>::to_vec));
```

Compile-time encoding:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};

const HELLO: [u8; 8] = match STRICT_STANDARD_PADDED.encode_array(b"hello") {
    Ok(output) => output,
    Err(_) => panic!("reviewed const encode failed"),
};
const URL_BYTES: [u8; 3] = match STRICT_URL_SAFE_UNPADDED.encode_array(b"\xfb\xff") {
    Ok(output) => output,
    Err(_) => panic!("reviewed const encode failed"),
};

assert_eq!(&HELLO, b"aGVsbG8=");
assert_eq!(&URL_BYTES, b"-_8");
```

Stable Rust cannot yet express the encoded length as the return array length
directly, so `encode_array` uses the destination array type supplied by the
caller. A wrong output length returns `ConstTransformError` and can fail during
const evaluation when matched as above.
Use `encode_array` for fixed-size static values, not for runtime data whose
size is controlled by an attacker.

Compile-time strict decoding:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};

const HELLO: [u8; 5] = match STRICT_STANDARD_PADDED.decode_array(b"aGVsbG8=") {
    Ok(output) => output,
    Err(_) => panic!("reviewed const decode failed"),
};
const URL_BYTES: [u8; 2] = match STRICT_URL_SAFE_UNPADDED.decode_array(b"-_8") {
    Ok(output) => output,
    Err(_) => panic!("reviewed const decode failed"),
};

assert_eq!(&HELLO, b"hello");
assert_eq!(&URL_BYTES, b"\xfb\xff");
```

`decode_array` is strict and returns `Result` for malformed input, padding
errors, and undersized output arrays. It is useful for fixed static Base64
literals and does not replace the `ct` APIs for secret-bearing decode.

For runtime values with compile-time capacity ceilings, use ordinary bounded
arrays:

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let encoded = STRICT_STANDARD_PADDED
    .encode_bounded::<8>(b"hello")
    .unwrap();
let decoded = STRICT_STANDARD_PADDED
    .decode_bounded::<5>(encoded.as_bytes())
    .unwrap();
assert_eq!(decoded.as_bytes(), b"hello");
```

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
use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};

STRICT_STANDARD_PADDED.validate(b"aGVsbG8=").unwrap();
assert!(STRICT_STANDARD_PADDED.validate(b"aGVsbG8").is_err());
STRICT_URL_SAFE_UNPADDED.validate(b"-_8").unwrap();
assert!(STRICT_URL_SAFE_UNPADDED.validate(b"+/8").is_err());
```

For line-wrapped or spaced legacy inputs, use the explicit legacy profile:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, legacy};

let mut decoded = [0u8; 5];
let written = legacy::ASCII_WHITESPACE
    .decode_into(
        &STRICT_STANDARD_PADDED,
        b" aG\r\nVs\tbG8= ",
        &mut decoded,
    )
    .unwrap();
assert_eq!(&decoded[..written], b"hello");
```

Exact WHATWG forgiving decode is a separate ordinary policy:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, web};

assert_eq!(web::FORGIVING.decode_to_vec(" Z h = = ").unwrap(), b"f");
assert!(STRICT_STANDARD_PADDED.decode_to_vec(b" Z h = = ").is_err());
```

Forgiving and legacy policies are intentionally unavailable to secret frames.

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

The 2.0 secret surface keeps classified encoding in a separate
bounded scalar state and wiping owner:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretInput};

let input = SecretInput::new(b"secret");
let encoded = STRICT_STANDARD_PADDED
    .encode_secret_array::<8>(&input)
    .unwrap();

assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");
```

Built-in secret alphabets use arithmetic mapping; validated custom alphabets
use a fixed 64-entry scan. Encoded output stays redacted and wiping until the
caller explicitly exposes or declassifies it. See
[`docs/2.0_SECRET_ENCODING.md`](docs/2.0_SECRET_ENCODING.md).

Allocation-specific assured operations require both a token and one protected
owner; an ordinary mutable slice cannot substitute for that owner:

```rust
use base64_ng::{
    STRICT_STANDARD_PADDED,
    assurance::{AssuranceContext, BestEffortProvider, ProtectedSecret, ProviderLimits},
    secret::SecretInput,
};

let context = AssuranceContext::new();
let token = context.best_effort_token();
let provider = BestEffortProvider::<1>::new(ProviderLimits {
    max_identities: 1,
    max_logical_bytes: 8,
    max_effective_pages: 2,
    max_registry_entries: 1,
    max_retry_attempts: 1,
    max_maintenance_work: 1,
    page_size: 8,
})?;
let allocation = ProtectedSecret::try_new(&provider, &token, 8)?;
let encoded = STRICT_STANDARD_PADDED.encode_assured(
    &token,
    allocation,
    &SecretInput::new(b"secret"),
)?;

assert_eq!(encoded.expose_secret().as_bytes(), b"c2VjcmV0");
let operation = encoded.operation_report(&token)?;
assert_eq!(operation.snapshot().operation, "secret-encode");
assert_eq!(operation.snapshot().wipe, "wipe-not-completed");

let cleanup = encoded.try_close()?;
assert_eq!(cleanup.snapshot().lifecycle, "closed");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The dependency-free default provider is finite, volatile, and best-effort. It
does not lock pages or provide persistent teardown recovery. Reviewed deployed
providers and attested tokens use an explicit unsafe extension boundary. See
[`docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md`](docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md).
Per-operation backend, token, allocation, and teardown reporting is documented
in [`docs/2.0_OPERATION_REPORTING.md`](docs/2.0_OPERATION_REPORTING.md).

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

New 2.0 code validates and owns custom alphabets before constructing a sealed
runtime codec:

```rust
use base64_ng::CodecBuilder;

let codec = CodecBuilder::from_table(
    *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
)
.unwrap()
.build()
.unwrap();
let encoded = codec.encode_to_string(b"hello").unwrap();
assert_eq!(codec.decode_to_vec(encoded.as_bytes()).unwrap(), b"hello");
```

`ValidatedAlphabet` and `CodecBuilder` reject duplicate, forbidden, padded, or
wrong-length tables before a codec exists. For custom tables, secret operations
use a deliberately conservative fixed 64-entry scan for every emitted or
decoded symbol. Benchmark this tradeoff before using custom alphabets for
untrusted high-volume traffic.

The historical `define_alphabet!` and `Alphabet` trait remain available only as
reviewed 1.x compatibility surfaces. New code should use the validated 2.0
value so policy construction and ownership are explicit.

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
Ordinary public-data builds require no cleanup opt-in. Enable `secrets` when
requesting the 2.0 secret capability; on WASM that capability also requires
`allow-wasm32-best-effort-wipe`. Secret builds on unsupported native targets
similarly require `allow-compiler-fence-only-wipe` after platform review.
Neither acknowledgement is a high-assurance upgrade.

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
secret buffers. This 1.x compatibility type still performs best-effort cleanup
in ordinary builds, but only an explicit `secrets` build requests the 2.0
fail-closed secret policy and its `allow-wasm32-best-effort-wipe`
acknowledgement.
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
  is Rust `1.97.1`. New deployments should prefer the latest tested stable
  Rust, currently Rust `1.97.1`.
- `no_std`-capable core; the default convenience feature set enables `alloc`
  and `std`, and `default-features = false` keeps the core freestanding.
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
- Every ordinary accelerated operation/backend pair must pass a direct
  known-answer test before first use. Runtime health reports expose its
  generation and `never-run`, `testing`, `healthy`, or `quarantined` state;
  malformed input cannot quarantine a backend. `checked-backend` adds bounded
  redundant scalar comparison and scalar retry without exposing a suspect
  chunk.
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
- [docs/2.0_SYNCHRONOUS_IO.md](docs/2.0_SYNCHRONOUS_IO.md) defines exact
  prefix commitment, retry, framing, third-party writer failure, and bounded
  secret-frame rules for the synchronous adapters rebuilt on the 2.0
  incremental core.
- [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) defines the dependency
  admission bar for any future external crate.
- `runtime::backend_report()` exposes the active admitted backend, detected
  candidate, candidate detection mode, SIMD feature status, security posture,
  and a conservative unsafe-boundary posture flag for audit logging. In 2.0,
  non-scalar active values describe admitted encode dispatch, and
  strict decode dispatch is exposed separately through
  `BackendReport::active_decode_backend()`. The
  unsafe-boundary flag is true only when the reserved `simd` feature is
  disabled; SIMD-enabled builds must rely on the release evidence scripts for
  boundary validation. On `no_std`, acceleration requires complete compile-time
  target-feature evidence plus the atomic health latch; otherwise execution is
  scalar. Unsafe deployment attestation is represented by a thread-bound,
  generation-bound `StaticBackendToken` and does not bypass KAT or quarantine.
  Its `encode_standard` and `encode_url_safe` methods execute the rewritten
  SSSE3/SSE4.1, AVX2, or AVX-512 hot path when that exact token remains healthy.
  Its `decode_standard` and `decode_url_safe` methods execute direct
  SSSE3/SSE4.1, AVX2, or AVX-512 strict decode. Automatic x86 strict decode
  remains on SSSE3/SSE4.1 or AVX2 because retained AVX-512 measurements missed
  the frozen performance margin; exact static-token calls may still use AVX-512
  from one 64-byte encoded block. Other token backends and invalidated
  generations use scalar execution. The complete frozen policy is in
  [`docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md`](docs/2.0_DISPATCH_AND_PERFORMANCE_MATRIX.md).
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
[docs/CONSTANT_TIME.md](docs/CONSTANT_TIME.md). The exact 2.0 pre-gate,
post-gate, target, timing, and generated-code evidence boundary is in
[docs/2.0_TIMING_AND_CODEGEN.md](docs/2.0_TIMING_AND_CODEGEN.md).
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

This runs the host tests, all-feature tests, clippy, direct NEON encode/decode
evidence, backend evidence, SIMD feature-bundle checks, and SIMD admission
validators on the real AArch64 host. To include the retained NEON performance
campaign, set `BASE64_NG_RUN_COMMIT29_PERF=1`.

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

Manage resumable one-hour release campaigns across local and SSH workers:

```sh
scripts/manage-fuzz-evidence.py
```

The ignored SQLite session pins one clean commit, tracks all 18 fuzz targets
plus the native RISC-V admission campaign, and retrieves and validates remote
evidence before final aggregation. See
[docs/FUZZING.md](docs/FUZZING.md) for the operator workflow and trust boundary.

Compile and audit the isolated performance harness directly:

```sh
scripts/check_perf.sh
```

Run the complete reproducible campaign. The perf crate measures production
auto dispatch, scalar, and every exact backend available on the host:

```sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

The campaign runs correctness before and after measurement, captures two raw
sample sets, validates reproducibility, and records exact-pinned
`base64 0.23.0` and `base64ct 1.8.3` comparisons only for matching canonical
slice semantics. See [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

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

- Keep the core dependency graph empty and isolate optional ecosystem
  dependencies in companion crates.
- Correctness first, speed second, unsafe last.
- The scalar implementation is the reference behavior.
- SIMD must prove equivalence to scalar behavior across fuzzed and deterministic inputs.
- Constant-time claims require empirical timing evidence, generated-code
  review, and explicit documented exclusions.
- Compatibility modes must be visible in the type/API surface.
- Release evidence belongs in the repository and CI, not in memory.

## Contributing And Releases

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution rules and [docs/RELEASE.md](docs/RELEASE.md) for the maintainer release checklist.
