#!/usr/bin/env sh
set -eu

cargo_version="$(
    sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | sed -n '1p'
)"

test -s docs/API_AUDIT.md

for required_text in \
    "# Public API Audit" \
    "## Audit Rules" \
    "## Status Legend" \
    "## Public Surface Under Review" \
    "## Audit Decisions" \
    "### Profiles" \
    "### Validation-Only APIs" \
    "### Stack-Backed Buffers" \
    "### Secret Buffer" \
    "### In-Place APIs" \
    "### Custom Alphabets" \
    "### Stream Module" \
    "### Error Types" \
    "## \`v1.0\` Admission Questions" \
    "## Initial \`v0.10\` Direction" \
    "Do not add broad conversion traits" \
    "Do not admit optional ecosystem dependencies" \
    "Do not admit active SIMD dispatch" \
    "review pending" \
    "Profile::checked_new" \
    "Keep validation/decode agreement release-tested" \
    "into_exposed_array" \
    "ExposedSecretVec" \
    "into_exposed_unprotected_vec_caller_must_zeroize" \
    "try_into_exposed_string" \
    "Encode-to-back and decode-to-front contracts" \
    "define_alphabet!" \
    "The \`ct\` module scans" \
    "\`Alphabet::ENCODE\` directly" \
    "Keep the custom-alphabet timing contract documented" \
    "Padded \`DecoderReader\` stops after terminal padding" \
    "Keep ct malformed-content errors opaque"
do
    if ! grep -F -q "$required_text" docs/API_AUDIT.md; then
        echo "api audit: docs/API_AUDIT.md is missing required text: $required_text" >&2
        exit 1
    fi
done

for required_area in \
    "Engine constants and \`Engine<A, PAD>\`" \
    "\`Profile<A, PAD>\` and named profiles" \
    "Length helpers" \
    "Slice encode/decode APIs" \
    "In-place APIs" \
    "Validation-only APIs" \
    "Stack-backed buffers" \
    "\`SecretBuffer\`" \
    "\`ct\` module" \
    "\`stream\` module" \
    "Runtime backend reporting" \
    "Feature flags" \
    "Error types" \
    "Macros and custom alphabets"
do
    if ! grep -F -q "| $required_area |" docs/API_AUDIT.md; then
        echo "api audit: docs/API_AUDIT.md is missing public surface row: $required_area" >&2
        exit 1
    fi
done

case "$cargo_version" in
    *-*)
        ;;
    *)
        if grep -F -q "| review pending |" docs/API_AUDIT.md; then
            echo "api audit: stable releases must not leave public API rows as review pending" >&2
            exit 1
        fi
        ;;
esac

echo "api audit: ok"
