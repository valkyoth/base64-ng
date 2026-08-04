# Release Evidence

Repository shell gates use baseline POSIX utilities and do not require
`ripgrep`. Constant-time assembly symbol evidence recognizes both GNU/ELF and
Apple/Mach-O function-definition labels; this parser is covered by
`scripts/test-ct-asm-symbols.sh`.

`base64-ng` treats release evidence as part of the artifact, not as an informal
local habit. The release gate generates and verifies evidence that downstream
users can inspect before adopting a version.

## Protocol Registry Freeze

`scripts/check-protocol-registry.sh` validates the versioned
`protocol-registry/v1` evidence set. It requires complete entries for all
specialized companion claims and every named core configuration, verifies
source/API/model/corpus hashes, executes independent grammar models against
the production APIs, repeats pinned `base64` 0.23.0 and `base64ct` 1.8.3
comparisons, checks dependency licenses/advisories, runs MSRV compilation when
available, and proves retained evidence is excluded from published packages.
Mutation tests establish fail-closed behavior for missing names, changed
source bytes, altered corpus decisions, and oracle contamination.

The 2.0 checkpoint program is governed by
[`2.0_GOVERNANCE.md`](2.0_GOVERNANCE.md). Each numbered checkpoint records
its exact commit, verification commands, tool and target identity, skips, and
external pentest coverage in the authoritative
[`2.0.0-release-plan.md`](../2.0.0-release-plan.md). Generated artifacts remain
under `target/release-evidence/`; the final pre-seal checkpoint record retains
checksums or immutable workflow/artifact references for evidence too large to
commit. Commit 55 changes only the permanent pentest report. Intermediate
pentests may cover contiguous checkpoint batches and remain working evidence;
the normalized permanent GitHub pentest report is added for the final 2.0.0
candidate.

`scripts/stable_release_gate.sh candidate` runs the strict evidence campaign
before Commit 55. `scripts/finalize-release-evidence.sh` rejects missing,
dirty-tree, or stale-source Miri, sanitizer, release-duration fuzz, dudect,
normal/advanced Kani, assembly, native-hardware, and SBOM artifacts and writes
`target/release-evidence/FINAL-MANIFEST.txt`. It also requires explicit success
for every Miri and sanitizer scope, dudect, backend evidence, both Kani sets,
and every named fuzz target. The source section must occur exactly once, and
its commit and tree-state keys must be exact, anchored, singleton values;
duplicate, conflicting, prefixed, suffixed, or substring matches fail closed.
Fuzz, dudect, sanitizer, reproducibility, and final index manifests are
published atomically only after successful completion, so an interrupted or
failed run cannot leave a final-looking manifest. Release mode also pins the
dudect acceptance threshold to `10`; the finalizer rejects missing, duplicate,
malformed, or weakened threshold records. Release mode repeats this exact
campaign for the report-only Commit 55 tag candidate.

Commit 51 completes the isolated fuzz and property inventory. Eighteen targets
cover ordinary/runtime codecs, forced native backends, incremental/in-place
state, sync and async adapters, every protocol companion, and volatile
assurance teardown. `scripts/check-2.0-fuzz-campaigns.sh` runs the deterministic
property, panic, cancellation, cleanup, and unsafe-provider isolation suite;
`scripts/check_fuzz.sh` records bounded or release-duration LibFuzzer evidence.

Commit 52 completes the timing and generated-code evidence boundary.
`scripts/validate-2.0-timing-boundaries.sh` preserves fixed-scan secret loops,
the result gate, cleanup revision, scalar dispatch isolation, and exact claim
wording. Release and fat-LTO assembly retain the equality, accumulation,
secret encode/decode, gate, and cleanup symbols. The dudect-style harness
separates thresholded equal-work classes from informational public-length
scaling and records source, lockfile, compiler, target, CPU, flags, and binary
checksums. See
[`2.0_TIMING_AND_CODEGEN.md`](2.0_TIMING_AND_CODEGEN.md).

Commit 2 freezes the machine-generated `v1.3.9` public API snapshots and the
[`2.0_API_MIGRATION_LEDGER.md`](2.0_API_MIGRATION_LEDGER.md) disposition rules.
`scripts/check-api-snapshots.sh`, `scripts/check-2.0-migration-smoke.sh`, and
`scripts/check-2.0-feature-contract.sh` prevent accidental inventory,
canonical-name, and Cargo feature-unification drift.

The release-owner-approved pre-seal usability reopening adds the ordinary
policy-carrying `Base64String<S>` and focused ordinary prelude. Focused tests
cover strict and runtime policies, malformed text, output limits, visible
formatting, and policy retention. The migration smoke compiles the external
surface, while the secret-storage gate proves `SecretInput` cannot enter the
new ordinary owner without explicit exposure. The regenerated 2.0 snapshot
records the exact added API before the final full-range pentest.

Commit 4 adds the exact RFC 4648 source lock, errata and requirements ledgers,
independent test-only oracle, versioned cross-crate semantic corpus, and the
complete Rust 1.90.0 capability matrix. `scripts/verify-rfcs.sh` is offline;
network comparison is an explicit maintainer operation.

Commit 5 adds an owned, allocation-free validated alphabet value whose encode
and decode mappings derive from one immutable table. The evidence in
`scripts/check-2.0-alphabet.sh` covers const acceptance and rejection, all
duplicate positions, every forbidden byte and position, runtime/const mapping
parity, the crate-owned fixed secret scan, and Rust 1.90.0 compatibility.
Kani proves the bounded per-position constructor accesses separately from the
exhaustive semantic tests. See
[`2.0_VALIDATED_ALPHABETS.md`](2.0_VALIDATED_ALPHABETS.md).

Commit 18 activates the dependency-free `secrets` storage boundary. Its gate
proves that secret owners require explicit exposure, generated secret newtypes
have no implicit `AsRef<[u8]>`, formatting is redacted, and borrowed, fixed,
and heap storage follow their documented cleanup and declassification rules.
See [`2.0_SECRET_STORAGE_AND_EXPOSURE.md`](2.0_SECRET_STORAGE_AND_EXPOSURE.md).

Commit 19 adds bounded secret decode frames. Their dedicated gate covers
fixed-work symbol scans, delayed final output, opaque malformed input,
capacity/overlap preflight, cleanup, stack-limit compile failures, and optional
Kani frame proofs. See
[`2.0_SECRET_DECODING.md`](2.0_SECRET_DECODING.md).

Commit 20 adds bounded secret encode frames. Built-in alphabets use private
arithmetic mapping, custom alphabets use one fixed 64-entry scan per output
symbol, and every frame owns or borrows complete wiping output storage before
accepting classified input. The dedicated gate includes compile-fail ordinary
sink checks, exhaustive mapping and chunking tests, optional Kani proofs,
dudect-style encode classes, and release/LTO assembly symbol evidence. See
[`2.0_SECRET_ENCODING.md`](2.0_SECRET_ENCODING.md).

Commit 21 separates ordinary portability from secret cleanup policy. Ordinary
native and WASM codecs need no cleanup acknowledgement; `secrets` enables
separate scalar secret owners and states with explicit best-effort lifecycle
limits. Staged in-place decode now owns staging through an unwind guard,
partial-state drop models cancellation cleanup, and `SecretVec` replacement
wipes the displaced allocation. The high-assurance cfg is an eligibility gate
that requires `secrets`, rejects SIMD, and rejects unsupported or unattested
speculation postures; it does not attest protected storage. See
[`2.0_SECRET_CAPABILITY_POLICY.md`](2.0_SECRET_CAPABILITY_POLICY.md).

