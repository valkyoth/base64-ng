#!/usr/bin/env sh
set -eu

target="${1:-}"
collection="${2:-target/fuzz-shards}"
duration="${3:-3600}"
machine_label="${BASE64_NG_FUZZ_MACHINE_LABEL:-$(uname -n)}"

if [ -z "$target" ]; then
    echo "usage: scripts/capture-fuzz-shard.sh <target> [collection] [seconds]" >&2
    exit 2
fi
if ! grep -F -x -q "$target" scripts/fuzz-release-targets.txt; then
    echo "fuzz shard capture: unknown release target: $target" >&2
    exit 2
fi
case "$duration" in
    '' | *[!0-9]*)
        echo "fuzz shard capture: duration must be an integer" >&2
        exit 2
        ;;
esac
if [ "$duration" -lt 3600 ]; then
    echo "fuzz shard capture: release shards require at least 3600 seconds" >&2
    exit 2
fi
case "$machine_label" in
    '' | *[!A-Za-z0-9._-]*)
        echo "fuzz shard capture: machine label must use A-Z, a-z, 0-9, dot, underscore, or hyphen" >&2
        exit 2
        ;;
esac

root="$(pwd -P)"
target_root="$root/target"
mkdir -p "$target_root"
collection_parent="$(dirname "$collection")"
mkdir -p "$collection_parent"
collection_parent="$(cd "$collection_parent" && pwd -P)"
collection="$collection_parent/$(basename "$collection")"
case "$collection" in
    "$target_root"/*) ;;
    *)
        echo "fuzz shard capture: collection must be under the ignored target/ directory" >&2
        exit 2
        ;;
esac
if [ -e "$collection/$target" ]; then
    echo "fuzz shard capture: refusing to replace existing bundle: $collection/$target" >&2
    exit 1
fi

if ! cargo +nightly fuzz --version >/dev/null 2>&1; then
    echo "fuzz shard capture: nightly and cargo-fuzz are required" >&2
    exit 1
fi

. scripts/evidence-source.sh
evidence_capture_source "fuzz shard capture"
source_tree="$(git rev-parse 'HEAD^{tree}')"

sha256_value() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

host_arch="$(uname -m)"
host_endian="$(python3 -c 'import sys; print(sys.byteorder)')"
rustc_host="$(rustc +nightly -Vv | sed -n 's/^host: //p')"
architecture_class="portable"
cpu_features="not-required"

cpu_text=""
if [ -r /proc/cpuinfo ]; then
    cpu_text="$(cat /proc/cpuinfo)"
elif command -v sysctl >/dev/null 2>&1; then
    cpu_text="$(sysctl -a 2>/dev/null || true)"
fi

case "$target" in
    x86_encode | x86_decode)
        case "$host_arch" in
            x86_64 | amd64) ;;
            *)
                echo "fuzz shard capture: $target requires an x86-64 host" >&2
                exit 1
                ;;
        esac
        for feature in avx512f avx512bw avx512vl avx512vbmi; do
            pattern="$feature"
            if [ "$feature" = "avx512vbmi" ]; then
                pattern='avx512_?vbmi'
            fi
            if ! printf '%s\n' "$cpu_text" | grep -E -i -q "(^|[[:space:]])${pattern}([[:space:]]|$)"; then
                echo "fuzz shard capture: $target requires CPU feature $feature" >&2
                exit 1
            fi
        done
        architecture_class="x86_64-avx512vbmi"
        cpu_features="avx512f,avx512bw,avx512vl,avx512vbmi"
        ;;
    neon)
        case "$host_arch" in
            aarch64 | arm64) ;;
            *)
                echo "fuzz shard capture: neon requires a native AArch64 host" >&2
                exit 1
                ;;
        esac
        if [ "$host_endian" != "little" ]; then
            echo "fuzz shard capture: neon requires a little-endian host" >&2
            exit 1
        fi
        architecture_class="aarch64-neon"
        cpu_features="neon"
        ;;
esac

echo "fuzz shard capture: compile $target"
cargo check --manifest-path fuzz/Cargo.toml --bin "$target"
scripts/check_fuzz_corpus.sh

temporary="$(mktemp -d "$target_root/.fuzz-shard-${target}.XXXXXX")"
work="$temporary/work"
bundle="$temporary/$target"
corpus="$work/corpus"
artifacts="$work/artifacts"
mkdir -p "$corpus" "$artifacts" "$bundle"

cleanup() {
    rm -rf "$temporary"
}
trap cleanup EXIT INT TERM

if [ -d "fuzz/corpus/$target" ]; then
    cp -R "fuzz/corpus/$target/." "$corpus/"
    find "$corpus" -type f -name .gitkeep -delete
fi

{
    echo "base64-ng distributed fuzz environment"
    echo "target=$target"
    echo "machine_label=$machine_label"
    echo "command=cargo +nightly fuzz run $target <corpus> -- -artifact_prefix=<artifacts>/ -print_final_stats=1 -max_total_time=$duration"
    echo
    echo "uname:"
    uname -a
    echo
    echo "rustc:"
    rustc +nightly -Vv
    echo
    echo "cargo:"
    cargo +nightly -V
    echo
    echo "cargo-fuzz:"
    cargo +nightly fuzz --version
    echo
    echo "cpu:"
    if command -v lscpu >/dev/null 2>&1; then
        lscpu
    elif command -v sysctl >/dev/null 2>&1; then
        sysctl -a 2>/dev/null || true
    else
        printf '%s\n' "$cpu_text"
    fi
} >"$bundle/environment.txt"

started="$(date +%s)"
echo "fuzz shard capture: running $target for $duration seconds"
if cargo +nightly fuzz run "$target" "$corpus" -- \
    -artifact_prefix="$artifacts/" \
    -print_final_stats=1 \
    -max_total_time="$duration" >"$bundle/campaign.log" 2>&1
then
    status=0
else
    status=$?
fi
finished="$(date +%s)"
elapsed="$((finished - started))"

if [ "$status" -ne 0 ]; then
    failed_log="$collection/${target}.failed.log"
    mkdir -p "$collection"
    cp "$bundle/campaign.log" "$failed_log"
    cat "$bundle/campaign.log"
    echo "fuzz shard capture: $target failed; retained log at $failed_log" >&2
    exit "$status"
fi

artifact_count="$(find "$artifacts" -type f | wc -l | tr -d '[:space:]')"
if [ "$artifact_count" != "0" ]; then
    echo "fuzz shard capture: $target left $artifact_count crash artifacts" >&2
    exit 1
fi
if [ "$elapsed" -lt "$duration" ]; then
    echo "fuzz shard capture: $target returned before its declared duration" >&2
    exit 1
fi
for marker in 'stat::number_of_executed_units:' 'stat::average_exec_per_sec:'; do
    if ! grep -F -q "$marker" "$bundle/campaign.log"; then
        echo "fuzz shard capture: $target is missing final statistic $marker" >&2
        exit 1
    fi
done

corpus_count="$(find "$corpus" -type f | wc -l | tr -d '[:space:]')"
tar -C "$corpus" -czf "$bundle/corpus.tar.gz" .

cat >"$bundle/MANIFEST.txt" <<EOF
schema=base64-ng-fuzz-shard-v1
target=$target
status=ok
source_commit=$EVIDENCE_SOURCE_COMMIT
source_tree=$source_tree
tree_state=$EVIDENCE_TREE_STATE
cargo_lock_sha256=$(sha256_value Cargo.lock)
fuzz_lock_sha256=$(sha256_value fuzz/Cargo.lock)
fuzz_manifest_sha256=$(sha256_value fuzz/Cargo.toml)
harness_sha256=$(sha256_value "fuzz/fuzz_targets/$target.rs")
duration_seconds=$duration
started_epoch=$started
finished_epoch=$finished
elapsed_seconds=$elapsed
architecture_class=$architecture_class
host_arch=$host_arch
host_endian=$host_endian
rustc_host=$rustc_host
cpu_features=$cpu_features
machine_label=$machine_label
corpus_count=$corpus_count
artifact_count=0
log_sha256=$(sha256_value "$bundle/campaign.log")
environment_sha256=$(sha256_value "$bundle/environment.txt")
corpus_archive_sha256=$(sha256_value "$bundle/corpus.tar.gz")
EOF

evidence_verify_source "fuzz shard capture"
python3 scripts/fuzz_shard_evidence.py validate "$bundle"
mkdir -p "$collection"
mv "$bundle" "$collection/$target"
trap - EXIT INT TERM
rm -rf "$temporary"
echo "fuzz shard capture: wrote $collection/$target"
