#!/usr/bin/env sh
set -eu

if [ ! -d kani ]; then
    echo "Advanced Kani checks: skipping; kani/ is not present"
    exit 0
fi

if ! cargo kani --version >/dev/null 2>&1; then
    echo "Advanced Kani checks: skipping; cargo kani is not installed"
    exit 0
fi

if [ -n "${RUSTFLAGS:-}" ]; then
    export RUSTFLAGS="$RUSTFLAGS --cfg base64_ng_kani_advanced"
else
    export RUSTFLAGS="--cfg base64_ng_kani_advanced"
fi

run_kani() {
    label="$1"
    shift
    log="$(mktemp)"

    echo "Advanced Kani checks: running $label"
    if cargo kani "$@" >"$log" 2>&1; then
        cat "$log"
        rm -f "$log"
        return 0
    else
        status="$?"
    fi

    if grep -q "Kani Rust Verifier" "$log" && grep -q "requires rustc" "$log"; then
        rm -f "$log"
        echo "Advanced Kani checks: skipping; installed Kani compiler is older than this crate's rust-version"
        exit 0
    fi

    cat "$log"
    rm -f "$log"
    exit "$status"
}

run_kani "advanced harness codegen" --no-default-features --only-codegen

if [ "${BASE64_NG_KANI_PROVE_PUBLIC_SURFACE:-0}" = "1" ]; then
    run_kani \
        "advanced_public_strict_decode_surfaces_do_not_panic_for_bounded_inputs" \
        --no-default-features \
        --harness "advanced_public_strict_decode_surfaces_do_not_panic_for_bounded_inputs"
else
    echo "Advanced Kani checks: skipped public-surface proof"
    echo "Advanced Kani checks: set BASE64_NG_KANI_PROVE_PUBLIC_SURFACE=1 to run it"
fi

if [ "${BASE64_NG_KANI_EXPENSIVE_WRAPPED:-0}" = "1" ]; then
    echo "Advanced Kani checks: running expensive wrapped proofs"
    run_kani \
        "advanced_wrapped_standard_decode_slice_returns_written_within_output" \
        --no-default-features \
        --harness "advanced_wrapped_standard_decode_slice_returns_written_within_output"
    run_kani \
        "advanced_wrapped_standard_decode_clear_tail_clears_output_on_error" \
        --no-default-features \
        --harness "advanced_wrapped_standard_decode_clear_tail_clears_output_on_error"
else
    echo "Advanced Kani checks: skipped expensive wrapped proofs"
    echo "Advanced Kani checks: set BASE64_NG_KANI_EXPENSIVE_WRAPPED=1 to run them"
fi
