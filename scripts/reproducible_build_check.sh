#!/usr/bin/env sh
set -eu

. scripts/evidence-source.sh
evidence_capture_source "reproducible build evidence"

evidence_dir="target/release-evidence/reproducible"
manifest="$evidence_dir/MANIFEST.txt"
mkdir -p target "$evidence_dir"
rm -f "$manifest"
first_target="$(mktemp -d target/base64-ng-repro-a.XXXXXX)"
second_target="$(mktemp -d target/base64-ng-repro-b.XXXXXX)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-reproducible.XXXXXX")"
manifest_tmp="$scratch/MANIFEST.txt"

cleanup() {
    rm -rf "$first_target" "$second_target" "$scratch"
}
trap cleanup EXIT INT TERM

SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export SOURCE_DATE_EPOCH

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

cp "$first_library" "$evidence_dir/release-a.rlib"
cp "$second_library" "$evidence_dir/release-b.rlib"
cp "$scratch/package-files-a.txt" "$evidence_dir/package-files-a.txt"
cp "$scratch/package-files-b.txt" "$evidence_dir/package-files-b.txt"
cp "$first_crate" "$evidence_dir/base64-ng-a.crate"
cp "$second_crate" "$evidence_dir/base64-ng-b.crate"

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
    evidence_checksum_file "$evidence_dir/release-a.rlib"
    evidence_checksum_file "$evidence_dir/release-b.rlib"
    evidence_checksum_file "$evidence_dir/package-files-a.txt"
    evidence_checksum_file "$evidence_dir/package-files-b.txt"
    evidence_checksum_file "$evidence_dir/base64-ng-a.crate"
    evidence_checksum_file "$evidence_dir/base64-ng-b.crate"
} >"$manifest_tmp"
mv "$manifest_tmp" "$manifest"

echo "reproducible build check: ok"
