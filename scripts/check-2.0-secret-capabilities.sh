#!/usr/bin/env sh
set -eu

document="docs/2.0_SECRET_CAPABILITY_POLICY.md"
workdir="target/2_0_secret_capabilities"
mkdir -p "$workdir/src"
for required in \
    'Ordinary public-data builds' \
    'best-effort cleanup' \
    'OOM abort' \
    '`mem::forget`' \
    'does not attest protected storage' \
    'Commit 22'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 secret capabilities: documentation is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 secret capabilities: ordinary portability"
cargo check --no-default-features --lib
cargo check --lib

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-secret-capability-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[features]
secrets = ["base64-ng/secrets"]

[dependencies]
base64-ng = { path = "../..", default-features = false }
TOML
cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::STRICT_STANDARD_PADDED;

fn main() {
    let _ = STRICT_STANDARD_PADDED.secret_decode_staging_len(8);
}
RS
if cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" \
    >"$workdir/ordinary-secret-surface.log" 2>&1
then
    echo "2.0 secret capabilities: secret operation leaked into ordinary API" >&2
    exit 1
fi
if ! grep -F -q 'no method named `secret_decode_staging_len`' \
    "$workdir/ordinary-secret-surface.log"
then
    echo "2.0 secret capabilities: ordinary API failed for an unexpected reason" >&2
    cat "$workdir/ordinary-secret-surface.log" >&2
    exit 1
fi
cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" --features secrets

echo "2.0 secret capabilities: best-effort secret matrix"
cargo test --no-default-features --features secrets --lib 'v2::secret'
cargo test --all-features --lib 'v2::secret'

echo "2.0 secret capabilities: build-policy matrix"
scripts/check_high_assurance_policy.sh
scripts/check_wasm_wipe_policy.sh

echo "2.0 secret capabilities: structural and lifecycle policy ok"