Commit 22 adds generation-bound `BestEffort` and `Attested` assurance tokens,
allocation-specific protected typestates, a finite volatile default provider,
and one ordered teardown path shared by explicit close and `Drop`. Resources
and a quarantine slot are reserved before plaintext materialization. Fault
tests pin wipe-before-protection-before-accounting-before-disposal ordering,
failed-wipe short-circuiting, conservative unknown-protection reporting, and
terminal non-addressable tombstones for indeterminate disposal. Compile-fail
tests pin token non-copyability, private typestate transitions, allocation-
specific operation arguments, and the protected auto-trait matrix on current
Rust and Rust 1.90.0 when installed. See
[`2.0_ASSURANCE_AND_PROTECTED_MEMORY.md`](2.0_ASSURANCE_AND_PROTECTED_MEMORY.md)
and `scripts/check-2.0-assurance.sh`.

Commit 23 separates ordinary encode, ordinary strict decode, and secret decode
backend reporting. Stable snapshots bind backend-health and assurance
generations without promoting them into physical protection. Exact allocation
reports, cleanup reports, cleanup errors, and provider reports retain wipe,
protection, accounting, lifecycle, pending stage/substage, and allocation
presence as independent redacted fields. Qualifying Standard operations are
correlated with test-only dispatch counters. See
[`2.0_OPERATION_REPORTING.md`](2.0_OPERATION_REPORTING.md) and
`scripts/check-2.0-operation-reporting.sh`.

Commit 24 admits ordinary accelerated backends only after direct known-answer
tests and records independent operation/backend health generations. Its gate
fault-injects failed and panicking initialization, impossible backend lengths,
and checked-output mismatches; proves permanent quarantine and malformed-input
separation; compiles scalar and static `no_std` postures; and validates the
unsafe static-token boundary. `checked-backend` uses bounded stack staging and
scalar retry without exposing the suspect chunk. See
[`2.0_BACKEND_HEALTH.md`](2.0_BACKEND_HEALTH.md) and
`scripts/check-2.0-backend-health.sh`.

Commit 25 replaces SSSE3/SSE4.1 and AVX2 encode staging prototypes with direct
exact-width loads, byte-shuffle alphabet mapping, batched block loops, and
compiler-emitted AVX transition handling. Its gate exhausts every byte value at
every fixed-block position, checks all tails through multiple blocks, exercises
both alphabets, both padding modes, and static tokens, and cross-builds the
`no_std` target-feature variants:

```sh
scripts/check-2.0-x86-encode-hot-paths.sh
BASE64_NG_RUN_COMMIT25_PERF=1 \
  scripts/check-2.0-x86-encode-hot-paths.sh
```

The optional focused benchmark compares exact SSSE3/AVX2 backends against
scalar only at admitted input sizes and defaults to a `1.02` median throughput
ratio. The `x86_encode` fuzz target forces each runtime-supported backend
through its static token and compares Standard and URL-safe padded and unpadded
output with the independent `base64` oracle. Miri separately exercises the
safe automatic-dispatch wrapper and fallback boundary; Miri does not interpret
the x86 intrinsics, so real-host tests and generated assembly remain the
evidence for those instructions. The pentest follow-up expands the startup KAT
to cover all 64 alphabet indices plus `0x00`/`0xff` in each three-byte
position, runs static-token `checked-backend` smoke on both x86 backends,
asserts logical output sentinels remain untouched, and structurally rejects
ordinary SIMD references from secret decode modules. The second follow-up
rejects transient `NeverRun`/`Testing` scalar policy snapshots and adds an
unwind guard plus no-std execution test that quarantines an escaping KAT panic.

Commit 26 extends the same production boundary to AVX-512 VBMI. The kernel uses
an exact 48-byte masked load, direct VBMI expansion and alphabet lookup, a
multi-block loop, and one ZMM cleanup at the call boundary. Its gate adds
exhaustive AVX-512 byte-position and tail comparison, forced-backend fuzz
coverage, checked and unchecked static `no_std` execution, generated assembly,
and exact-backend performance validation:

```sh
scripts/check-2.0-x86-encode-hot-paths.sh
BASE64_NG_RUN_COMMIT26_PERF=1 \
  scripts/check-2.0-x86-encode-hot-paths.sh
```

Automatic x86 encode uses AVX2 below the retained two-block AVX-512 threshold:
SSSE3/SSE4.1 covers 12–23 bytes, AVX2 covers 24–191 bytes, and AVX-512 begins at
192 bytes. The exact static and evidence contracts retain a 48-byte AVX-512
minimum. Local real-hardware evidence on an AMD Ryzen 9 9950X3D shows AVX2 is
equal or slightly faster around one AVX-512 block while AVX-512 wins from two
blocks and widens its advantage on larger inputs. A second AVX-512
microarchitecture remains an explicit pre-release evidence requirement; no
portable throughput claim is made from this single host.

Commit 27 replaces SSSE3/SSE4.1 and AVX2 strict-decode staging with direct
vector classification, mapping, packing, and exact-width stores. Whole-input
scalar validation still runs before SIMD and remains the sole authority for
canonicality, detailed errors, decoded length, and no-write-on-error behavior.
The gate exhausts every valid and invalid byte in every vector lane, all tails
through 257 raw bytes, every malformed source position in a 256-byte encoded
frame, the static `no_std` token in checked and unchecked modes, the dedicated
forced-backend fuzz target, unsafe boundaries, and the mutation-tested focused
performance validator:

```sh
scripts/check-2.0-x86-decode-hot-paths.sh
BASE64_NG_RUN_COMMIT27_PERF=1 \
  scripts/check-2.0-x86-decode-hot-paths.sh
```

Local AMD Ryzen 9 9950X3D evidence exceeds scalar at every admitted tested
SSSE3/SSE4.1 and AVX2 size for Standard and URL-safe, padded and unpadded
input. This is exact-host evidence; another microarchitecture remains useful
before broad performance wording.

Commit 28 replaces the AVX-512 VBMI strict-decode staging prototype with a
direct 64-byte ASCII classifier, 6-bit mapper, multiply-add packer, VBMI lane
compactor, and exact masked 48-byte caller-output store. The multi-block loop
clears ZMM state once at the call boundary. Whole-input scalar validation
remains authoritative for canonicality, detailed errors, required length,
padding, and transactional rejection. The gate adds exhaustive 64-lane valid
and invalid classification, malformed-position and tail tests, forced-backend
fuzz coverage, checked and unchecked static `no_std` AVX-512 execution,
generated assembly checks, and exact-backend performance validation:

```sh
scripts/check-2.0-x86-decode-hot-paths.sh
BASE64_NG_RUN_COMMIT28_PERF=1 \
  scripts/check-2.0-x86-decode-hot-paths.sh
```

