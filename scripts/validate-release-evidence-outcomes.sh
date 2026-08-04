#!/usr/bin/env sh
set -eu

root="${1:-target/release-evidence}"

require_file() {
    if [ ! -s "$1" ]; then
        echo "release evidence outcomes: missing or empty artifact: $1" >&2
        exit 1
    fi
}

require_exact_key() {
    file="$1"
    key="$2"
    expected="$3"
    require_file "$file"
    actual="$(sed -n "s/^${key}=//p" "$file")"
    if [ "$actual" != "$expected" ]; then
        echo "release evidence outcomes: invalid $key in $file: ${actual:-missing}" >&2
        exit 1
    fi
}

for file in \
    "$root/kani/normal/status.txt" \
    "$root/kani/advanced/status.txt"
do
    require_file "$file"
    if [ "$(sed -n '1p' "$file")" != "PASS" ]; then
        echo "release evidence outcomes: required Kani status is not PASS: $file" >&2
        exit 1
    fi
done

for key in \
    no_default_features \
    all_features \
    base64_ng_bytes \
    base64_ng_tokio_readers \
    base64_ng_tokio_writers
do
    require_exact_key "$root/miri/MANIFEST.txt" "$key" 0
done
require_exact_key "$root/2.0-memory-sanitizers/MANIFEST.txt" address_status 0
require_exact_key "$root/2.0-memory-sanitizers/MANIFEST.txt" leak_status 0
require_exact_key "$root/2.0-memory-sanitizers/MANIFEST.txt" thread_status 0
require_exact_key "$root/2.0-memory-sanitizers/MANIFEST.txt" target x86_64-unknown-linux-gnu
require_exact_key "$root/dudect/MANIFEST.txt" status 0
require_exact_key "$root/backend/MANIFEST.txt" runtime_backend_report 0
require_exact_key "$root/backend/MANIFEST.txt" simd_prototype_equivalence 0
require_exact_key "$root/fuzz/MANIFEST.txt" mode release-duration

fuzz_targets="
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
if grep -E -q '^[a-z0-9_-]+=failed([[:space:]]|$)' "$root/fuzz/MANIFEST.txt"; then
    echo "release evidence outcomes: fuzz campaign contains a failed target" >&2
    exit 1
fi
for target in $fuzz_targets; do
    success_count="$(grep -E -c "^${target}=ok corpus=[0-9]+ artifacts=0$" \
        "$root/fuzz/MANIFEST.txt" || true)"
    if [ "$success_count" -ne 1 ]; then
        echo "release evidence outcomes: fuzz target $target lacks exactly one successful result" >&2
        exit 1
    fi
done

fuzz_duration="$(sed -n 's/^campaign_argument=-max_total_time=//p' "$root/fuzz/MANIFEST.txt")"
dudect_samples="$(sed -n 's/^samples=//p' "$root/dudect/MANIFEST.txt")"
dudect_iterations="$(sed -n 's/^iterations=//p' "$root/dudect/MANIFEST.txt")"
dudect_warmup="$(sed -n 's/^warmup=//p' "$root/dudect/MANIFEST.txt")"
for value in "$fuzz_duration" "$dudect_samples" "$dudect_iterations" "$dudect_warmup"; do
    case "$value" in
        '' | *[!0-9]*)
            echo "release evidence outcomes: campaign manifest contains a missing or invalid numeric parameter" >&2
            exit 1
            ;;
    esac
done
if [ "$fuzz_duration" -lt 3600 ]; then
    echo "release evidence outcomes: fuzz manifest lacks the one-hour-per-target floor" >&2
    exit 1
fi
if [ "$dudect_samples" -lt 20000 ] || [ "$dudect_iterations" -lt 64 ] || \
    [ "$dudect_warmup" -lt 1000 ]; then
    echo "release evidence outcomes: dudect manifest lacks the release parameter floors" >&2
    exit 1
fi
grep -F -q 'neon_automatic_dispatch=retained-native-performance' \
    "$root/commit-53/MANIFEST.txt" || {
    echo "release evidence outcomes: retained native NEON evidence is incomplete" >&2
    exit 1
}

echo "release evidence outcomes: all mandatory campaigns passed"
