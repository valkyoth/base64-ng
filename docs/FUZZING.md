# Fuzzing Policy

`base64-ng` keeps fuzzing isolated from the published crate. The root crate
remains dependency-free; fuzz-only dependencies live under `fuzz/` and are
checked by the standard local gate while remaining outside the published crate.

Run fuzz harness checks with:

```sh
scripts/check_fuzz.sh
```

Run corpus policy checks directly with:

```sh
scripts/check_fuzz_corpus.sh
```

## Targets

Current fuzz targets:

- `decode`: arbitrary strict, strict line-wrapped, legacy, and
  constant-time-oriented decode input plus wrapped encode slice/alloc
  equivalence; it also derives malformed cases from canonical encodings to
  exercise invalid-byte positions, mixed alphabets, early padding,
  non-canonical trailing bits, and clear-tail error behavior
- `in_place`: in-place encode/decode, legacy compaction behavior, and strict
  line-wrapped in-place compaction behavior
- `stream_chunks`: fragmented stream reader/writer state machines, adjacent
  framed payload boundaries, and stream state-helper invariants
- `differential`: canonical output comparison against the established Base64
  behavior used by the harness, plus static RFC 4648 ground-truth vectors so
  the differential oracle is not the only source of truth
- `x86_encode`: forced SSSE3/SSE4.1 and AVX2 Standard and URL-safe encode
  comparison against the independent `base64` oracle on hosts that report the
  complete runtime feature bundle; input is capped at 64 KiB per iteration
- `x86_decode`: forced SSSE3/SSE4.1 and AVX2 Standard and URL-safe strict
  decode comparison against the scalar public contract, including exact error
  equality and rejected-output retention; canonical input is also checked
  against the independent `base64` oracle
- `neon`: forced little-endian AArch64 NEON Standard and URL-safe encode and
  strict-decode comparison against the scalar public contract and independent
  canonical oracle; input is capped at 64 KiB and the target is a no-op on
  non-AArch64 fuzz hosts
- `mime_body`, `pem_document`, `multibase_family`, `imap_payload`,
  `password_records`, and `openpgp_armor`: bounded complete-protocol parsing,
  generation, incremental partitioning, canonical regeneration, and malformed
  input checks for every 2.0 protocol companion
- `v2_runtime_codec`: runtime-alphabet construction and rejection, every
  runtime padding/trailing-bit policy combination, one-shot, in-place, append,
  allocation-limit, and transactional-output behavior against an independent
  encoder
- `v2_incremental`: arbitrary encoder/decoder partitions, absorbing malformed
  state, legacy whitespace, WHATWG forgiving decode, short counted sinks, and
  caller formatter panic propagation
- `v2_async`: manually polled Tokio reader and writer adapters under one-byte
  I/O, short writes, alternating `Pending`, shutdown, malformed failure, and
  cancellation-by-drop schedules
- `v2_assurance`: finite default-provider admission, invalidation,
  maintenance, retry, exhaustion, assured encode/decode, and every teardown
  fault stage through a fuzz-only unsafe provider whose assertions enforce
  ordering, exact-once effects, zero-before-teardown, quarantine, tombstoning,
  and claim separation

`tests/v2_fuzz_properties.rs` supplies deterministic exhaustive-small
properties for 64 runtime alphabet rotations, padded and unpadded policies,
and all small input/output partition combinations. The target-specific x86 and
NEON harnesses reach every runtime-supported admitted native backend directly.
Wasm `simd128` cannot be executed by the native libFuzzer process; its admitted
path remains covered by the dedicated Node, Wasmtime, Chromium, Firefox, and
Safari runtime/browser differential gates.

`scripts/check_fuzz.sh` also runs the fuzz workspace supply-chain gates:

```sh
cargo audit --file fuzz/Cargo.lock
scripts/cargo-deny-check.sh fuzz/Cargo.toml fuzz/deny.toml
```

The isolated `fuzz/deny.toml` permits the `libfuzzer-sys` license exception
needed by the harness. The published crate remains governed by the stricter
root `deny.toml`.

## Corpus Admission

Committed corpus inputs are allowed only under:

- `fuzz/corpus/decode/`
- `fuzz/corpus/in_place/`
- `fuzz/corpus/stream_chunks/`
- `fuzz/corpus/differential/`
- `fuzz/corpus/x86_encode/`
- `fuzz/corpus/x86_decode/`
- `fuzz/corpus/neon/`
- one directory matching each remaining target name listed above

Each committed corpus input must be:

- small enough to review manually, with a hard local limit of 64 KiB
- relevant to a previously fixed bug, a protocol boundary, or an edge case not
  already represented by deterministic tests
- non-sensitive and safe to publish
- named or documented well enough that reviewers can understand why it exists

Generated crashes, hangs, and local artifacts must stay out of commits. The
release gate rejects files under `fuzz/artifacts/` other than `.gitignore`.

## Running Local Campaigns

Install nightly and cargo-fuzz:

```sh
rustup toolchain install nightly
cargo install --locked cargo-fuzz --version 0.13.2
```

Run bounded smoke campaigns before release-sensitive stream or decode changes:

```sh
BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh
```

Use `BASE64_NG_FUZZ_RUNS=<n>` to change the per-target run count. The default
is `1000` runs for each target.

Run the release-duration campaign with:

```sh
BASE64_NG_RUN_FUZZ_RELEASE=1 \
BASE64_NG_FUZZ_SECONDS_PER_TARGET=3600 \
scripts/check_fuzz.sh
```

The release mode defaults to one hour per target. Both modes require zero
crash artifacts and record tool identities, parameters, final LibFuzzer
statistics, corpus counts and hashes, output hashes, and minimization status in
`MANIFEST.txt`. Generated corpus changes must be reviewed deliberately. Keep
only non-sensitive inputs that improve coverage or preserve a regression.

Opt-in smoke campaigns write release evidence under:

```text
target/release-evidence/fuzz/
```

Expected files:

- `decode.txt`
- `in_place.txt`
- `stream_chunks.txt`
- `differential.txt`
- `profiles.txt`
- `x86_encode.txt`
- `x86_decode.txt`
- `neon.txt`
- one `<target>.txt` file for each of the 18 targets
- `MANIFEST.txt`

Smoke campaigns use temporary corpus and artifact directories under
`target/release-evidence/fuzz/` so ordinary release smoke runs do not leave
generated files under committed `fuzz/corpus/` or `fuzz/artifacts/`.

## Assurance Scope

Base 2.0 has no persistent teardown provider. The assurance target therefore
models one volatile provider-instance generation and treats generation
termination as final: it exposes no restart, import, or resume parser that
could honestly be fuzzed. A future persistent provider must add a separate
authenticated parser and rollback, replay, corruption, torn-write,
compaction, and capacity campaign before making a persistence claim.

The fuzz-only unsafe provider is not a supported extension example. It exists
to reject invalid hook order and lifecycle claims. A deliberately unwinding
unsafe provider is exercised only in an isolated subprocess because unwinding
from those teardown hooks violates their unsafe contract and may abort during
double-panic cleanup.
