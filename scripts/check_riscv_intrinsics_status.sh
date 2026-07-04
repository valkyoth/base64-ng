#!/usr/bin/env sh
set -eu

tmp="${TMPDIR:-/tmp}/base64-ng-riscv-intrinsics.$$"
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT

require_rust_target() {
    target="$1"
    if ! rustup target list --installed | grep -qx "$target"; then
        echo "RISC-V intrinsics status: installing missing Rust target $target"
        rustup target add "$target"
    fi
}

probe_riscv_intrinsics() {
    target="$1"
    feature_gate="$2"

    require_rust_target "$target"

    cat >"$tmp/probe.rs" <<'RS'
#![no_std]

use core::arch::riscv64::*;

pub unsafe fn probe() {}
RS

    if rustc --crate-type lib --target "$target" "$tmp/probe.rs" >"$tmp/stdout" 2>"$tmp/stderr"; then
        echo "RISC-V intrinsics status: riscv64 intrinsics compiled on stable" >&2
        echo "RISC-V intrinsics status: revisit docs/RISCV_QEMU_REVIEW.md and SIMD admission before releasing" >&2
        cat "$tmp/stdout" >&2
        cat "$tmp/stderr" >&2
        exit 1
    fi

    if ! grep -F -q "$feature_gate" "$tmp/stderr"; then
        echo "RISC-V intrinsics status: expected unstable feature gate $feature_gate" >&2
        echo "RISC-V intrinsics status: observed stderr:" >&2
        cat "$tmp/stderr" >&2
        exit 1
    fi

    echo "RISC-V intrinsics status: riscv64 remains gated by $feature_gate"
}

echo "RISC-V intrinsics status: rustc=$(rustc --version)"

probe_riscv_intrinsics "riscv64gc-unknown-linux-gnu" "riscv_ext_intrinsics"

echo "RISC-V intrinsics status: ok"
