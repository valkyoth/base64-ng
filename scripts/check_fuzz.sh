#!/usr/bin/env sh
set -eu

if [ ! -d fuzz ]; then
    echo "fuzz checks: skipping; fuzz/ is not present"
    exit 0
fi

echo "fuzz checks: compile harnesses"
cargo check --manifest-path fuzz/Cargo.toml --bins

echo "fuzz checks: corpus policy"
scripts/check_fuzz_corpus.sh

echo "fuzz checks: RustSec advisories"
cargo audit --file fuzz/Cargo.lock

echo "fuzz checks: dependency policy"
cargo deny --manifest-path fuzz/Cargo.toml --config fuzz/deny.toml check

if [ "${BASE64_NG_RUN_FUZZ_SMOKE:-0}" != "1" ]; then
    echo "fuzz checks: smoke campaigns skipped; set BASE64_NG_RUN_FUZZ_SMOKE=1 to run bounded campaigns"
    exit 0
fi

if ! cargo fuzz --version >/dev/null 2>&1; then
    echo "fuzz checks: cargo fuzz is not installed" >&2
    exit 1
fi

evidence_dir="target/release-evidence/fuzz"
manifest="$evidence_dir/MANIFEST.txt"
runs="${BASE64_NG_FUZZ_RUNS:-1000}"
mkdir -p "$evidence_dir"

{
    echo "base64-ng fuzz smoke evidence"
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "cargo:"
    cargo -V
    echo
    echo "cargo-fuzz:"
    cargo fuzz --version
    echo
    echo "parameters:"
    echo "runs=$runs"
    echo
    echo "targets:"
} >"$manifest"

for target in decode in_place stream_chunks differential profiles; do
    output="$evidence_dir/$target.txt"
    corpus_dir="$evidence_dir/corpus/$target"
    artifact_dir="$evidence_dir/artifacts/$target"
    mkdir -p "$corpus_dir" "$artifact_dir"
    echo "fuzz checks: smoke campaign $target ($runs runs)"
    if cargo +nightly fuzz run "$target" "$corpus_dir" -- \
        -artifact_prefix="$artifact_dir/" \
        -runs="$runs" >"$output" 2>&1
    then
        printf '%s=%s\n' "$target" "ok" >>"$manifest"
    else
        cat "$output"
        printf '%s=%s\n' "$target" "failed" >>"$manifest"
        exit 1
    fi
done

{
    echo
    echo "artifacts:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$evidence_dir"/*.txt
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$evidence_dir"/*.txt
    else
        cksum "$evidence_dir"/*.txt
    fi
} >>"$manifest"

echo "fuzz checks: wrote $evidence_dir"
