#!/usr/bin/env sh
set -eu

tmp="${TMPDIR:-/tmp}/base64-ng-big-endian-intrinsics.$$"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp"

check_unstable_intrinsics() {
    target="$1"
    module="$2"
    feature_gate="$3"
    source="$tmp/$target.rs"
    stderr="$tmp/$target.stderr"

    cat >"$source" <<EOF
#![no_std]

#[cfg(target_arch = "$module")]
use core::arch::$module::*;

#[no_mangle]
pub extern "C" fn base64_ng_intrinsics_probe() {}
EOF

    if rustc --target "$target" --crate-type lib "$source" >"$tmp/$target.stdout" 2>"$stderr"; then
        echo "big-endian intrinsics status: $module intrinsics compiled on stable" >&2
        echo "big-endian intrinsics status: revisit docs/BIG_ENDIAN_QEMU_REVIEW.md and SIMD admission before releasing" >&2
        exit 1
    fi

    if ! grep -F -q "$feature_gate" "$stderr"; then
        echo "big-endian intrinsics status: expected unstable feature gate $feature_gate for $module" >&2
        echo "big-endian intrinsics status: observed stderr:" >&2
        cat "$stderr" >&2
        exit 1
    fi

    echo "big-endian intrinsics status: $module remains gated by $feature_gate"
}

echo "big-endian intrinsics status: rustc=$(rustc --version)"
check_unstable_intrinsics s390x-unknown-linux-gnu s390x stdarch_s390x
check_unstable_intrinsics powerpc64-unknown-linux-gnu powerpc64 stdarch_powerpc
echo "big-endian intrinsics status: ok"
