# Borrowed Base64 Checkpoint

Commit 16 adds `Base64Ref<'a, S: Codec>` for ordinary encoded input that callers
want to validate once and decode repeatedly without copying. This is a
development checkpoint; external review and CI acceptance remain pending.

```rust
use base64_ng::{Base64Ref, STRICT_STANDARD_PADDED};

let input = b"aGVsbG8=";
let view = Base64Ref::parse(STRICT_STANDARD_PADDED, input).unwrap();
assert_eq!(view.as_bytes().as_ptr(), input.as_ptr());
let mut output = [0; 5];
assert_eq!(view.decoded_len(), 5);
assert_eq!(view.decode_into(&mut output), Ok(5));
assert_eq!(&output, b"hello");
```

## API And Ownership

The type is exported from the root, `v2`, and ordinary `prelude`.

| Methods | Contract |
| --- | --- |
| `parse(codec, input)` | Fully validate using Auto; retain an immutable borrow, no copy or allocation |
| `parse_with_validation(codec, input, policy)` | Retain an explicit validation policy |
| `as_bytes`, `AsRef<[u8]>`, `len`, `is_empty` | Expose ordinary encoded bytes and encoded length |
| `decoded_len` | Return the exact checked output length established during construction |
| `codec`, `settings`, `validation` | Inspect the retained codec, owned settings snapshot and validation policy |
| `decode_into` | Decode into caller storage; every returned error leaves the entire destination unchanged |
| `decode_to_vec`, `decode_to_vec_with_limit` | Optional `alloc` output, with fallible reservation and exact output limit |

Fields are private and there is no unchecked constructor, mutable input access,
or conversion that asserts validity without checking. The immutable borrow
prevents source mutation and lifetime escape in safe Rust. The view is not
`Clone` or `Copy`; repeated decoding borrows it. Runtime custom alphabets and
relaxed padding/tail policies retain their exact settings, not a substitute
strict preset. Debug output contains lengths and policy, not payload bytes.

`Base64String::as_base64_ref()` and `as_base64_ref_with_validation(policy)`
require `S: Clone` and are fallible: they validate before borrowing the owner's
bytes. Encoding is not used as an unchecked proof of decode acceptance. The
owner itself gains no cached proof; its existing `decode()` still validates
on each call. The owner must remain alive and unmodified while its view is used.

## Reuse And Execution Controls

The private preflight proof still binds one input, owned settings and checked
layout. An internal reborrow can retain only those same values; each writer
consumes its own reborrowed proof. It cannot replace the source or settings.

- Auto reuses a successful grammar proof. A vector-validated proof also retains
  the validator's healthy operation/backend generation. A changed snapshot
  requires reference revalidation and matching decoded length before writing.
- Construction never retains an already unhealthy or saturated-generation
  vector result: it establishes a reference proof instead.
- ScalarReference validates at construction and on every decode, including
  before capacity, output-limit and reservation checks. It is not silently
  converted into a cached Auto policy.
- The writer still checks backend availability/health. Rejected or corrupt
  vector output is quarantined and fully overwritten through scalar recovery.
  `checked-backend` retains independent validation at initial preparation and
  independent output comparison on every accelerated decode. Reuse does not
  mean zero reference work in checked builds.
- No mutable cache, new lock or global state is introduced. A stale vector
  snapshot is not refreshed in the view, so later decodes continue to revalidate.
  A view initially prepared on a scalar path may conservatively retain that
  writer even if another backend later becomes eligible.

These health checks are snapshots, not synchronous revocation of in-flight
operations. The view is not a static deployment token or execution attestation.
It does not change static-token methods, progressive/incremental decoding,
in-place mutation or the separate CT/secret APIs. Ordinary bytes and decoded
allocations are not automatically wiped; enforce application input-size limits.

## Verification

`sh scripts/check-2.1-borrowed-view.sh` covers active Rust 1.99.0 and MSRV 1.90.0,
core-only/alloc/std, SIMD and checked builds, Clippy, and safety policies.
Compile-fail docs reject mutation, lifetime escape and private-field forgery.
Runtime tests cover all four strict profiles, custom/relaxed settings, malformed
construction, exact capacity/suffix preservation, repeated decoding, allocation
limits and injected reservation failure. Counters distinguish cached grammar
work from mandatory reference revalidation and prove allocation-free parsing
and caller-buffer decoding.

