#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SANITIZER_TOOLCHAIN:-nightly}"
target="${BASE64_NG_SANITIZER_TARGET:-x86_64-unknown-linux-gnu}"

. scripts/evidence-source.sh
evidence_capture_source "2.0 sanitizer evidence"

if ! rustup run "$toolchain" rustc -V >/dev/null 2>&1; then
    echo "2.0 in-place sanitizers: missing toolchain $toolchain" >&2
    exit 1
fi

if ! rustup component list --toolchain "$toolchain" --installed | grep -q '^rust-src'; then
    echo "2.0 in-place sanitizers: rust-src is not installed for $toolchain" >&2
    exit 1
fi

evidence_dir="target/release-evidence/2.0-memory-sanitizers"
address_log="$evidence_dir/address-sanitizer.txt"
leak_log="$evidence_dir/leak-sanitizer.txt"
thread_log="$evidence_dir/thread-sanitizer.txt"
manifest="$evidence_dir/MANIFEST.txt"
mkdir -p "$evidence_dir"

echo "2.0 in-place sanitizers: AddressSanitizer overlap and cursor suite"
if env \
    RUSTFLAGS="-Zsanitizer=address" \
    RUSTDOCFLAGS="-Zsanitizer=address" \
    rustup run "$toolchain" cargo test \
        -Zbuild-std \
        --target "$target" \
        --all-features \
        --lib \
        'v2::in_place_tests' >"$address_log" 2>&1 && \
    env \
        RUSTFLAGS="-Zsanitizer=address" \
        RUSTDOCFLAGS="-Zsanitizer=address" \
        rustup run "$toolchain" cargo test \
            -Zbuild-std \
            --target "$target" \
            --all-features \
            --lib \
            'v2::secret_in_place_tests' >>"$address_log" 2>&1
then
    address_status=0
else
    address_status="$?"
fi
cat "$address_log"

leak_status=0
thread_status=0
if [ "$target" = "x86_64-unknown-linux-gnu" ]; then
    echo "2.0 memory sanitizers: LeakSanitizer secret ownership suite"
    if env \
        RUSTFLAGS="-Zsanitizer=leak" \
        rustup run "$toolchain" cargo test \
            -Zbuild-std \
            --target "$target" \
            --all-features \
            --lib \
            'v2::secret_' >"$leak_log" 2>&1
    then
        leak_status=0
    else
        leak_status="$?"
    fi
    cat "$leak_log"

    echo "2.0 memory sanitizers: ThreadSanitizer backend-health convergence"
    if env \
        RUSTFLAGS="-Zsanitizer=thread" \
        rustup run "$toolchain" cargo test \
            -Zbuild-std \
            --target "$target" \
            --all-features \
            --lib \
            'v2::backend_health::tests::concurrent_first_use_converges_without_waiting_on_testing' \
            -- --exact >"$thread_log" 2>&1
    then
        thread_status=0
    else
        thread_status="$?"
    fi
    cat "$thread_log"
else
    printf '%s\n' "skipped: LeakSanitizer campaign is admitted only on x86_64 Linux" >"$leak_log"
    printf '%s\n' "skipped: ThreadSanitizer campaign is admitted only on x86_64 Linux" >"$thread_log"
fi

evidence_verify_source "2.0 sanitizer evidence"

{
    echo "base64-ng 2.0 in-place sanitizer evidence"
    echo
    evidence_write_source_manifest
    echo
    rustup run "$toolchain" rustc -Vv
    rustup run "$toolchain" cargo -V
    echo "target=$target"
    echo "address_status=$address_status"
    echo "leak_status=$leak_status"
    echo "thread_status=$thread_status"
    echo "undefined_behavior_status=not-separately-supported-by-rustc-sanitizers"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$address_log" "$leak_log" "$thread_log"
    else
        shasum -a 256 "$address_log" "$leak_log" "$thread_log"
    fi
} >"$manifest"

for status in "$address_status" "$leak_status" "$thread_status"; do
    if [ "$status" -ne 0 ]; then
        exit "$status"
    fi
done

echo "2.0 memory sanitizers: ok"
