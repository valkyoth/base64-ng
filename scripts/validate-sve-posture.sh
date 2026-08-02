#!/usr/bin/env sh
set -eu

for required_file in \
    src/simd/sve.rs \
    src/simd/sve_tests.rs \
    docs/SVE_QEMU_REVIEW.md \
    hardware-evidence/sve/README.md \
    hardware-evidence/sve/schema-v1.json \
    scripts/check_sve_qemu.sh \
    scripts/check_sve_hardware.sh \
    scripts/generate_sve_asm_evidence.sh \
    scripts/validate-sve-hardware-evidence.py \
    scripts/test-sve-hardware-evidence.py
do
    if [ ! -f "$required_file" ]; then
        echo "SVE posture: missing $required_file" >&2
        exit 1
    fi
done

for required_text in \
    "complete, non-admitted SVE candidate" \
    "128, 256, and 512 bits" \
    "two real SVE systems" \
    "normal published builds continue to use admitted NEON or scalar" \
    "PR_SVE_GET_VL" \
    "fallback suites run with SVE disabled" \
    "QEMU harnesses are serialized" \
    "not thread-safety evidence"
do
    if ! grep -F -q "$required_text" docs/SVE_QEMU_REVIEW.md; then
        echo "SVE posture: docs/SVE_QEMU_REVIEW.md is missing required text: $required_text" >&2
        exit 1
    fi
done

if grep -R -E -n 'ActiveBackend::Sve|EncodeBackend::Sve|DecodeBackend::Sve' \
    src/encode_backend.rs src/decode_backend.rs src/simd/mod.rs
then
    echo "SVE posture: candidate must not enter production dispatch" >&2
    exit 1
fi

scripts/validate-sve-hardware-evidence.py --schema-only
scripts/test-sve-hardware-evidence.py
echo "SVE posture: QEMU candidate present and production dispatch remains NEON/scalar"
