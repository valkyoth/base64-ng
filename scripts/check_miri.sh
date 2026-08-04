#!/usr/bin/env sh
set -eu

if ! rustup run nightly cargo miri --version >/dev/null 2>&1; then
    if [ "${BASE64_NG_REQUIRE_MIRI:-0}" = "1" ]; then
        echo "Miri checks: nightly Miri is required for this release gate" >&2
        exit 1
    fi
    echo "Miri checks: skipping; nightly Miri is not installed"
    exit 0
fi

. scripts/evidence-source.sh
evidence_capture_source "Miri evidence"

evidence_dir="target/release-evidence/miri"
no_default_output="$evidence_dir/no-default-features.txt"
all_features_output="$evidence_dir/all-features.txt"
bytes_output="$evidence_dir/base64-ng-bytes.txt"
tokio_reader_output="$evidence_dir/base64-ng-tokio-readers.txt"
tokio_writer_output="$evidence_dir/base64-ng-tokio-writers.txt"
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
    "errors::tests::index_offsets_saturate_on_overflow" \
    "v2::in_place_tests::checked_byte_ranges_define_every_overlap_boundary"
do
    if [ "$no_default_status" -eq 0 ]; then
        run_miri_case "$no_default_output" "--no-default-features" "$test_filter" || no_default_status="$?"
    fi
done
cat "$no_default_output"

if [ "$no_default_status" -ne 0 ]; then
    all_features_status=99
    bytes_status=99
    tokio_reader_status=99
    tokio_writer_status=99
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
        "tests::encode_backend_boundary_uses_only_admitted_backends" \
        "decode_slice_clear_tail_scrubs_output_on_error" \
        "ct_decode_slice_staged_clear_tail_copies_only_after_success" \
        "stream_encoder_direct_write_buffers_tail_bytes" \
        "stream_decoder_direct_write_processes_multiple_quads" \
        "stream_decoder_fails_closed_after_malformed_input" \
        "v2::in_place_tests::ordinary_preflight_and_input_errors_do_not_mutate" \
        "v2::secret_in_place_tests::staged_secret_decode_miri_overlap_contract"
    do
        if [ "$all_features_status" -eq 0 ]; then
            run_miri_case "$all_features_output" "--all-features" "$test_filter" || all_features_status="$?"
        fi
    done
    cat "$all_features_output"

    if [ "$all_features_status" -ne 0 ]; then
        bytes_status=99
        tokio_reader_status=99
        tokio_writer_status=99
    else
        echo "Miri checks: base64-ng-bytes fragmented and panic boundaries"
        bytes_status=0
        {
            echo "base64-ng-bytes Miri evidence"
            echo
            echo "command: rustup run nightly cargo miri test --manifest-path crates/base64-ng-bytes/Cargo.toml --all-features --test bytes"
        } >"$bytes_output"
        rustup run nightly cargo miri test \
            --manifest-path crates/base64-ng-bytes/Cargo.toml \
            --all-features \
            --test bytes >>"$bytes_output" 2>&1 || bytes_status="$?"
        cat "$bytes_output"

        if [ "$bytes_status" -ne 0 ]; then
            tokio_reader_status=99
            tokio_writer_status=99
        else
            echo "Miri checks: base64-ng-tokio exact reader boundary"
            tokio_reader_status=0
            {
                echo "base64-ng-tokio AsyncRead Miri evidence"
                echo
                echo "command: rustup run nightly cargo miri test --manifest-path crates/base64-ng-tokio/Cargo.toml --test tokio_reader_adversarial exact_readers_stop_without_consuming_adjacent_frames -- --exact"
            } >"$tokio_reader_output"
            rustup run nightly cargo miri test \
                --manifest-path crates/base64-ng-tokio/Cargo.toml \
                --test tokio_reader_adversarial \
                exact_readers_stop_without_consuming_adjacent_frames \
                -- --exact >>"$tokio_reader_output" 2>&1 || tokio_reader_status="$?"
            cat "$tokio_reader_output"

            if [ "$tokio_reader_status" -ne 0 ]; then
                tokio_writer_status=99
            else
                echo "Miri checks: base64-ng-tokio writer cancellation boundary"
                tokio_writer_status=0
                {
                    echo "base64-ng-tokio AsyncWrite Miri evidence"
                    echo
                    echo "command: rustup run nightly cargo miri test --manifest-path crates/base64-ng-tokio/Cargo.toml --test tokio_writer_adversarial dropped_pending_write_futures_resume_without_loss_or_duplication -- --exact"
                } >"$tokio_writer_output"
                rustup run nightly cargo miri test \
                    --manifest-path crates/base64-ng-tokio/Cargo.toml \
                    --test tokio_writer_adversarial \
                    dropped_pending_write_futures_resume_without_loss_or_duplication \
                    -- --exact >>"$tokio_writer_output" 2>&1 || tokio_writer_status="$?"
                cat "$tokio_writer_output"
            fi
        fi
    fi
fi

evidence_verify_source "Miri evidence"

{
    echo "base64-ng Miri evidence"
    echo
    evidence_write_source_manifest
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
    echo "base64_ng_bytes=$bytes_status"
    echo "base64_ng_tokio_readers=$tokio_reader_status"
    echo "base64_ng_tokio_writers=$tokio_writer_status"
    echo
    echo "artifacts:"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$no_default_output" "$all_features_output" "$bytes_output" "$tokio_reader_output" "$tokio_writer_output" 2>/dev/null || true
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$no_default_output" "$all_features_output" "$bytes_output" "$tokio_reader_output" "$tokio_writer_output" 2>/dev/null || true
    else
        cksum "$no_default_output" "$all_features_output" "$bytes_output" "$tokio_reader_output" "$tokio_writer_output" 2>/dev/null || true
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

if [ "$bytes_status" -ne 0 ]; then
    exit "$bytes_status"
fi

if [ "$tokio_reader_status" -ne 0 ]; then
    exit "$tokio_reader_status"
fi

if [ "$tokio_writer_status" -ne 0 ]; then
    exit "$tokio_writer_status"
fi

echo "Miri checks: ok"
