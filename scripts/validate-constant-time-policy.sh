#!/usr/bin/env sh
set -eu

test -s docs/CONSTANT_TIME.md

for required_phrase in \
    "does not currently claim a formally verified cryptographic" \
    "generated-code review" \
    "cargo rustc --release --lib --no-default-features -- --emit=asm" \
    "cargo rustc --release --lib --all-features -- --emit=asm" \
    "constant-time-oriented"
do
    if ! grep -qi -- "$required_phrase" docs/CONSTANT_TIME.md; then
        echo "constant-time policy: docs/CONSTANT_TIME.md missing required phrase: $required_phrase" >&2
        exit 1
    fi
done

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
    "#[must_use = \"handle decode errors; use crate::ct for secret-bearing payloads\"]"
do
    if ! grep -F -q "$required_source_text" src/lib.rs; then
        echo "constant-time policy: src/lib.rs is missing decode security warning text: $required_source_text" >&2
        exit 1
    fi
done

echo "constant-time policy: ok"