Commit 34 supersedes the provisional crossover. Automatic x86 strict decode
selects SSSE3/SSE4.1 from 16 encoded bytes and AVX2 from 32 encoded bytes.
AVX-512 retains exact static/evidence execution from its 64-byte block minimum,
but has no automatic threshold. A retained 15-sample AMD Ryzen 9 9950X3D
campaign found a weakest AVX-512/AVX2 median ratio of 1.0166, below the frozen
1.02 requirement; a separate seven-sample campaign also missed the requirement.
The final matrix therefore makes no automatic AVX-512 decode performance claim.

The Commit 20 pentest follow-up also pins fail-closed forward-progress guards
for WHATWG and legacy one-shot loops, immediate pending-state cleanup on
secret decode failure, checked secret-array frame construction, and the
1,368-byte stack ceiling for `SecretArrayEncoder`.

The subsequent evidence-integrity follow-up isolates CT, SIMD, and wasm
generated-code builds from persistent Cargo targets, rejects ambiguous
artifact sets, disables incremental compilation, and binds manifests to the
source and lockfile inputs.

The provenance follow-up captures the Git commit, exact clean worktree state,
and lockfile checksum before evidence compilation and verifies all three after
compilation. Evidence generation fails closed when Git inspection is
unavailable or the tree is dirty. The explicit dirty-tree override is only for
development checks and cannot pass through the stable release gate.

Run the strict pre-seal evidence gate with:

```sh
scripts/stable_release_gate.sh candidate
```

`candidate` and `release` reject pre-release Cargo versions and require the
complete exact-source evidence set. `release` additionally validates the
report-only final commit. Use `scripts/stable_release_gate.sh check` for
development snapshots.

The published crate package includes the core local gate scripts, Rust
toolchain pin, and cargo-deny policy referenced by this document, so downstream
reviewers can inspect the release checks alongside the source and documentation.

Install the optional targets and Cargo tools that make the release gate
exercise the deepest local paths:

```sh
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
cargo install --locked cargo-nextest --version 0.9.140
cargo install --locked cargo-fuzz --version 0.13.2
cargo install --locked kani-verifier --version 0.67.0
```

`cargo-fuzz` and Miri use nightly components:

```sh
rustup toolchain install nightly --component miri
rustup component add rust-src --toolchain nightly
cargo +nightly miri setup
```

## Required Checks

The release gate runs:

- formatting checks
- release metadata validation
- documentation version consistency checks for README, changelog, and SIMD
  release-status docs
- MSRV/toolchain policy validation for `Cargo.toml`, `rust-toolchain.toml`,
  docs.rs metadata, CI install paths, target matrices, and release-evidence
  tooling
- Rust 1.90.0 host and target capability checks through
  `scripts/check-2.0-msrv.sh`, with newer-only optimizations required to retain
  a documented MSRV implementation or scalar fallback
- offline RFC 4648, RFC 2045, and RFC 7468 byte, checksum, source, errata, requirements,
  Git normalization, and package-exclusion validation plus fail-closed
  mutation tests
- independent-oracle differential tests and the versioned semantic corpus
  across core, streaming, bytes, Tokio, serde, and sanitization surfaces
- fragmented `Buf`/`BufMut` integration through `scripts/check-2.0-bytes.sh`,
  including one-byte partitions, exact prefix progress, cumulative limits,
  mutable-`remaining` limit-bypass rejection, transactional owned output,
  panic latching, Miri coverage, and an allocation counter that rejects
  full-input coalescing
- Tokio read-all and `AsyncRead` integration through
  `scripts/check-2.0-tokio-readers.sh`, including one-byte destinations,
  arbitrary deterministic chunk/`Pending` schedules, cancellation/resume,
  exact-frame adjacent-byte preservation, irrevocable plaintext-prefix
  behavior, wrapped-reader unwind cleanup, release tests, Miri coverage, and
  shared-state routing checks
- Tokio `AsyncWrite` integration through `scripts/check-2.0-tokio-writers.sh`,
  including bounded acceptance, short writes, arbitrary deterministic
  backpressure, dropped pending futures, retryable write/flush/shutdown
  failures, checked recovery, terminal cleanup, release tests, Miri coverage,
  wrapped-writer unwind latching, and shared-state routing checks
- Serde integration through `scripts/check-2.0-serde.sh`, including validated
  2.0 codec routing, fixed-capacity ordinary output, fixed-work wiping secret
  output, opaque secret errors, borrowed string/byte input, explicit
  human-readable versus binary text representation, wrapped-body streaming,
  full-capacity owned-input and serializer-unwind cleanup, and
  no-default-feature, clippy, and documentation checks
- Sanitization protected fill through `scripts/check-2.0-sanitization.sh`,
  including no-default and no-std memory-lock builds, separately protected
  staging and destination mappings, fixed-work `SecretFrame` routing, exact
  fixed-size and bounded dynamic output, redacted failures, clippy, and docs
- reviewed equality integration through `scripts/check-2.0-subtle.sh`,
  including every final 2.0 secret owner/view, equal and unequal contents,
  public-length mismatch, compile-fail misuse cases, optimized assembly, and
  the isolated dudect equality class
- hardened derive integration through `scripts/check-2.0-derive.sh`, including
  every sealed alphabet/padding/exposure combination, exact-length and
  redaction behavior, stable malformed-declaration diagnostics, implicit-trait
  compile failures, and generated-source routing inspection
- RFC 2045 Base64 content-transfer body integration through
  `scripts/check-2.0-mime-body.sh`, including canonical and bounded-compatible
  policies, finite input/output/line/skip/work limits, transactional one-shot
  output, one-byte chunk schedules, near-`usize::MAX` absorbing arithmetic
  failures, Python `email.base64mime` and OpenSSL interoperability, package
  scope, and immutable Section 6.8 evidence
- RFC 7468 textual encoding integration through `scripts/check-2.0-pem.sh`,
  including strict and bounded-compatible document grammar, exact labels and
  boundaries, multiple blocks, adjacent text, secret-frame release, the
  official certificate example, Python `ssl` and OpenSSL interoperability,
  full-document fuzzing, and immutable RFC/errata evidence
- Base64-family multibase integration through `scripts/check-2.0-multibase.sh`,
  including a hash-locked upstream registry and official vectors, all four
  admitted prefixes, strict canonical rejection, finite limits, transactional
  one-shot output, heapless one-byte partition schedules, Python standard
  library differential checks, source-mutation checks, and package scope
- 2.0 validated-alphabet evidence through
  `scripts/check-2.0-alphabet.sh`, including const compile acceptance and
  rejection, exhaustive invalid-table diagnostics, all-value mapping parity,
  executable-callback exclusion, fixed-scan semantic checks, and the same
  checks under Rust 1.90.0
- public API audit validation; stable releases fail if public API rows remain
  marked as `review pending`
- packaged release script presence, executable-bit, and shebang validation
- zero-dependency policy check for the published crate
- packaged dependency admission policy for future external-crate review
- dependency admission review keeping the core crate dependency-free while
  treating `base64-ng-serde`, `base64-ng-bytes`, `base64-ng-subtle`,
  `base64-ng-tokio`, `base64-ng-mime`, `base64-ng-multibase`, and
  `base64-ng-pem` as separately reviewed optional
  companion crates; `zeroize`,
  property-testing, and benchmark frameworks remain out of the core package
  unless separately admitted
- reserved feature placeholder checks for `tokio`, `kani`, and `fuzzing`,
  including inert-feature and per-feature dependency graph validation
