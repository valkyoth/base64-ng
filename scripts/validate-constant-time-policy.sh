#!/usr/bin/env sh
set -eu

test -s docs/CONSTANT_TIME.md

for required_phrase in \
    "does not currently claim a formally verified cryptographic" \
    "generated-code review" \
    "cargo rustc --release --lib --no-default-features -- --emit=asm" \
    "cargo rustc --release --lib --all-features -- --emit=asm" \
    "constant-time-oriented" \
    "transient" \
    "partial plaintext before the final wipe" \
    "sole MAC, bearer-token"
do
    if ! grep -qi -- "$required_phrase" docs/CONSTANT_TIME.md; then
        echo "constant-time policy: docs/CONSTANT_TIME.md missing required phrase: $required_phrase" >&2
        exit 1
    fi
done

if ! grep -q "concurrent or unsafe access to the output buffer during" docs/SECURITY_CONTROLS.md; then
    echo "constant-time policy: docs/SECURITY_CONTROLS.md must document the transient CT output-buffer observation window" >&2
    exit 1
fi

if ! grep -q "documented as a formally verified cryptographic constant-time API" README.md; then
    echo "constant-time policy: README.md must keep the non-claim wording" >&2
    exit 1
fi

if ! grep -q "not currently claim a formally verified cryptographic constant-time" SECURITY.md; then
    echo "constant-time policy: SECURITY.md must keep the non-claim wording" >&2
    exit 1
fi

for required_source_text in \
    "# Security" \
    "Do not use this method for token comparison, key-material" \
    "[\`crate::ct::STANDARD\`]" \
    "[\`crate::ct::URL_SAFE_NO_PAD\`]" \
    "The CT decoder exposes only clear-tail and stack-backed decode APIs." \
    "Do not use this helper as the sole MAC" \
    "#[must_use = \"handle decode errors; use crate::ct for secret-bearing payloads\"]"
do
    if ! grep -F -q "$required_source_text" src/lib.rs src/ct/*.rs src/profiles.rs src/buffers/*.rs; then
        echo "constant-time policy: source is missing decode security warning text: $required_source_text" >&2
        exit 1
    fi
done

ct_public_methods="$(
    sed -n '/^pub struct CtEngine/,/impl<A, const PAD: bool> core::fmt::Display for CtEngine/p' src/ct/mod.rs
)"

for removed_method in \
    "pub fn decode_slice(&self" \
    "pub fn decode_in_place<'a>(&self"
do
    if printf '%s\n' "$ct_public_methods" | grep -F -q "$removed_method"; then
        echo "constant-time policy: removed ct API is still public: $removed_method" >&2
        exit 1
    fi
done

echo "constant-time policy: ok"
