#!/usr/bin/env sh
set -eu

review_doc="docs/BIG_ENDIAN_QEMU_REVIEW.md"
simd_doc="docs/SIMD.md"
admission_doc="docs/SIMD_ADMISSION.md"
qemu_script="scripts/check_big_endian_qemu.sh"
intrinsics_script="scripts/check_big_endian_intrinsics_status.sh"
byte_order_script="scripts/validate-big-endian-byte-order.sh"
hardware_validator="scripts/validate-big-endian-hardware-evidence.py"
hardware_validator_tests="scripts/test-big-endian-hardware-evidence.py"
hardware_schema="hardware-evidence/big-endian/schema-v1.json"

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q -- "$text" "$file"; then
        echo "big-endian posture: $file is missing required text: $text" >&2
        exit 1
    fi
}

reject_text() {
    file="$1"
    text="$2"
    if grep -F -q -- "$text" "$file"; then
        echo "big-endian posture: $file contains rejected text: $text" >&2
        exit 1
    fi
}

test -s "$review_doc"
test -x "$qemu_script"
test -x "$intrinsics_script"
test -x "$byte_order_script"
test -x "$hardware_validator"
test -x "$hardware_validator_tests"
test -s "$hardware_schema"

require_text "$review_doc" "QEMU-tested scalar/fallback targets"
require_text "$review_doc" "not accepted for:"
require_text "$review_doc" "real hardware performance claims"
require_text "$review_doc" "stdarch_s390x"
require_text "$review_doc" "stdarch_powerpc"
require_text "$review_doc" "big-endian runtime reports must remain scalar active"
require_text "$review_doc" "Reports remain QEMU-tested until real hardware evidence is linked"
require_text "$review_doc" "scripts/check_big_endian_qemu.sh --all"
require_text "$review_doc" "hardware-evidence/big-endian/schema-v1.json"
require_text "$simd_doc" "Big-endian and RISC-V acceleration work follows a QEMU-first evidence path"
require_text "$simd_doc" "published builds remain scalar on RISC-V"
require_text "$admission_doc" "big-endian AArch64, and CT"
require_text "$admission_doc" "secret decode remain scalar or"
require_text "$qemu_script" "not_evidence_for=real hardware performance, timing, microarchitectural behavior, register retention, physical cleanup, or side-channel behavior"
require_text "$qemu_script" "covered_surfaces=default,all-features,no-default-features,RFC4648,malformed,incremental,stream,in-place,wrapping,secret-cleanup,backend-reporting,doctests"
require_text "$qemu_script" "hardware_status=community real-hardware reports required before accelerated big-endian admission"
require_text "$intrinsics_script" "stdarch_s390x"
require_text "$intrinsics_script" "stdarch_powerpc"

"$intrinsics_script"
"$byte_order_script"
"$hardware_validator" --schema-only
"$hardware_validator_tests"

if grep -R -E 'S390x|s390x|Powerpc64|powerpc64' src/encode_backend.rs src/decode_backend.rs src/simd/mod.rs src/runtime; then
    echo "big-endian posture: source contains big-endian backend names before active admission review" >&2
    exit 1
fi

reject_text "$admission_doc" "| s390x | admitted backend |"
reject_text "$admission_doc" "| PowerPC64 | admitted backend |"

echo "big-endian posture: QEMU-only scalar/fallback posture ok"