- fail-closed wasm wipe policy check proving default `wasm32` builds reject
  compiler-fence-only cleanup and the explicit
  `allow-wasm32-best-effort-wipe` opt-in build succeeds
- wasm SIMD posture validation proving the narrow wasm `simd128` runtime
  dispatch profile, its feature gates, its runtime smoke evidence, and its
  remaining JIT/zeroization caveats
- wasm SIMD codegen evidence through
  `scripts/generate_wasm_simd_evidence.sh`, which emits release test-harness
  LLVM IR with `target-feature=+simd128` when the wasm target is installed and
  checks for vector shuffle, 128-bit byte-vector, and wasm bitselect markers
  while leaving runtime/JIT behavior to the separate runtime smoke gate
- wasm simd128 runtime smoke evidence through
  `scripts/check_wasm_runtime_dispatch.sh`, which builds a wasm32 smoke module
  with `target-feature=+simd128` and executes it under Node/V8 and Wasmtime
  when installed, requiring `wasm-simd128` active encode/decode reporting and
  Standard plus URL-safe deterministic length sweeps, independent scalar
  reference encode checks, malformed-input rejection, and round trips
- strict decode public-surface evidence through
  `standard_family_decode_surfaces_cover_tails_and_padding` and
  `standard_family_decode_error_surfaces_match_scalar`, proving Standard and
  URL-safe padded/unpadded slice, clear-tail, stack-buffer, vec, and secret
  helpers match the scalar reference for accepted input and rejected input
  while clear-tail APIs wipe caller buffers on errors
- wasm simd128 browser smoke evidence through
  `scripts/check_wasm_browser_dispatch.sh`, which executes the same wasm32
  smoke module in a Chromium-family browser when installed or when
  `BASE64_NG_BROWSER` points to a compatible browser binary; the gate first
  proves its exact success attribute is absent from static HTML and then
  requires that runtime-created attribute in the browser's dumped DOM
- wasm simd128 Firefox/SpiderMonkey smoke evidence through
  `scripts/check_wasm_browser_firefox_dispatch.sh`, which executes the same
  wasm32 smoke module through `geckodriver` when Firefox is installed
- Safari/WebKit WebDriver smoke evidence through
  `scripts/check_wasm_browser_safari_dispatch.sh`; the `1.3.3` release
  evidence includes a macOS pass with
  `/System/Cryptexes/App/usr/bin/safaridriver`
- 2.0 Commit 30 exact npm-package evidence through
  `scripts/check-2.0-wasm-loader.sh`: deterministic scalar and direct
  `simd128` artifact checksums, artifact ABI clippy and unsafe-count gates,
  Node/V8 differential and hostile-object tests, Wasmtime self-tests, measured
  Node encode/decode benefit, npm tarball file inspection, and
  install-from-package smoke
- exact extracted-package browser evidence through
  `scripts/check_wasm_loader_browser_dispatch.sh` and
  `scripts/check_wasm_loader_browser_firefox_dispatch.sh`, which serve the
  packed npm artifact over HTTP and run Standard/URL-safe padded/unpadded
  scalar/SIMD sweeps in Chromium/V8 and Firefox/SpiderMonkey under a restrictive
  CSP; Safari/WebKit uses
  `scripts/check_wasm_loader_browser_safari_dispatch.sh` on an operator macOS
  host
- fail-closed unsupported-native wipe policy documented through
  `allow-compiler-fence-only-wipe` for architectures without an implemented
  hardware wipe barrier
- clippy with warnings denied
- default, all-features, and no-default-features tests
- CI platform tests on Linux, Windows, pinned macOS ARM images
  (`macos-15`, `macos-26`), pinned Intel macOS (`macos-15-intel`), and
  `macos-latest` as a moving-label migration signal
- local macOS host verification through `scripts/check_macos.sh`, which runs
  the full host test/clippy set and compile-checks both Apple Darwin triples
- local AArch64 Linux host verification through
  `scripts/check_aarch64_linux.sh`, which runs the full host test/clippy set,
  NEON encode block evidence, backend evidence, SIMD feature-bundle checks, and
  SIMD admission validators on real ARM Linux hardware
- big-endian QEMU user-mode verification through
  `scripts/check_big_endian_qemu.sh --all`, which requires both
  `s390x-unknown-linux-gnu` and `powerpc64-unknown-linux-gnu`. Each target runs
  complete default, all-feature, and no-default-feature tests and doctests,
  including RFC 4648, malformed input, incremental, stream, in-place,
  wrapped/legacy, secret-cleanup, and backend-reporting surfaces. Guest libtest
  execution is serialized to avoid host-QEMU thread-scheduler instability
  without skipping any tests; an emulator crash remains a hard failure. This is
  functional correctness and scalar/fallback evidence under emulation only;
  it is not real-hardware performance, timing, microarchitectural, physical
  cleanup, or side-channel evidence. Native community submissions use the
  checked `hardware-evidence/big-endian/schema-v1.json` contract before any
  backend can be upgraded from QEMU-tested to hardware-attested.
- RISC-V QEMU user-mode verification through `scripts/check_riscv_qemu.sh`,
  which runs complete `riscv64gc-unknown-linux-gnu` default, all-feature,
  no-default-feature, and doctest suites. Commit 32 additionally compiles the
  isolated RVV 1.0 candidate and runs differential encode/decode, malformed
  transactionality, capability-probe, and runtime-report tests at VLEN 128 and
  256. `scripts/generate_rvv_asm_evidence.sh` checks exact candidate symbols,
  vector loads/stores, mapping instructions, cleanup, absence of nested calls,
  and ELF vector attributes. This remains emulation and codegen evidence, not
  native performance, timing, ABI/signal preservation, microarchitectural,
  register-retention, or side-channel evidence. Production RISC-V dispatch
  remains scalar until a report accepted by
  `hardware-evidence/riscv/schema-v1.json` and external review pass.
- AArch64 SVE QEMU user-mode verification through `scripts/check_sve_qemu.sh`,
  which runs the complete portable fallback suites and the isolated Commit 33
  candidate at vector lengths 128, 256, and 512. The candidate covers
  Standard/URL-safe ordinary encode and strict decode, malformed no-write
  behavior, capability-probe failures, per-thread vector-length changes, and
  static `no_std` compilation. Portable fallback suites use
  `-cpu max,sve=off`, and all QEMU libtest and doctest harnesses use
  `--test-threads=1`; candidate runs separately enable SVE. This serialization
  is a QEMU 8.2 compatibility constraint, not thread-safety or native-hardware
  evidence. Concurrency-specific tests continue to create worker threads
  internally. `scripts/generate_sve_asm_evidence.sh` checks
  exact leaf symbols, structured loads/stores, predicate mapping, register
  cleanup, absence of nested calls, and absence of stack use using target-aware
  AArch64 binutils rather than the host disassembler. This remains
  emulation and codegen evidence, not native performance, timing, ABI/signal
  preservation, microarchitectural, register-retention, or side-channel
  evidence. Public AArch64 dispatch remains admitted NEON or scalar until two
  reports accepted by `hardware-evidence/sve/schema-v1.json` at different
  vector lengths and external review pass.
- moved-code review for the `src/alphabet.rs` extraction, preserving root
  public exports for built-in alphabets, custom alphabet validation, and the
  `define_alphabet!` macro
