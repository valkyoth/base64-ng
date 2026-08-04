#!/usr/bin/env sh
set -eu

check_fixed_scan() {
    source_file="$1"
    function_name="$2"
    body="$(sed -n "/fn $function_name(/,/^}/p" "$source_file")"

    if [ -z "$body" ]; then
        echo "2.0 timing boundaries: missing $function_name in $source_file" >&2
        exit 1
    fi
    if ! printf '%s\n' "$body" | grep -F -q 'while candidate < 64'; then
        echo "2.0 timing boundaries: $function_name lost its fixed 64-entry scan" >&2
        exit 1
    fi
    if ! printf '%s\n' "$body" | grep -F -q 'candidate += 1'; then
        echo "2.0 timing boundaries: $function_name lost its public counter step" >&2
        exit 1
    fi
    if printf '%s\n' "$body" | grep -E -q \
        '^[[:space:]]*(if|match|return|break|continue)([[:space:](]|$)'
    then
        echo "2.0 timing boundaries: forbidden data-dependent exit in $function_name" >&2
        exit 1
    fi
}

check_branchless_mapper() {
    source_file="$1"
    function_name="$2"
    body="$(sed -n "/fn $function_name(/,/^}/p" "$source_file")"

    if [ -z "$body" ]; then
        echo "2.0 timing boundaries: missing $function_name in $source_file" >&2
        exit 1
    fi
    if printf '%s\n' "$body" | grep -E -q \
        '^[[:space:]]*(if|match|while|for|return|break|continue)([[:space:](]|$)'
    then
        echo "2.0 timing boundaries: branch or early exit in $function_name" >&2
        exit 1
    fi
}

check_fixed_scan src/v2/secret_decoder.rs decode_symbol
check_fixed_scan src/v2/secret_in_place.rs decode_symbol
check_fixed_scan src/v2/secret_encoder.rs secret_encode_scan
check_branchless_mapper src/v2/secret_encoder.rs secret_encode_ascii

for secret_source in \
    src/v2/secret_decoder.rs \
    src/v2/secret_encoder.rs \
    src/v2/secret_in_place.rs \
    src/v2/secret/encoders.rs \
    src/v2/secret/frames.rs
do
    if grep -E -q 'crate::simd|decode_backend::|encode_backend::' "$secret_source"; then
        echo "2.0 timing boundaries: secret path entered ordinary dispatch: $secret_source" >&2
        exit 1
    fi
done

for required_gate_text in \
    '#[inline(never)]' \
    'pub(crate) fn ct_error_gate_barrier' \
    'core::sync::atomic::compiler_fence' \
    'core::arch::asm!("lfence"' \
    'core::arch::asm!("isb sy", "hint #20"' \
    'core::arch::asm!("fence rw, rw"'
do
    if ! grep -F -q "$required_gate_text" src/ct/equality.rs; then
        echo "2.0 timing boundaries: result gate is missing: $required_gate_text" >&2
        exit 1
    fi
done

for required_wipe_text in \
    'WIPE_PRIMITIVE_REVISION' \
    'core::ptr::write_volatile(byte, 0)' \
    'wipe_barrier(bytes.as_mut_ptr(), bytes.len())' \
    'core::sync::atomic::compiler_fence'
do
    if ! grep -F -q "$required_wipe_text" src/cleanup.rs; then
        echo "2.0 timing boundaries: wipe boundary is missing: $required_wipe_text" >&2
        exit 1
    fi
done

for required_timing_case in \
    'valid-vs-invalid-fixed-work-pre-gate' \
    'reviewed-equality-mismatch-position' \
    'decode-public-length-scaling' \
    'encode-public-length-scaling' \
    'equality-public-length-scaling' \
    'public-length-may-differ' \
    'equal-work'
do
    if ! grep -F -q "$required_timing_case" dudect/src/main.rs; then
        echo "2.0 timing boundaries: dudect case is missing: $required_timing_case" >&2
        exit 1
    fi
done

for required_doc_text in \
    'Fixed-work pre-result-gate boundary' \
    'Success-only post-result-gate boundary' \
    'runtime wipe generation' \
    'compiler-created copies' \
    'does not prove valid/invalid whole-call timing equality'
do
    if ! grep -F -q "$required_doc_text" docs/2.0_TIMING_AND_CODEGEN.md; then
        echo "2.0 timing boundaries: documentation is missing: $required_doc_text" >&2
        exit 1
    fi
done

echo "2.0 timing boundaries: fixed-work loops, result gate, wipe revision, and claim wording ok"
