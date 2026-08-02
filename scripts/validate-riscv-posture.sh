#!/usr/bin/env sh
set -eu

review="docs/RISCV_QEMU_REVIEW.md"
rvv="src/simd/rvv.rs"
simd="src/simd/mod.rs"
encode="src/encode_backend.rs"
decode="src/decode_backend.rs"

require_text() {
    if ! grep -F -q -- "$2" "$1"; then
        echo "RISC-V posture: $1 is missing required text: $2" >&2
        exit 1
    fi
}

for file in "$review" "$rvv" "$simd" "$encode" "$decode"; do
    test -s "$file"
done
test -x scripts/check_riscv_qemu.sh
test -x scripts/check_riscv_hardware.sh
test -x scripts/generate_rvv_asm_evidence.sh

require_text "$review" "non-admitted RVV 1.0 candidate"
require_text "$review" "VLEN 128 and VLEN 256"
require_text "$review" "real hardware evidence remains mandatory"
require_text "$review" "normal published builds remain scalar"
require_text "$review" "riscv_hwprobe"
require_text "$review" "PR_RISCV_V_GET_CONTROL"
require_text "$rvv" '.attribute arch, "rv64gcv"'
require_text "$rvv" "base64_ng_rvv_encode_standard_12"
require_text "$rvv" "base64_ng_rvv_decode_standard_16"
require_text "$rvv" "vmv.v.i v15, 0"
require_text "$simd" "base64_ng_rvv_candidate"
require_text "$simd" "return Candidate::Rvv;"
require_text "$encode" "EncodeBackend::Scalar"
require_text "$decode" "DecodeBackend::Scalar"

if grep -E -q 'EncodeBackend::Rvv|DecodeBackend::Rvv|ActiveBackend::Rvv' "$encode" "$decode" "$simd"; then
    echo "RISC-V posture: RVV entered production dispatch before hardware admission" >&2
    exit 1
fi

scripts/check_riscv_intrinsics_status.sh
scripts/validate-riscv-hardware-evidence.py --schema-only
scripts/test-riscv-hardware-evidence.py

echo "RISC-V posture: QEMU candidate present and production dispatch remains scalar"