Fault injection checks validation rejection, checked false acceptance, writer
unavailability, partial-store rejection and checked output corruption. A native
x86 child quarantines the real selected backend after view construction and
verifies reference revalidation and scalar recovery; a separate synthetic test
exercises changed generations and inconsistent cached lengths.

Local full workspace all-feature release tests, focused Miri, and checked ARM
QEMU/WASI Wasmtime tests passed. The existing runtime-codec fuzz target now
checks repeated borrowed decoding and constructor diagnostics; a seeded
10,000-execution smoke passed. These bounded checks do not replace the final
long fuzz campaign or native ARM, Mac, Windows and RVV release evidence.
Core-only tests, workspace Clippy, RISC-V checked test cross-compilation and
bare-metal Thumb core-only compilation also passed. Both loader artifacts are
rebuilt with synchronized integrity pins, and all 26 Node loader tests pass;
this adds no JavaScript API or new browser performance claim.
The repository `scripts/checks.sh` sequence also passed across the initial run
and resumed remainder. Its isolation gate required a host-permission rerun
because Codex's sandbox could not access the systemd user bus; that rerun passed.
This includes frozen API compatibility, Chromium/Firefox dispatch, package
installation, documentation builds, Clippy, dependency policy and RustSec audit.

## Development Measurements

The ignored `borrowed_view_same_process_benchmark` separately measures complete
one-shot decode, Auto view reuse with parsing excluded, and parse-only cost.
Seven samples alternate mode order; input/output allocation is outside timing.
Output is verified. Run on the Ryzen 9 9950X3D desktop with Rust 1.99.0,
`std,simd`, warmed backend health and CPU 2 affinity:

```sh
taskset -c 2 cargo test --locked --release --no-default-features \
  --features std,simd --lib borrowed_view_same_process_benchmark \
  -- --ignored --nocapture --test-threads=1
```

| Raw payload | One-shot median | Reused decode median | Parse-only median | Reuse speedup |
| --- | --- | --- | --- | --- |
| 32 B | 24.0 ns | 15.0 ns | 29.9 ns | 1.60x |
| 4 KiB | 548 ns | 389 ns | 238 ns | 1.41x |
| 1 MiB | 111 us | 78.0 us | 33.6 us | 1.43x |

Parsing must be amortized: approximate break-even is four decodes for 32 B and
two for the larger cases here. A single parse-plus-decode is not shown as a
faster one-shot call. Results cover only strict Standard padded Auto, without
checked-backend; tiny timings and unlocked desktop clocks limit precision.
They are development observations, not hardware admission or universal gains.

Two separate paired comparisons use the unchanged public-call harness against
parent `d3c84b2`, with identical Rust 1.99.0 flags and CPU affinity. Each has
168 cases across four profiles, six operations and seven payload sizes, with
seven alternating-order sample pairs. The first has 164 cases within 5%, two
inconclusive, one improvement signal and one regression signal (32-byte
Standard-unpadded historical owned decode: roughly 37.4 to 39.2 ns). That
regression did not repeat: the second has 162 within 5%, three inconclusive,
two noisy and one improvement signal, with no regression signal. All bulk
cases were within 5% in the first capture; the second includes an inconclusive
64 KiB historical case and a noisy 1 MiB historical case. No persistent
regression is established, but this is not a universal no-regression guarantee.
No one-shot algorithm or dispatch threshold changes in this checkpoint.

Local raw data under `target/release-evidence/2.1-commit16-borrowed/`:

| File | SHA-256 |
| --- | --- |
| `reuse.log` | `df7f39e745a6f5306a67d61029819c1f09341e9f56ddd081531aecff4bb30497` |
| `parent-comparison.json` | `f9a717ab5f434c126ef703f95bc2efbed8a218311af343eec61f065e4eff9cbd` |
| `parent-comparison-repeat.json` | `e5d0ed26f69b16aebe8cde18ebe9a67044198ff745311ef5e3a50960ff91ef72` |

Parent executable SHA-256:
`ff6e40efdc2bb66511b8b094c0000d23da13837cffa9c94a6b180f14d4325ecf`.
Candidate executable SHA-256:
`43d816e95e75072933c6f21ba1f0aa1213fed0efe3f31e5c68b8944e4284b206`.
Reproduce by building the parent and this source with
`RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo build --locked --release --manifest-path perf/public-api/Cargo.toml --no-default-features --features std,simd`,
then run `scripts/measure-2.1-encode-public-api.py BASELINE CANDIDATE --direction decode`.
