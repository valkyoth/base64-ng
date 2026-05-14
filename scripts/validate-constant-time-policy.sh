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

echo "constant-time policy: ok"
