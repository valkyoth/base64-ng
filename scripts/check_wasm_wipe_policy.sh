#!/usr/bin/env sh
set -eu

target="${1:-wasm32-unknown-unknown}"
output="${TMPDIR:-/tmp}/base64-ng-wasm-wipe-policy.txt"

ensure_target_installed() {
    if ! command -v rustup >/dev/null 2>&1; then
        echo "wasm wipe policy: rustup is required to verify $target target availability" >&2
        exit 1
    fi

    if rustup target list --installed | grep -qx "$target"; then
        return
    fi

    echo "wasm wipe policy: installing missing Rust target $target"
    rustup target add "$target"
}

ensure_target_installed

echo "wasm wipe policy: checking ordinary portable codec for $target"
cargo check --target "$target" --no-default-features --lib
cargo check --target "$target" --lib

echo "wasm wipe policy: checking fail-closed secrets capability for $target"

if cargo check \
    --target "$target" \
    --no-default-features \
    --features secrets \
    --lib >"$output" 2>&1; then
    echo "wasm wipe policy: expected secrets $target build to fail closed" >&2
    cat "$output" >&2
    exit 1
fi

cat "$output"

if ! grep -F -q "allow-wasm32-best-effort-wipe" "$output"; then
    echo "wasm wipe policy: compile error did not mention the explicit allow feature" >&2
    exit 1
fi

if ! grep -F -q "compiler-fence-only wipe barrier" "$output"; then
    echo "wasm wipe policy: compile error did not explain the wasm cleanup caveat" >&2
    exit 1
fi

echo "wasm wipe policy: checking explicit allow feature for $target"
cargo check \
    --target "$target" \
    --no-default-features \
    --features secrets,allow-wasm32-best-effort-wipe \
    --lib

echo "wasm wipe policy: checking high-assurance rejection for $target"
if RUSTFLAGS="--cfg base64_ng_require_high_assurance" cargo check \
    --target "$target" \
    --no-default-features \
    --features secrets,allow-wasm32-best-effort-wipe \
    --lib >"$output" 2>&1; then
    echo "wasm wipe policy: high-assurance wasm build unexpectedly compiled" >&2
    exit 1
fi
if ! grep -F -q "unsupported or unattested" "$output"; then
    echo "wasm wipe policy: high-assurance rejection was not posture-specific" >&2
    cat "$output" >&2
    exit 1
fi

echo "wasm wipe policy: ok"
