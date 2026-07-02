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

`scripts/check_fuzz.sh` also runs the fuzz workspace supply-chain gates:

```sh
cargo audit --file fuzz/Cargo.lock
cargo deny --manifest-path fuzz/Cargo.toml check --config fuzz/deny.toml
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
cargo install --locked cargo-fuzz
```

Run bounded smoke campaigns before release-sensitive stream or decode changes:

```sh
BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh
```

Use `BASE64_NG_FUZZ_RUNS=<n>` to change the per-target run count. The default
is `1000` runs for each target.

Longer campaigns are useful before release candidates, but generated corpus
changes should be reviewed deliberately. Keep only the inputs that improve
coverage or preserve a regression.

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
- `MANIFEST.txt`

Smoke campaigns use temporary corpus and artifact directories under
`target/release-evidence/fuzz/` so ordinary release smoke runs do not leave
generated files under committed `fuzz/corpus/` or `fuzz/artifacts/`.
