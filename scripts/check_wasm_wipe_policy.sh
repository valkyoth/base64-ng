#!/usr/bin/env sh
set -eu

target="${1:-wasm32-unknown-unknown}"
output="${TMPDIR:-/tmp}/base64-ng-wasm-wipe-policy.txt"

echo "wasm wipe policy: checking fail-closed feature for $target"

if cargo check \
    --target "$target" \
    --no-default-features \
    --features deny-wasm32-best-effort-wipe \
    --lib >"$output" 2>&1; then
    echo "wasm wipe policy: expected $target build to fail closed" >&2
    cat "$output" >&2
    exit 1
fi

cat "$output"

if ! grep -F -q "deny-wasm32-best-effort-wipe" "$output"; then
    echo "wasm wipe policy: compile error did not mention the fail-closed feature" >&2
    exit 1
fi

if ! grep -F -q "compiler-fence-only wipe barrier" "$output"; then
    echo "wasm wipe policy: compile error did not explain the wasm cleanup caveat" >&2
    exit 1
fi

echo "wasm wipe policy: ok"
