#!/usr/bin/env sh
set -eu

review_doc="docs/RISCV_QEMU_REVIEW.md"
simd_doc="docs/SIMD.md"
admission_doc="docs/SIMD_ADMISSION.md"
qemu_script="scripts/check_riscv_qemu.sh"
intrinsics_script="scripts/check_riscv_intrinsics_status.sh"

require_text() {
    file="$1"
    text="$2"
    if ! grep -F -q -- "$text" "$file"; then
        echo "RISC-V posture: $file is missing required text: $text" >&2
        exit 1
    fi
}

reject_text() {
    file="$1"
    text="$2"
    if grep -F -q -- "$text" "$file"; then
        echo "RISC-V posture: $file contains rejected text: $text" >&2
        exit 1
    fi
}

test -s "$review_doc"
test -x "$qemu_script"
test -x "$intrinsics_script"

require_text "$review_doc" "QEMU-tested scalar/fallback target"
require_text "$review_doc" "not accepted for:"
require_text "$review_doc" "real RVV hardware performance claims"
require_text "$review_doc" "riscv_ext_intrinsics"
require_text "$review_doc" "RISC-V runtime reports must remain scalar active"
require_text "$review_doc" "QEMU-tested until real RVV hardware evidence is linked"
require_text "$review_doc" "scheduled for \`1.3.10\`"
require_text "$review_doc" "The \`1.3.9\` sanitization-companion migration does not"
require_text "$simd_doc" "Big-endian and RISC-V acceleration work is tracked as a QEMU-first evidence"
require_text "$simd_doc" "documented as QEMU-tested and community-hardware evidence requested"
require_text "$simd_doc" "RVV proof and admission review is scheduled for"
require_text "$simd_doc" "\`1.3.10\`; \`1.3.9\` does not admit a RISC-V backend."
require_text "$admission_doc" "RISC-V acceleration remains scalar/fallback-only under QEMU evidence"
require_text "$qemu_script" "not_evidence_for=real RVV hardware performance, timing, microarchitectural behavior, or side-channel behavior"
require_text "$qemu_script" "hardware_status=community real-hardware reports requested for RVV 1.0 RISC-V systems"
require_text "$intrinsics_script" "riscv_ext_intrinsics"

"$intrinsics_script"

if grep -R -E 'Rvv|RVV|riscv64.*admitted|Riscv64' src/encode_backend.rs src/decode_backend.rs src/simd/mod.rs src/runtime; then
    echo "RISC-V posture: source contains RISC-V backend admission names before active RVV review" >&2
    exit 1
fi

reject_text "$admission_doc" "| RVV | admitted backend |"
reject_text "$admission_doc" "| RISC-V | admitted backend |"

echo "RISC-V posture: QEMU-only scalar/fallback posture ok"
