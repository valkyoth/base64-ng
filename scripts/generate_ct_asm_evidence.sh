#!/usr/bin/env sh
set -eu

output_dir="target/release-evidence/asm"
mkdir -p "$output_dir"

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

echo "ct asm evidence: wrote $output_dir"
