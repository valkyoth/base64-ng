#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/asm"
manifest="$output_dir/MANIFEST.txt"
mkdir -p "$output_dir"

checksum_file() {
    file="$1"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file"
    else
        cksum "$file"
    fi
}

copy_single_asm() {
    target_dir="$1"
    output_file="$2"
    asm_file="$(
        find "$target_dir/release/deps" -maxdepth 1 -type f -name 'base64_ng-*.s' \
            | sort \
            | sed -n '1p'
    )"

    if [ -z "$asm_file" ]; then
        echo "ct asm evidence: no assembly file found under $target_dir" >&2
        exit 1
    fi

    cp "$asm_file" "$output_file"
    test -s "$output_file"
}

echo "ct asm evidence: no-default-features release assembly"
CARGO_TARGET_DIR="target/ct-asm-no-default" \
    cargo rustc --release --lib --no-default-features -- --emit=asm
copy_single_asm "target/ct-asm-no-default" "$output_dir/base64_ng-no-default-features.s"

echo "ct asm evidence: all-features release assembly"
CARGO_TARGET_DIR="target/ct-asm-all-features" \
    cargo rustc --release --lib --all-features -- --emit=asm
copy_single_asm "target/ct-asm-all-features" "$output_dir/base64_ng-all-features.s"

{
    echo "base64-ng constant-time assembly evidence"
    echo
    echo "rustc:"
    rustc -Vv
    echo
    echo "commands:"
    echo "CARGO_TARGET_DIR=target/ct-asm-no-default cargo rustc --release --lib --no-default-features -- --emit=asm"
    echo "CARGO_TARGET_DIR=target/ct-asm-all-features cargo rustc --release --lib --all-features -- --emit=asm"
    echo
    echo "artifacts:"
    checksum_file "$output_dir/base64_ng-no-default-features.s"
    checksum_file "$output_dir/base64_ng-all-features.s"
    echo
    echo "review focus:"
    echo "- ct::CtEngine decode entry points"
    echo "- ct_decode_* scalar helper code"
    echo "- ct_decode_ascii_base64 symbol mapping"
    echo "- ct_mask_* arithmetic helpers"
    echo "- absence of secret-indexed lookup tables in ct symbol mapping"
    echo "- absence of secret-byte-class branches in fixed-length ct decode loops"
} >"$manifest"

echo "ct asm evidence: wrote $output_dir"