- moved-code review for the `src/profiles.rs` extraction, preserving root
  public exports for `Profile` and the named MIME/PEM/bcrypt/crypt profiles
- moved-code review for the `src/cleanup.rs` extraction, preserving internal
  cleanup call paths and updating the unsafe-boundary gate for the new audited
  unsafe location
- moved-code review for the `src/buffers/` extraction, preserving root
  public exports for stack-backed buffers, exposed ownership wrappers, and
  `SecretBuffer`
- all-features and no-default-features doctests plus documentation builds
- `cargo deny check`
- `cargo audit`
- daily and manually dispatchable RustSec plus cargo-deny advisory monitoring
  for the workspace and isolated fuzz, dudect, and performance lockfiles
- `cargo license --json`
- async admission documentation packaged while the `tokio` feature remains
  inert and dependency-free
- Miri through `scripts/check_miri.sh` when nightly Miri is installed,
  covering no-default-features scalar APIs and all-features alloc/stream APIs
  and writing a release evidence manifest
- fuzz target compile check when `cargo-fuzz` is installed
- fuzz corpus policy validation for target-specific reviewed corpus inputs and
  release-blocking artifact cleanup
- isolated dudect, fuzz, and performance harness dependency checks as part of
  the standard gate
- installed-target `no_std` checks for the reserved `simd` feature
- no-alloc portability smoke crate checks for stack-backed encode/decode,
  wrapped output, URL-safe no-padding, and constant-time-oriented decode with
  default features disabled, plus validate-only, legacy decode, in-place
  encode/decode, scalar and constant-time clear-tail cleanup,
  constant-time-oriented in-place decode, named MIME/PEM/bcrypt/crypt profiles,
  custom alphabet/profile, recoverable length, stack-buffer state surfaces,
  and native byte-array and `FromStr` interop surfaces; the harness also runs
  host-side unit tests before cross-target compile checks
- Standard-family encode surface tests covering `encode_slice`,
  `encode_slice_clear_tail`, stack buffers, and alloc helpers for every input
  length from 0 through 193 bytes, including fixed-block thresholds, all tail
  lengths, and padded or unpadded output
- Local and CI target-matrix no-alloc portability smoke checks so installed
  Linux, FreeBSD, wasm32, ARM, and Cortex-M targets compile the same
  stack-backed dependency-free harness
- migration-guide smoke tests for strict standard, URL-safe no-pad, MIME/PEM,
  legacy whitespace, custom alphabet, stack-buffer, secret-buffer, and stream
  migration examples
- reserved SIMD feature-bundle compile checks for AVX2, AVX-512 VBMI,
  SSSE3/SSE4.1, NEON, and wasm `simd128` under `no_std` when the corresponding
  Rust targets are installed
- backend evidence capture for runtime backend reporting, admitted AVX-512
  VBMI, AVX2, SSSE3/SSE4.1, or NEON encode dispatch when supported, and
  admitted strict decode dispatch when supported
- Standard-family strict decode surface tests covering `decode_slice`,
  `decode_slice_clear_tail`, stack buffers, and alloc helpers for every input
  length from 0 through 193 bytes after scalar-reference encode, including
  fixed-block thresholds, short inputs, non-block tails, and padded or
  unpadded input
- SIMD admission policy for the current release series, with AVX-512 VBMI,
  AVX2, SSSE3/SSE4.1, and NEON encode/strict decode admitted for runtime-probed
  `std` or health-gated static `no_std` execution on applicable x86/x86_64 or
  little-endian AArch64 Standard and URL-safe alphabet families, and no SIMD
  performance claims without complete local benchmark evidence
- unsafe-boundary validation that confines `allow(unsafe_code)` to the audited
  cleanup helpers in `src/cleanup.rs`, CT barrier/comparison helpers in
  `src/ct/`, and the SIMD boundary in `src/simd/`
- unsafe-boundary validation that confines inline assembly to the cleanup and CT
  barriers and confines CPU feature detection and `target_feature` gates to
  `src/simd/`
- unsafe-boundary validation that requires inventory documentation for every
  SIMD-boundary unsafe function and a nearby `SAFETY:` explanation for every
  unsafe block
- panic-policy validation that fails on unreviewed non-test `panic!`,
  `unreachable!`, `.unwrap()`, or `.expect()` sites
- constant-time policy validation that keeps non-claim wording and
  generated-code review requirements in the documented release bar
- dudect-style timing harness compile and dependency checks, with timing runs
  opt-in for local release evidence
- constant-time assembly evidence generation for no-default-features and
  all-features release builds
- runtime backend report tests proving the public active backend remains scalar
  until an accelerated backend is explicitly admitted
- runtime backend policy tests for scalar execution and no-SIMD deployment
  assertions
- high-assurance scalar-only backend policy tests
- stable runtime enum string identifier tests for audit-friendly evidence
- stable key/value runtime report and policy-failure formatting tests
- constant-time-oriented clear-tail decode tests for success, malformed input,
  undersized output, and in-place cleanup
- constant-time-oriented validate/decode agreement tests for valid and
  malformed inputs across supported alphabets and padding modes
- stream encoder and decoder tests proving policy accessors, state accessors,
  `finish()`, `try_finish()`, `into_inner()`, and adjacent-payload behavior
  remain intact after cleanup hardening
- stream encoder and decoder retry tests proving pending input survives wrapped
  writer failures, and finalization flush retries do not re-emit terminal
  encoded or decoded bytes
- stream encoder and decoder short-write tests proving buffered writer output
  is retained until the wrapped writer reports bytes accepted
- stream reader output queues drain into caller buffers in bounded slices while
  consumed queue slots are cleared
- stream decoder fail-closed tests proving malformed Base64 input poisons the
  adapter while preserving explicit unchecked inner recovery
- stream fuzz coverage for chunked writers, fragmented reader sources, and
  adjacent framed payload boundaries, including fail-closed decoder state
  invariants after malformed input
- profile and custom-alphabet fuzz coverage for MIME, PEM, bcrypt-style,
  `crypt(3)`-style, and caller-defined alphabets
- opt-in bounded fuzz smoke evidence through
  `BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh`
- opt-in release-duration fuzz evidence through
  `BASE64_NG_RUN_FUZZ_RELEASE=1 scripts/check_fuzz.sh`, including final
  statistics, corpus and output hashes, artifact counts, and minimization
  status for all 18 targets
- generated constant-time assembly artifacts through
  `scripts/generate_ct_asm_evidence.sh`
- manual generated-code review checklist in [CT_ASM_REVIEW.md](CT_ASM_REVIEW.md)
- LTO symbol-presence checks for non-inlined wipe boundaries,
  `constant_time_eq_public_len`, `secret_encode_ascii`, and
  `secret_encode_scan`; the parser
  accepts both legacy Rust symbols and the v0 symbols enabled by default in
  Rust `1.97.1`
- Kani proofs through `scripts/check_kani.sh`; the machine-checked inventory is
  43 normal, 19 advanced, and 6 exploratory harnesses on the Rust `1.90.0`
  Kani toolchain with `cargo-kani 0.67.0`
- complete required advanced release-host execution through
  `BASE64_NG_KANI_ALL_ADVANCED=1 scripts/check_kani_advanced.sh`
