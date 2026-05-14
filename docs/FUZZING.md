# Fuzzing Policy

`base64-ng` keeps fuzzing isolated from the published crate. The root crate
remains dependency-free; fuzz-only dependencies live under `fuzz/` and are
checked separately.

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

- `decode`: arbitrary strict, legacy, and constant-time-oriented decode input
- `in_place`: in-place encode/decode and legacy compaction behavior
- `stream_chunks`: fragmented stream reader/writer state machines and adjacent
  framed payload boundaries
- `differential`: canonical output comparison against the established Base64
  behavior used by the harness

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
cargo +nightly fuzz run decode -- -runs=1000
cargo +nightly fuzz run in_place -- -runs=1000
cargo +nightly fuzz run stream_chunks -- -runs=1000
cargo +nightly fuzz run differential -- -runs=1000
```

Longer campaigns are useful before release candidates, but generated corpus
changes should be reviewed deliberately. Keep only the inputs that improve
coverage or preserve a regression.
