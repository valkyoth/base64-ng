#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_SANITIZER_TOOLCHAIN:-nightly}"
target="${BASE64_NG_SANITIZER_TARGET:-x86_64-unknown-linux-gnu}"

if ! rustup run "$toolchain" rustc -V >/dev/null 2>&1; then
    echo "2.0 in-place sanitizers: missing toolchain $toolchain" >&2
    exit 1
fi

if ! rustup component list --toolchain "$toolchain" --installed | grep -q '^rust-src'; then
    echo "2.0 in-place sanitizers: rust-src is not installed for $toolchain" >&2
    exit 1
fi

evidence_dir="target/release-evidence/2.0-in-place-sanitizers"
log="$evidence_dir/address-sanitizer.txt"
manifest="$evidence_dir/MANIFEST.txt"
mkdir -p "$evidence_dir"

echo "2.0 in-place sanitizers: AddressSanitizer overlap and cursor suite"
if env \
    RUSTFLAGS="-Zsanitizer=address" \
    RUSTDOCFLAGS="-Zsanitizer=address" \
    rustup run "$toolchain" cargo test \
        -Zbuild-std \
        --target "$target" \
        --lib \
        'v2::in_place_tests' >"$log" 2>&1 && \
    env \
        RUSTFLAGS="-Zsanitizer=address" \
        RUSTDOCFLAGS="-Zsanitizer=address" \
        rustup run "$toolchain" cargo test \
            -Zbuild-std \
            --target "$target" \
            --lib \
            'v2::secret_in_place_tests' >>"$log" 2>&1
then
    status=0
else
    status="$?"
fi
cat "$log"

{
    echo "base64-ng 2.0 in-place sanitizer evidence"
    echo
    rustup run "$toolchain" rustc -Vv
    rustup run "$toolchain" cargo -V
    echo "target=$target"
    echo "status=$status"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$log"
    else
        shasum -a 256 "$log"
    fi
} >"$manifest"

if [ "$status" -ne 0 ]; then
    exit "$status"
fi

echo "2.0 in-place sanitizers: ok"
