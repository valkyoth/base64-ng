#!/usr/bin/env sh
set -eu

if ! rustc -vV | grep -q '^host: riscv64'; then
    echo "RISC-V hardware checks: this gate must run on a native riscv64 host" >&2
    exit 1
fi
if grep -E -i -q 'qemu|emulator' /proc/cpuinfo 2>/dev/null; then
    echo "RISC-V hardware checks: emulated CPUs are not accepted" >&2
    exit 1
fi
if command -v systemd-detect-virt >/dev/null 2>&1 && systemd-detect-virt --quiet; then
    echo "RISC-V hardware checks: virtualized execution is not accepted" >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "RISC-V hardware checks: worktree must be clean" >&2
    exit 1
fi

commit="$(git rev-parse HEAD)"
if ! printf '%s\n' "$commit" | grep -E -q '^[0-9a-f]{40}$'; then
    echo "RISC-V hardware checks: HEAD is not a full Git object id" >&2
    exit 1
fi

evidence_dir="target/release-evidence/riscv-hardware"
transcript="$evidence_dir/output.txt"
mkdir -p "$evidence_dir"
: >"$transcript"

run() {
    printf '%s\n' "+ $*" >>"$transcript"
    if "$@" >>"$transcript" 2>&1; then
        return
    fi
    cat "$transcript" >&2
    exit 1
}

run rustc -Vv
run cargo -V
run uname -a
run sh -c 'cat /proc/cpuinfo'
run cargo test --all-targets --all-features
run cargo test --doc --all-features
run env RUSTFLAGS=--cfg=base64_ng_rvv_candidate cargo test --release --all-features --lib rvv_ -- --nocapture
run env RUSTFLAGS=--cfg=base64_ng_rvv_candidate cargo test --release --all-features --lib \
    simd::rvv_tests::rvv_state_survives_linux_signal_delivery -- \
    --ignored --exact --nocapture
run scripts/generate_rvv_asm_evidence.sh

cat "$transcript"
sha256="$(sha256sum "$transcript" | sed 's/ .*//')"
echo "RISC-V hardware checks: source_commit=$commit"
echo "RISC-V hardware checks: output_sha256=$sha256"
echo "RISC-V hardware checks: benchmark and ABI review remain separate report inputs"
echo "RISC-V hardware checks: ok"