- retained verifier/compiler identities, exact commands, harness lists,
  results, resource reports, unsupported-construct output, status, and
  checksums under `target/release-evidence/kani/`
- bounded Kani coverage for constant-time-oriented decode result bounds,
  clear-tail cleanup on error, and validate/decode agreement
- opt-in bounded Kani coverage for Commit 20 final-quantum bounds, absorbing
  failure, and overlap/address-range preflight
- independent fixed-array RFC 4648 refinement for every strict 2.0 preset,
  incremental and in-place behavior, and canonical trailing-bit rejection
- bounded portable SIMD arithmetic/mask/wrapper models and an explicit
  initialized-before-visible commit model; architecture intrinsics and inline
  assembly remain covered by differential, generated-code, and platform gates
- an explicitly volatile in-memory four-axis teardown and journal model; it is
  not persistence, crash-recovery, allocator, OS-protection, or unsafe-provider
  proof
- bounded-index invariant documentation in [INVARIANTS.md](INVARIANTS.md)
- explicit Kani compatibility or verifier-exception documentation in
  [KANI.md](KANI.md) if a future installed Kani compiler cannot run the proofs
- the historical initial `1.0.0` Kani verifier exception is superseded for the
  current bounded harness set by the clean `scripts/check_kani.sh` run above;
  future verifier incompatibility must be documented explicitly rather than
  treated as proof
- SBOM generation
- reproducible package/build check

## Generated Artifacts

Evidence is written under:

```text
target/release-evidence/
```

Expected files:

- `base64-ng.spdx.json`
- `base64-ng.cyclonedx.json`
- `sbom-MANIFEST.txt`
- `backend/MANIFEST.txt`
- `backend/runtime-backend-report.txt`
- `backend/simd-prototype-equivalence.txt`
- `asm/MANIFEST.txt`
- `asm/base64_ng-no-default-features.s`
- `asm/base64_ng-all-features.s`

The SBOMs and `sbom-MANIFEST.txt` describe the published crate dependency
graph and record tool versions, commands, and checksums. The normal published
crate is zero-dependency; fuzz-only dependencies live under `fuzz/` and are
reviewed separately.

## Fuzz-Only Dependency Evidence

The fuzz harness is intentionally isolated from the published crate. Review it
with:

```sh
scripts/check_fuzz.sh
```

`fuzz/deny.toml` allows the NCSA license only for `libfuzzer-sys`. The root
`deny.toml` remains stricter for the published crate.

`scripts/check_fuzz.sh` explicitly runs:

```sh
cargo audit --file fuzz/Cargo.lock
scripts/cargo-deny-check.sh fuzz/Cargo.toml fuzz/deny.toml
```

The `differential` fuzz target includes static RFC 4648 ground-truth vectors in
addition to comparison against the established `base64` crate oracle.

The `stream_chunks` fuzz target covers:

- chunked streaming encoders and decoders
- fragmented `EncoderReader` sources compared with slice encoding
- fragmented `DecoderReader` sources compared with slice decoding when payload
  boundary semantics match
- padded `DecoderReader` payloads followed by adjacent framed bytes, proving
  the reader leaves those bytes unread
- stream state-helper invariants for pending quanta, buffered output capacity,
  recovery readiness, and terminal input state

The final 2.0 targets add runtime-alphabet and policy differential checks,
arbitrary incremental partitions, legacy and WHATWG policies, caller
formatter panic propagation, manually polled Tokio cancellation/backpressure,
finite provider-budget schedules, and every volatile teardown fault stage.
Static `no_std` backend-token behavior is executed by the architecture smoke
crates because libFuzzer itself requires `std`. Wasm `simd128` remains bound to
its dedicated runtime/browser differential evidence rather than a native fuzz
process. Base 2.0 has no persistent provider or persistent parser, so no
persistence/restart claim is made by the volatile assurance target.

Run a bounded local smoke test with:

```sh
cargo +nightly fuzz run stream_chunks -- -runs=1000
```

LibFuzzer may generate local corpus files under `fuzz/corpus/`; review them
before committing and discard accidental local corpus churn.

## SIMD Feature-Bundle Evidence

Reserved SIMD code must compile under the feature bundles that future admitted
backends will rely on. Check installed SIMD feature bundles with:

```sh
scripts/check_simd_feature_bundles.sh
```

This currently proves `no_std` reserved builds for AVX2, SSSE3/SSE4.1, the
AVX-512 Base64 candidate bundle (`avx512f`, `avx512bw`, `avx512vl`, and
`avx512vbmi`), NEON, and wasm `simd128` when the corresponding Rust targets
are installed. For wasm `simd128`, the script also builds the wasm test
binaries with `target-feature=+simd128` so the admitted fixed-block wasm code
remains typechecked and codegen-ready.

Capture local runtime backend and prototype evidence with:

```sh
scripts/check_backend_evidence.sh
```

The script runs the runtime backend-report test and the gated SIMD
scalar-equivalence tests with `--nocapture`. The runtime report records
`candidate_detection_mode`, which distinguishes x86/x86_64 `std` runtime CPU
probing from compile-time target-feature reporting used by `no_std` and other
compile-time-only targets. On CPUs with AVX-512 VBMI, AVX2, SSSE3/SSE4.1, or
little-endian AArch64 NEON, or wasm `simd128`, an admitted encode path may be
active for Standard and URL-safe alphabets. Big-endian AArch64 stays scalar,
and 32-bit ARM NEON remains scaffold evidence. Wasm `simd128` evidence is kept
in `scripts/check_simd_feature_bundles.sh` as compile/test-binary evidence and
in `scripts/check_wasm_runtime_dispatch.sh` as Node/V8 and Wasmtime runtime
smoke evidence, in `scripts/check_wasm_browser_dispatch.sh` as
Chromium-family browser smoke evidence, in
`scripts/check_wasm_browser_firefox_dispatch.sh` as Firefox/SpiderMonkey smoke
evidence, and in `scripts/check_wasm_browser_safari_dispatch.sh` as
Safari/WebKit smoke evidence. Runtime/JIT timing behavior remains outside the
crate's formal claim. The script writes
`target/release-evidence/backend/MANIFEST.txt`, `runtime-backend-report.txt`,
and `simd-prototype-equivalence.txt` so local CPU evidence can be archived. The
manifest labels prototype-only evidence as `real-non-dispatchable` and
separately records
`active_backend_admitted=avx512-vbmi-or-avx2-or-ssse3-sse4.1-or-neon-or-wasm-simd128-encode`,
so audit logs do not confuse remaining fixed-block prototype execution with
active dispatch admission.

The 2.0 JavaScript-facing evidence is separate from this older Rust-runtime
smoke. `scripts/check-2.0-wasm-loader.sh` builds, tests, benchmarks, packs, and
installs the supported `base64-ng-wasm-loader` package. Its browser scripts use
the extracted npm tarball, not a source-tree-only wasm fixture. The package
reports probe evidence and selected scalar/SIMD artifact independently and
does not expose secret APIs or wasm linear-memory views. The package verifies
embedded SHA-256 digests before instantiation, requires explicit digests for
custom artifact sources, tests proportional per-operation cleanup separately
from complete teardown cleanup, traps rather than spins on Rust panic, rebuilds
from two absolute checkout paths, and rejects host checkout paths in artifacts.

The release gate also runs:

