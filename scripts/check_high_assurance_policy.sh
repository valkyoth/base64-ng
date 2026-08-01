#!/usr/bin/env sh
set -eu

output="${TMPDIR:-/tmp}/base64-ng-high-assurance-policy.txt"

expect_failure() {
    expected="$1"
    shift
    if "$@" >"$output" 2>&1; then
        echo "high-assurance policy: expected command to fail: $*" >&2
        exit 1
    fi
    if ! grep -F -q "$expected" "$output"; then
        echo "high-assurance policy: rejection did not mention: $expected" >&2
        cat "$output" >&2
        exit 1
    fi
}

echo "high-assurance policy: ordinary builds cannot claim secret assurance"
expect_failure \
    'requires the `secrets` capability' \
    env RUSTFLAGS="--cfg base64_ng_require_high_assurance" \
    cargo check --no-default-features --lib

echo "high-assurance policy: additive ordinary SIMD keeps scalar secret types"
RUSTFLAGS="--cfg base64_ng_require_high_assurance" \
    cargo check --no-default-features --features secrets,simd --lib

target_arch="$(rustc --print cfg | sed -n 's/^target_arch="\([^"]*\)"$/\1/p')"
case "$target_arch" in
    x86|x86_64)
        echo "high-assurance policy: checking native eligible target $target_arch"
        RUSTFLAGS="--cfg base64_ng_require_high_assurance" \
            cargo check --no-default-features --features secrets --lib
        ;;
    aarch64)
        echo "high-assurance policy: checking unattested AArch64 rejection"
        expect_failure \
            'unsupported or unattested' \
            env RUSTFLAGS="--cfg base64_ng_require_high_assurance" \
            cargo check --no-default-features --features secrets --lib
        echo "high-assurance policy: checking operator-attested AArch64 eligibility"
        RUSTFLAGS="--cfg base64_ng_require_high_assurance --cfg base64_ng_aarch64_csdb_attested" \
            cargo check --no-default-features --features secrets --lib
        ;;
    *)
        echo "high-assurance policy: checking unsupported target rejection for $target_arch"
        expect_failure \
            'unsupported or unattested' \
            env RUSTFLAGS="--cfg base64_ng_require_high_assurance" \
            cargo check --no-default-features --features secrets --lib
        ;;
esac

echo "high-assurance policy: eligibility gate ok"
