#!/usr/bin/env sh
set -eu

if [ ! -d fuzz ]; then
    if [ "${BASE64_NG_RUN_FUZZ_RELEASE:-0}" = "1" ]; then
        echo "fuzz checks: fuzz/ is required for the release campaign" >&2
        exit 1
    fi
    echo "fuzz checks: skipping; fuzz/ is not present"
    exit 0
fi

if grep -R -n -E 'catch_unwind|resume_unwind' fuzz/fuzz_targets; then
    echo "fuzz checks: unwind-catching APIs are invalid under libFuzzer panic-abort execution" >&2
    echo "fuzz checks: put deliberate panic injection in an unwind-capable unit test" >&2
    exit 1
fi

if [ "${BASE64_NG_RUN_FUZZ_SMOKE:-0}" = "1" ] || \
    [ "${BASE64_NG_RUN_FUZZ_RELEASE:-0}" = "1" ]; then
    mkdir -p target/release-evidence/fuzz
    rm -f \
        target/release-evidence/fuzz/MANIFEST.txt \
        target/release-evidence/fuzz/MANIFEST.txt.tmp
fi

echo "fuzz checks: compile harnesses"
cargo check --manifest-path fuzz/Cargo.toml --bins

echo "fuzz checks: corpus policy"
scripts/check_fuzz_corpus.sh

echo "fuzz checks: RustSec advisories"
cargo audit --file fuzz/Cargo.lock

echo "fuzz checks: dependency policy"
scripts/cargo-deny-check.sh fuzz/Cargo.toml fuzz/deny.toml

if [ "${BASE64_NG_RUN_FUZZ_SMOKE:-0}" != "1" ] &&
    [ "${BASE64_NG_RUN_FUZZ_RELEASE:-0}" != "1" ]
then
    echo "fuzz checks: campaigns skipped; set BASE64_NG_RUN_FUZZ_SMOKE=1 or BASE64_NG_RUN_FUZZ_RELEASE=1"
    exit 0
fi

if ! cargo fuzz --version >/dev/null 2>&1; then
    echo "fuzz checks: cargo fuzz is not installed" >&2
    exit 1
fi

evidence_dir="target/release-evidence/fuzz"
manifest="$evidence_dir/MANIFEST.txt"
manifest_tmp="$evidence_dir/MANIFEST.txt.tmp"
targets="
decode
in_place
stream_chunks
differential
profiles
x86_encode
x86_decode
neon
mime_body
pem_document
multibase_family
imap_payload
password_records
openpgp_armor
v2_runtime_codec
v2_incremental
v2_async
v2_assurance
"
if [ "${BASE64_NG_RUN_FUZZ_RELEASE:-0}" = "1" ]; then
    mode="release-duration"
    duration="${BASE64_NG_FUZZ_SECONDS_PER_TARGET:-3600}"
    case "$duration" in
        '' | *[!0-9]*)
            echo "fuzz checks: release duration must be an integer" >&2
            exit 1
            ;;
    esac
    if [ "$duration" -lt 3600 ]; then
        echo "fuzz checks: release duration must be at least 3600 seconds per target" >&2
        exit 1
    fi
    campaign_argument="-max_total_time=$duration"
else
    mode="bounded-smoke"
    runs="${BASE64_NG_FUZZ_RUNS:-1000}"
    campaign_argument="-runs=$runs"
fi
mkdir -p "$evidence_dir"

cleanup_manifest() {
    rm -f "$manifest_tmp"
}
trap cleanup_manifest EXIT INT TERM

. scripts/evidence-source.sh
evidence_capture_source "fuzz campaign evidence"

{
    echo "base64-ng fuzz campaign evidence"
    echo
    evidence_write_source_manifest
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
    echo "mode=$mode"
    echo "campaign_argument=$campaign_argument"
    echo "panic_oracle=crate-originated panic is a campaign failure"
    echo "artifact_oracle=zero artifacts required"
    echo
    echo "targets:"
} >"$manifest_tmp"

for target in $targets; do
    output="$evidence_dir/$target.txt"
    corpus_dir="$evidence_dir/corpus/$target"
    artifact_dir="$evidence_dir/artifacts/$target"
    mkdir -p "$corpus_dir" "$artifact_dir"
    echo "fuzz checks: $mode campaign $target ($campaign_argument)"
    if cargo +nightly fuzz run "$target" "$corpus_dir" -- \
        -artifact_prefix="$artifact_dir/" \
        -print_final_stats=1 \
        "$campaign_argument" >"$output" 2>&1
    then
        artifact_count="$(find "$artifact_dir" -type f | wc -l | tr -d '[:space:]')"
        corpus_count="$(find "$corpus_dir" -type f | wc -l | tr -d '[:space:]')"
        if [ "$artifact_count" != "0" ]; then
            echo "fuzz checks: $target left $artifact_count crash artifacts" >&2
            exit 1
        fi
        printf '%s=%s corpus=%s artifacts=%s\n' \
            "$target" "ok" "$corpus_count" "$artifact_count" >>"$manifest_tmp"
        grep '^stat::' "$output" >>"$manifest_tmp" || true
    else
        cat "$output"
        exit 1
    fi
done

evidence_verify_source "fuzz campaign evidence"

{
    echo
    echo "campaign-output-hashes:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$evidence_dir"/*.txt
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$evidence_dir"/*.txt
    else
        cksum "$evidence_dir"/*.txt
    fi
    echo
    echo "corpus-hashes:"
    corpus_files="$(find "$evidence_dir/corpus" -type f -print)"
    if [ -z "$corpus_files" ]; then
        echo "none"
    elif command -v sha256sum >/dev/null 2>&1; then
        find "$evidence_dir/corpus" -type f -exec sha256sum {} \;
    elif command -v shasum >/dev/null 2>&1; then
        find "$evidence_dir/corpus" -type f -exec shasum -a 256 {} \;
    else
        find "$evidence_dir/corpus" -type f -exec cksum {} \;
    fi
    echo
    echo "minimization: no crashing artifact remained; no crash minimization required"
} >>"$manifest_tmp"

mv "$manifest_tmp" "$manifest"
trap - EXIT INT TERM

echo "fuzz checks: wrote $evidence_dir"
