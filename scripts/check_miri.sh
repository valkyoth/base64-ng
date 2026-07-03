#!/usr/bin/env sh
set -eu

if ! rustup run nightly cargo miri --version >/dev/null 2>&1; then
    echo "Miri checks: skipping; nightly Miri is not installed"
    exit 0
fi

evidence_dir="target/release-evidence/miri"
no_default_output="$evidence_dir/no-default-features.txt"
all_features_output="$evidence_dir/all-features.txt"
manifest="$evidence_dir/MANIFEST.txt"

run_miri_case() {
    output="$1"
    feature_args="$2"
    test_filter="$3"

    echo "command: rustup run nightly cargo miri test $feature_args $test_filter -- --exact" >>"$output"
    rustup run nightly cargo miri test $feature_args "$test_filter" -- --exact >>"$output" 2>&1
}

mkdir -p "$evidence_dir"

echo "Miri checks: no-default-features scalar surface"
no_default_status=0
{
    echo "base64-ng Miri no-default-features evidence"
    echo
} >"$no_default_output"

for test_filter in \
    "tests::encodes_standard_vectors" \
    "tests::decodes_standard_vectors" \
    "tests::rejects_non_canonical_padding_bits" \
    "tests::supports_unpadded_url_safe" \
    "decode_backend::tests::boundary_uses_only_admitted_backends" \
    "errors::tests::index_offsets_saturate_on_overflow"
do
    if [ "$no_default_status" -eq 0 ]; then
        run_miri_case "$no_default_output" "--no-default-features" "$test_filter" || no_default_status="$?"
    fi
done
cat "$no_default_output"

if [ "$no_default_status" -ne 0 ]; then
    all_features_status=99
else
    echo "Miri checks: all-features scalar, alloc, and stream surface"
    all_features_status=0
    {
        echo "base64-ng Miri all-features evidence"
        echo
    } >"$all_features_output"

    for test_filter in \
        "tests::encodes_standard_vectors" \
        "tests::decodes_standard_vectors" \
        "decode_slice_clear_tail_scrubs_output_on_error" \
        "ct_decode_slice_staged_clear_tail_copies_only_after_success" \
        "stream_encoder_direct_write_buffers_tail_bytes" \
        "stream_decoder_direct_write_processes_multiple_quads" \
        "stream_decoder_fails_closed_after_malformed_input"
    do
        if [ "$all_features_status" -eq 0 ]; then
            run_miri_case "$all_features_output" "--all-features" "$test_filter" || all_features_status="$?"
        fi
    done
    cat "$all_features_output"
fi

{
    echo "base64-ng Miri evidence"
    echo
    echo "rustc:"
    rustup run nightly rustc -Vv
    echo
    echo "cargo:"
    rustup run nightly cargo -V
    echo
    echo "miri:"
    rustup run nightly cargo miri --version
    echo
    echo "system:"
    if command -v uname >/dev/null 2>&1; then
        uname -a
    else
        echo "uname unavailable"
    fi
    echo
    echo "commands:"
    echo "See no-default-features.txt and all-features.txt for exact per-test Miri commands."
    echo
    echo "status:"
    echo "no_default_features=$no_default_status"
    echo "all_features=$all_features_status"
    echo
    echo "artifacts:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$no_default_output" "$all_features_output" 2>/dev/null || true
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$no_default_output" "$all_features_output" 2>/dev/null || true
    else
        cksum "$no_default_output" "$all_features_output" 2>/dev/null || true
    fi
    echo
    echo "interpretation:"
    echo "This evidence records bounded Miri coverage for representative scalar, alloc, and stream surfaces on this machine."
    echo "Exhaustive/property-style parity sweeps are intentionally handled by normal test, nextest, hardware, and CI gates."
    echo "It checks undefined behavior that Miri can observe, but it is not a formal proof."
} >"$manifest"

echo "Miri checks: wrote $evidence_dir"

if [ "$no_default_status" -ne 0 ]; then
    exit "$no_default_status"
fi

if [ "$all_features_status" -ne 0 ]; then
    exit "$all_features_status"
fi

echo "Miri checks: ok"
