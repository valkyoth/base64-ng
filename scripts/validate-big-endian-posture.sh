#!/usr/bin/env sh
set -eu

review_doc="docs/BIG_ENDIAN_QEMU_REVIEW.md"
simd_doc="docs/SIMD.md"
admission_doc="docs/SIMD_ADMISSION.md"
qemu_script="scripts/check_big_endian_qemu.sh"
intrinsics_script="scripts/check_big_endian_intrinsics_status.sh"

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

require_text "$review_doc" "QEMU-tested scalar/fallback targets"
require_text "$review_doc" "not accepted for:"
require_text "$review_doc" "real hardware performance claims"
require_text "$review_doc" "stdarch_s390x"
require_text "$review_doc" "stdarch_powerpc"
require_text "$review_doc" "big-endian runtime reports must remain scalar active"
require_text "$review_doc" "QEMU-tested until real hardware evidence is linked"
require_text "$simd_doc" "Big-endian and RISC-V acceleration work is tracked as a QEMU-first evidence"
require_text "$simd_doc" "documented as QEMU-tested and community-hardware evidence requested"
require_text "$admission_doc" "big-endian AArch64, CT secret decode, and \`no_std\` remain scalar or"
require_text "$qemu_script" "not_evidence_for=real hardware performance, timing, microarchitectural behavior, or side-channel behavior"
require_text "$qemu_script" "hardware_status=community real-hardware reports requested for s390x, powerpc64, and big-endian AArch64"
require_text "$intrinsics_script" "stdarch_s390x"
require_text "$intrinsics_script" "stdarch_powerpc"

"$intrinsics_script"

if grep -R -E 'S390x|s390x|Powerpc64|powerpc64' src/encode_backend.rs src/decode_backend.rs src/simd/mod.rs src/runtime; then
    echo "big-endian posture: source contains big-endian backend names before active admission review" >&2
    exit 1
fi

reject_text "$admission_doc" "| s390x | admitted backend |"
reject_text "$admission_doc" "| PowerPC64 | admitted backend |"

echo "big-endian posture: QEMU-only scalar/fallback posture ok"
