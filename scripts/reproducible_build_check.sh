#!/usr/bin/env sh
set -eu

. scripts/evidence-source.sh
evidence_capture_source "reproducible build evidence"

first_target="${BASE64_NG_REPRO_TARGET_A:-target/reproducible-a}"
second_target="${BASE64_NG_REPRO_TARGET_B:-target/reproducible-b}"
evidence_dir="target/release-evidence/reproducible"
manifest="$evidence_dir/MANIFEST.txt"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-reproducible.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT INT TERM

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export SOURCE_DATE_EPOCH

rm -rf "$first_target" "$second_target"
mkdir -p "$evidence_dir"

CARGO_TARGET_DIR="$first_target" cargo build --release --locked
CARGO_TARGET_DIR="$second_target" cargo build --release --locked

first_library="$first_target/release/libbase64_ng.rlib"
second_library="$second_target/release/libbase64_ng.rlib"
test -s "$first_library"
test -s "$second_library"
cmp "$first_library" "$second_library"

CARGO_TARGET_DIR="$first_target" cargo package --locked --allow-dirty --list \
    >"$scratch/package-files-a.txt"
CARGO_TARGET_DIR="$first_target" cargo package --locked --allow-dirty
CARGO_TARGET_DIR="$second_target" cargo package --locked --allow-dirty --list \
    >"$scratch/package-files-b.txt"
CARGO_TARGET_DIR="$second_target" cargo package --locked --allow-dirty

cmp "$scratch/package-files-a.txt" "$scratch/package-files-b.txt"

version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p')"
first_crate="$first_target/package/base64-ng-$version.crate"
second_crate="$second_target/package/base64-ng-$version.crate"
test -s "$first_crate"
test -s "$second_crate"
cmp "$first_crate" "$second_crate"

evidence_verify_source "reproducible build evidence"

{
    echo "base64-ng reproducible build evidence"
    echo
    evidence_write_source_manifest
    echo
    echo "SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
    echo "comparisons=release-rlib,package-file-list,crate-archive"
    echo
    echo "artifacts:"
    evidence_checksum_file "$first_library"
    evidence_checksum_file "$second_library"
    evidence_checksum_file "$scratch/package-files-a.txt"
    evidence_checksum_file "$scratch/package-files-b.txt"
    evidence_checksum_file "$first_crate"
    evidence_checksum_file "$second_crate"
} >"$manifest"

echo "reproducible build check: ok"