```sh
scripts/validate-simd-admission.sh
scripts/validate-simd-encode-admission-draft.sh
```

That validator keeps active SIMD dispatch limited to admitted backends until a
release includes the required scalar differential tests, fuzz evidence, unsafe
inventory updates, architecture evidence, benchmark evidence, release-note
wording, and an updated `docs/SIMD_ADMISSION.md` manifest.

For a future encode-dispatch release, use
[`SIMD_ENCODE_ADMISSION_DRAFT.md`](SIMD_ENCODE_ADMISSION_DRAFT.md) as the
working package. It defines the runtime-report expectations, benchmark record,
release-note wording, and decision checklist required before any encode backend
can move from real non-dispatchable prototype evidence to active dispatch.
`scripts/validate-simd-encode-admission-draft.sh` keeps that draft packaged and
checks that the future admission contract still names the required runtime,
fallback, benchmark, release-note, and architecture-specific evidence.

## Miri Evidence

Run Miri coverage with:

```sh
scripts/check_miri.sh
```

When nightly Miri is installed, the script runs no-default-features and
all-features test surfaces and writes
`target/release-evidence/miri/MANIFEST.txt`, `no-default-features.txt`, and
`all-features.txt`. This evidence is useful for release review of the
dependency-free scalar core, alloc helpers, stream wrappers, and cleanup
helpers. The all-features set also enters the safe encode-dispatch wrapper, but
does not claim to execute architecture intrinsics under Miri. It remains
tool-backed undefined-behavior evidence, not a formal proof.

## In-Place AddressSanitizer Evidence

Run the Commit 14 cursor, overlap, and staged-secret suites under nightly
AddressSanitizer with:

```sh
scripts/check-2.0-in-place-sanitizers.sh
```

The script requires nightly `rust-src`, rebuilds the standard library with
AddressSanitizer, and writes a manifest and complete log under
`target/release-evidence/2.0-memory-sanitizers/`. It detects observable
out-of-bounds and lifetime defects in those executions; it does not prove the
logical fixed-work or cleanup contracts by itself.

## Formatting And Append Evidence

Run the Commit 15 allocation, formatter, counted-sink, rollback, chunk, and
lifetime evidence with:

```sh
scripts/check-2.0-format-append-chunks.sh
```

The allocation test uses an isolated integration-test global allocator and
counts heap allocation calls only around `display` and `encode_to_fmt`. Append
tests inject reserve failures, returned errors after mutation, and unwinding
panics. Formatter tests distinguish fully successful calls from possible
partial mutation inside the failing call. These are bounded execution tests,
not whole-program allocation or foreign-sink proofs.

## WHATWG And Expert Compatibility Evidence

Run the Commit 16 core, exact-pinned compatibility, wasm, and locally
available browser evidence with:

```sh
scripts/check-2.0-web-forgiving.sh --browsers
```

The locked fixture corpus is executed through Rust one-shot and every-split
incremental decoding, Rust wasm, and browser-native `atob`. Browser scripts
cover Node/V8, Chromium/Blink, Firefox/SpiderMonkey, and Safari/WebKit when the
corresponding runtime is installed. A skipped runtime is not execution
evidence and must be identified as such in the release record. Expert ordinary
policies are compared with exact-pinned `base64` 0.23.0 and an independent
bit-level oracle. This is interoperability evidence, not a timing or secret
processing claim.

## Constant-Time Timing Evidence

The standard local gate, normal CI gate, and release gate compile the isolated
dudect-style harness and check its dependency policy:

```sh
scripts/check_dudect.sh
```

Timing measurements are opt-in because shared CI runners are not stable enough
for reliable side-channel statistics:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

Equal-work cases cover valid contents, malformed positions/classes, the
fixed-work valid-versus-malformed pre-gate boundary, built-in/custom encoding,
and equality mismatch positions. Public-length decode, encode, and equality
comparisons are informational and are not thresholded. Whole-call
valid-versus-invalid equality is not claimed because successful decode performs
the documented post-gate release copy.

Archive the raw output with CPU, OS, Rust version, sample count, and command
line when using dudect-style evidence for a security review. Opt-in timing runs
write `target/release-evidence/dudect/dudect-output.txt` and
`target/release-evidence/dudect/MANIFEST.txt` for this purpose. This evidence
is empirical and does not replace generated-code review or Kani proofs.

The release gate also generates assembly artifacts for reviewer inspection
with:

```sh
scripts/generate_ct_asm_evidence.sh
```

Target-specific code-generation evidence uses an installed target:

```sh
BASE64_NG_CT_ASM_TARGET=aarch64-unknown-linux-gnu \
  scripts/generate_ct_asm_evidence.sh
```

The script writes `target/release-evidence/asm/base64_ng-no-default-features.s`,
`target/release-evidence/asm/base64_ng-all-features.s`, and
`target/release-evidence/asm/base64_ng-all-features-lto.s`, plus
`target/release-evidence/asm/MANIFEST.txt` with rustc metadata, commands,
review focus, artifact checksums, the source commit and tree state, and the
`Cargo.lock` checksum. Every assembly build uses a new isolated target
directory with incremental compilation disabled, requires exactly one fresh
crate artifact, and uses `--locked`; persistent Cargo caches are never an
evidence source. Git and lockfile provenance are captured before compilation
and rechecked afterward; release generation rejects dirty or unavailable
state. The LTO artifact exists so reviewers can check that cleanup
primitives such as `wipe_bytes` and `wipe_barrier` remain visible call
boundaries under aggressive optimization.
The manifest also records the wipe primitive revision and target barrier.
Operation-specific `WipedAttested` evidence must additionally carry the
runtime wipe generation from its operation report; static assembly cannot
infer it. The retained claim covers only the logical byte range and excludes
registers, caches, allocator history, swap, snapshots, and compiler-created
copies.

Capture generated assembly evidence for x86 SIMD encode paths with:

```sh
scripts/generate_simd_asm_evidence.sh
```

On x86/x86_64 hosts, the script emits release test-harness assembly for the
admitted AVX-512 VBMI, AVX2, and SSSE3/SSE4.1 encode paths, then checks for the
expected byte-shuffle, byte-permute, vector-register, and transition/cleanup
instructions. Commit 25 also checks exact-width SSSE3/AVX2 loads, rejects
per-block wipe overhead, and requires one explicit register cleanup at each
complete block-loop boundary.
When the `aarch64-unknown-linux-gnu` target is installed, it also emits AArch64
NEON release assembly and checks for table lookup, bit-select, and
register-cleanup instructions. Cross-host runs record NEON library assembly
and compile evidence; real AArch64 host runs provide the matching test-harness
execution evidence. On non-x86 hosts it records a skip manifest. The generated
files are written to `target/release-evidence/simd-asm/`. SIMD assembly and
wasm LLVM-IR generation follow the same isolated-target, exact-artifact,
locked-input, and source-binding policy as constant-time assembly evidence.

Commit 29 adds `scripts/generate_neon_asm_evidence.sh`, which works from any
host with the AArch64 Rust target and checks direct decode validity reduction,
exact output stores, table compaction, alphabet selection, and register
cleanup. `scripts/check-2.0-neon-hot-paths.sh` combines that codegen evidence
with real-device exhaustive tests, static `no_std` execution, fuzz contracts,
and optional Apple/server ARM performance campaigns.

