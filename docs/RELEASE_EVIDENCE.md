# Release Evidence

`base64-ng` treats release evidence as part of the artifact, not as an informal
local habit. The release gate generates and verifies evidence that downstream
users can inspect before adopting a version.

Run the gate with:

```sh
scripts/stable_release_gate.sh release
```

`release` mode rejects pre-release Cargo versions. Use
`scripts/stable_release_gate.sh check` for development snapshots.

The published crate package includes the core local gate scripts, Rust
toolchain pin, and cargo-deny policy referenced by this document, so downstream
reviewers can inspect the release checks alongside the source and documentation.

Install the optional targets and Cargo tools that make the release gate
exercise the deepest local paths:

```sh
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf
cargo install --locked cargo-nextest
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier
```

`cargo-fuzz` and Miri use nightly components:

```sh
rustup toolchain install nightly --component miri
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
- public API audit validation; stable releases fail if public API rows remain
  marked as `review pending`
- packaged release script presence, executable-bit, and shebang validation
- zero-dependency policy check for the published crate
- packaged dependency admission policy for future external-crate review
- final `v1.0` dependency admission review keeping `tokio`, `serde`, `bytes`,
  `zeroize`, `subtle`, property-testing, and benchmark frameworks out of the
  published crate contract unless separately admitted
- reserved feature placeholder checks for `tokio`, `kani`, and `fuzzing`,
  including inert-feature and per-feature dependency graph validation
- fail-closed wasm wipe policy check proving default `wasm32` builds reject
  compiler-fence-only cleanup and the explicit
  `allow-wasm32-best-effort-wipe` opt-in build succeeds
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
- Local and CI target-matrix no-alloc portability smoke checks so installed
  Linux, FreeBSD, wasm32, ARM, and Cortex-M targets compile the same
  stack-backed dependency-free harness
- migration-guide smoke tests for strict standard, URL-safe no-pad, MIME/PEM,
  legacy whitespace, custom alphabet, stack-buffer, secret-buffer, and stream
  migration examples
- reserved SIMD feature-bundle compile checks for AVX2, AVX-512 VBMI,
  SSSE3/SSE4.1, NEON, and wasm `simd128` under `no_std` when the corresponding
  Rust targets are installed
- backend evidence capture for runtime backend reporting and inactive SIMD
  prototype scalar-equivalence output
- scalar-only SIMD admission policy for the current release series, with no
  active accelerated dispatch and no SIMD performance claims unless a complete
  backend admission evidence package lands
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
- generated constant-time assembly artifacts through
  `scripts/generate_ct_asm_evidence.sh`
- manual generated-code review checklist in [CT_ASM_REVIEW.md](CT_ASM_REVIEW.md)
- LTO symbol-presence checks for non-inlined wipe boundaries and the
  `constant_time_eq_public_len` equal-length comparison helper
- Kani proofs through `scripts/check_kani.sh`; current local evidence is the
  full no-default-features harness set on Rust `1.90.0` with
  `cargo-kani 0.67.0`
- bounded Kani coverage for constant-time-oriented decode result bounds,
  clear-tail cleanup on error, and validate/decode agreement
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
cargo deny --manifest-path fuzz/Cargo.toml check --config fuzz/deny.toml
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
are installed.

Capture local runtime backend and prototype evidence with:

```sh
scripts/check_backend_evidence.sh
```

The script runs the runtime backend-report test and the gated SIMD prototype
scalar-equivalence scaffolding tests with `--nocapture`. The runtime report
records `candidate_detection_mode`, which distinguishes x86/x86_64 `std`
runtime CPU probing from compile-time target-feature reporting used by
`no_std` and other compile-time-only targets. On CPUs with
SSSE3/SSE4.1, AVX2, or the AVX-512 candidate bundle, those prototype tests
execute the inactive prototype body and compare it against scalar output. The
x86 prototypes exercise real fixed-block vector encode logic when the required
CPU feature bundles are available. On AArch64 NEON-capable hosts, the NEON test
exercises the inactive fixed-block vector prototype for Standard and URL-safe
alphabets; 32-bit ARM remains scaffold evidence. The script writes
`target/release-evidence/backend/MANIFEST.txt`, `runtime-backend-report.txt`,
and `simd-prototype-equivalence.txt` so local CPU evidence can be archived.

The release gate also runs:

```sh
scripts/validate-simd-admission.sh
```

That validator keeps active SIMD dispatch scalar-only until the release includes
the required scalar differential tests, fuzz evidence, unsafe inventory updates,
architecture evidence, benchmark evidence, release-note wording, and an updated
`docs/SIMD_ADMISSION.md` manifest.

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
helpers. It remains tool-backed undefined-behavior evidence, not a formal proof.

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

The script writes `target/release-evidence/asm/base64_ng-no-default-features.s`,
`target/release-evidence/asm/base64_ng-all-features.s`, and
`target/release-evidence/asm/base64_ng-all-features-lto.s`, plus
`target/release-evidence/asm/MANIFEST.txt` with rustc metadata, commands,
review focus, and artifact checksums. The LTO artifact exists so reviewers can
check that cleanup primitives such as `wipe_bytes` and `wipe_barrier` remain
visible call boundaries under aggressive optimization.

Capture generated assembly evidence for the inactive x86 SIMD encode
prototypes with:

```sh
scripts/generate_simd_asm_evidence.sh
```

On x86/x86_64 hosts, the script emits release test-harness assembly for the
SSSE3/SSE4.1, AVX2, and AVX-512 VBMI feature bundles and checks for the
expected byte-shuffle, byte-permute, vector-register, and cleanup instructions.
On non-x86 hosts it records a skip manifest. The generated files are written to
`target/release-evidence/simd-asm/` and are inactive prototype evidence only;
runtime dispatch remains scalar-only until the SIMD admission manifest is
updated in a future release.

## Performance Evidence

The performance harness is intentionally isolated from the published crate.
The standard local gate compiles and reviews its dependencies. Run the same
check directly while iterating on benchmark code with:

```sh
scripts/check_perf.sh
```

Run local scalar comparison measurements with:

```sh
cargo run --release --manifest-path perf/Cargo.toml
```

Capture benchmark output and a manifest with:

```sh
BASE64_NG_RUN_PERF=1 scripts/check_perf.sh
```

This writes `target/release-evidence/perf/perf-output.csv` and
`target/release-evidence/perf/MANIFEST.txt`.

Performance numbers are release notes evidence only when paired with hardware,
OS, Rust version, CPU governor, and the exact command output.

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
matches the release tag, runs the standard local gate and `cargo publish
--dry-run` for selected crates, publishes `base64-ng` first, waits for crates.io
visibility, and then publishes dependent companion crates. The default publish
preflight does not rerun Kani because Kani is already part of the pre-tag stable
gate. Use `scripts/release_crates.py --full-gate` only when the release manager
deliberately wants to rerun the expensive gate immediately before upload.

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
