#!/usr/bin/env sh
set -eu

audit_doc="docs/2.0_BIG_ENDIAN_AUDIT.md"

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q -- "$text" "$file"; then
        echo "big-endian byte-order audit: $file is missing required text: $text" >&2
        exit 1
    fi
}

test -s "$audit_doc"

for text in \
    "ordinary scalar encode and decode" \
    "Incremental state" \
    "One-shot and in-place transforms" \
    "Wrapping inserts or removes literal ASCII bytes" \
    "Secret encode and decode" \
    "Big-endian runtime reports must remain scalar active" \
    "QEMU proves functional behavior under emulation"
do
    require_text "$audit_doc" "$text"
done

portable_sources="
src/alphabet.rs
src/buffers
src/ct
src/decode_backend.rs
src/encode_backend.rs
src/engine
src/length.rs
src/profiles.rs
src/runtime
src/scalar.rs
src/scalar_encode_in_place.rs
src/stream
src/v2
src/wrap.rs
"

for source in $portable_sources; do
    if grep -R -n -E \
        'from_ne_bytes|to_ne_bytes|swap_bytes|transmute|read_unaligned|write_unaligned|cast::<[ui](16|32|64|128)>' \
        "$source"
    then
        echo "big-endian byte-order audit: native-width operation escaped an architecture module" >&2
        exit 1
    fi
done

for source in src/simd/neon.rs src/simd/mod.rs src/encode_backend.rs src/decode_backend.rs; do
    require_text "$source" 'target_endian = "little"'
done

if grep -R -n -E 'S390x|s390x|Powerpc64|powerpc64' \
    src/encode_backend.rs src/decode_backend.rs src/simd/mod.rs src/runtime
then
    echo "big-endian byte-order audit: an unreviewed big-endian backend entered dispatch" >&2
    exit 1
fi

echo "big-endian byte-order audit: portable byte equations and scalar dispatch ok"