## Performance Evidence

The performance harness is intentionally isolated from the published crate.
The standard local gate compiles and reviews its dependencies. Run the same
check directly while iterating on benchmark code with:

```sh
scripts/check_perf.sh
```

Capture the complete two-run benchmark campaign with:

```sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

This writes raw runs, a statistical summary, exact-backend availability and
scalar-ratio admission tables, resource measurements, environment JSON, and a
checksum manifest under `target/release-evidence/perf/`. The harness measures
production auto dispatch, scalar, and each exact backend available on the host.
It compares exact-pinned `base64 0.23.0` and `base64ct 1.8.3` only where
canonical caller-owned slice semantics match.

Commit 17 adds `scripts/check-2.0-profiles.sh`. It differentially checks
PBKDF2 alphabet behavior against exact-pinned `base64ct 1.8.3`, BinHex and
IMAP alphabets against exact-pinned `base64 0.23.0`, and renamed 1.x body and
alphabet behavior against its explicit 2.0 equivalent. Exhaustive ASCII,
chunking, original-index, overflow, transactional-output, and secret-exclusion
checks cover the retained legacy-whitespace policy.

`docs/2.0_DEVICE_VERIFICATION_QUEUE.md` records the macOS/AArch64, Linux
AArch64, x86-64, and browser commands planned for the consolidated Commit 20
checkpoint. Pending rows are not evidence and must never be reported as pass.

Campaign generation fails closed unless the worktree is clean and `HEAD`
resolves to one full commit identifier that remains unchanged through
measurement and manifest creation. Validation reconstructs the complete
expected matrix from backend availability, requires every fixed length and
exact sample index, rejects surplus CSV cells, and restricts every textual
evidence field to its schema or
`[A-Za-z0-9][A-Za-z0-9._-]{0,63}`. Comparison and derivation commands require
the complete bundle, while retained derived rows must exactly match the raw
runs. The original dirty-tree Commit 3 campaign is invalidated and cannot
support a release or backend-admission claim. The replacement is retained under
`performance-baselines/commit-5-correction-amd-9950x3d-linux/` and records
clean source commit `9665094362c535550e3a7cb5d812bf3bccccb0b7`.

The retained schema and community submission contract are documented in
[`BENCHMARKS.md`](BENCHMARKS.md) and
[`../performance-baselines/README.md`](../performance-baselines/README.md).
Performance numbers are release-note evidence only when paired with the source
commit, raw samples, hardware, microcode, OS, Rust version, flags, CPU governor,
and manifest.
Performance numbers are release notes evidence only; they do not independently
admit or preserve a runtime backend.

## Commit 46 IMAP Payload Evidence

`base64-ng-imap` is release-gated by `scripts/check-2.0-imap.sh`. The gate
validates the locked RFC 3501 bytes, reviewed erratum 261, and requirement
ledger; builds the companion without default features and with allocation;
runs conformance, limit, rollback, fragmented-state, Python differential, and
optional `iconv` interoperability tests; compiles its fuzz target; checks
Clippy, rustdoc, MSRV when locally installed, dependencies, and exact package
contents; and proves the locked RFC material is excluded from publication.

The evidence supports only the modified-Base64 payload transform over
already-converted UTF-16BE bytes. It does not support a full modified UTF-7,
mailbox-parser, Unicode-conversion, current IMAP4rev2, secret-processing, or
constant-time claim.

## Commit 47 Password-Record Evidence

`base64-ng-password` is release-gated by `scripts/check-2.0-password.sh`. The
gate builds without default features, runs all five exact record formats,
checks Passlib documentation vectors and independent adapted-Base64 and
SHA-crypt permutation answers, exercises optional OpenSSL interoperability,
tests malformed delimiters, rounds, salt alphabets, checksum lengths, unused
bits, limits, transactional output, and redacted formatting, compiles the fuzz
target, and checks Clippy, rustdoc, MSRV, dependencies, and package contents.

This evidence covers field transformation plus record parsing and generation
only. It supports no password input, PBKDF2/SHA derivation, password hashing,
password verification, storage-policy, constant-time, or cleanup claim. Commit
49 owns the exact release source and implementation pins.

## Commit 48 OpenPGP Armor Evidence

`base64-ng-openpgp` is release-gated by `scripts/check-2.0-openpgp.sh`. The
gate validates the locked July 2024 RFC 9580 bytes, all seven current RFC
Editor errata records, and the armor requirement ledger; builds without
default features; runs all ordinary labels, checksum states, official vectors,
finite limits, malformed framing, incremental partitions, short and
over-reporting I/O, redaction, and fixed-work secret release; requires GnuPG
and Sequoia differential dearmor evidence in CI; compiles the whole-message
fuzz target; and checks Clippy, rustdoc, MSRV, dependencies, and exact package
contents.

The evidence supports complete ordinary ASCII armor only. It does not support
OpenPGP packet semantics, cleartext signatures, cryptographic integrity from
CRC-24, or implicit permission to generate CRC-24 in RFC 9580 contexts that
forbid it.

## Reproducibility

The reproducible package/build check packages and verifies the crate twice and
compares the generated package file list. This catches accidental metadata,
include-list, or generated-file drift before release.

## Publishing

Before tagging:

```sh
scripts/stable_release_gate.sh release
```

The stable release gate is the expensive pre-tag gate and includes Kani,
generated assembly evidence, SBOM generation, and reproducibility checks. Run
it before creating the immutable GitHub tag.

After the gate passes, push the release commit, wait for GitHub CI, then create
and push the `v<version>` tag. Publish only from that tagged commit:

```sh
scripts/release_crates.py --check
scripts/release_crates.py --dry-run
```

Publish with:

```sh
scripts/release_crates.py
```

The helper reads `release-crates.toml`, refuses real publishing unless `HEAD`
matches a verified signed release tag, runs the standard local gate and
`cargo publish --dry-run` for selected crates, publishes `base64-ng` first,
waits for crates.io visibility, and then publishes dependent companion crates.
The default publish preflight does not rerun Kani because Kani is already part
of the pre-tag stable gate. Use `scripts/release_crates.py --full-gate` only
when the release manager deliberately wants to rerun the expensive gate
immediately before upload.

Manual fallback for companion releases:

```sh
cargo publish -p base64-ng
cargo package -p base64-ng-sanitization
cargo publish -p base64-ng-sanitization --dry-run
cargo publish -p base64-ng-sanitization
cargo package -p base64-ng-derive
cargo publish -p base64-ng-derive --dry-run
cargo publish -p base64-ng-derive
cargo package -p base64-ng-serde
cargo publish -p base64-ng-serde --dry-run
cargo publish -p base64-ng-serde
cargo package -p base64-ng-bytes
cargo publish -p base64-ng-bytes --dry-run
cargo publish -p base64-ng-bytes
cargo package -p base64-ng-subtle
cargo publish -p base64-ng-subtle --dry-run
cargo publish -p base64-ng-subtle
cargo package -p base64-ng-tokio
cargo publish -p base64-ng-tokio --dry-run
cargo publish -p base64-ng-tokio
```

After `cargo publish`, verify crates.io metadata with:

```sh
cargo info base64-ng
```

Do not move an existing release tag. If the tagged source is wrong, cut a new
patch release.
