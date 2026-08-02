#!/usr/bin/env sh
set -eu

if ! rustc -vV | grep -q '^host: aarch64'; then
    echo "SVE hardware checks: this gate must run on a native AArch64 host" >&2
    exit 1
fi
if ! grep -E -q '(^|[[:space:]])sve([[:space:]]|$)' /proc/cpuinfo 2>/dev/null; then
    echo "SVE hardware checks: the native CPU does not report SVE" >&2
    exit 1
fi
if grep -E -i -q 'qemu|emulator' /proc/cpuinfo 2>/dev/null; then
    echo "SVE hardware checks: emulated CPUs are not accepted" >&2
    exit 1
fi
if command -v systemd-detect-virt >/dev/null 2>&1 && systemd-detect-virt --quiet; then
    echo "SVE hardware checks: virtualized execution is not accepted" >&2
    exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "SVE hardware checks: worktree must be clean" >&2
    exit 1
fi

commit="$(git rev-parse HEAD)"
evidence_dir="target/release-evidence/sve-hardware"
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
run env RUSTFLAGS=--cfg=base64_ng_sve_candidate cargo test --release --all-features --lib sve_ -- --nocapture
run scripts/generate_sve_asm_evidence.sh

cat "$transcript"
sha256="$(sha256sum "$transcript" | sed 's/ .*//')"
echo "SVE hardware checks: source_commit=$commit"
echo "SVE hardware checks: output_sha256=$sha256"
echo "SVE hardware checks: benchmark, signal, and ABI review remain separate report inputs"
echo "SVE hardware checks: ok"
