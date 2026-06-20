#!/usr/bin/env sh
set -eu

expected_root="base64-ng v"
tree_output="$(
    cargo tree -p base64-ng --all-features --edges normal,build,dev --prefix none --no-dedupe
)"
line_count="$(
    printf '%s\n' "$tree_output" | sed '/^[[:space:]]*$/d' | wc -l | tr -d '[:space:]'
)"
first_line="$(
    printf '%s\n' "$tree_output" | sed -n '1p'
)"

case "$first_line" in
    "$expected_root"*) ;;
    *)
        echo "dependency policy: unexpected cargo tree root:" >&2
        printf '%s\n' "$first_line" >&2
        exit 1
        ;;
esac

if [ "$line_count" != "1" ]; then
    echo "dependency policy: external crate dependencies are not allowed without review" >&2
    printf '%s\n' "$tree_output" >&2
    exit 1
fi

echo "dependency policy: zero external crates"
