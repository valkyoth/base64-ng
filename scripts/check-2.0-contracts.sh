#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_CONTRACT_TOOLCHAIN:-}"
workdir="target/2_0_contract_compile"
mkdir -p "$workdir"

test -s docs/2.0_OPERATION_CONTRACTS.md
for required in \
    'Failure is absorbing.' \
    '`OutputFull` is a retryable status, not a failure.' \
    'original absolute byte index' \
    'never saturates different positions to' \
    '| One-shot caller slice |' \
    '| Allocating one-shot |' \
    '| `Vec`/`String` append |' \
    '| Formatter or writer |' \
    '| Incremental transform |' \
    '| In-place transform |' \
    '| Bounded secret frame |' \
    '| Unbounded stream |' \
    'global allocator,' \
    'Before a secret operation'
do
    if ! grep -F -q "$required" docs/2.0_OPERATION_CONTRACTS.md; then
        echo "2.0 contracts: documentation is missing: $required" >&2
        exit 1
    fi
done

run_cargo() {
    if [ -n "$toolchain" ]; then
        cargo +"$toolchain" "$@"
    else
        cargo "$@"
    fi
}

run_rustc() {
    if [ -n "$toolchain" ]; then
        rustup run "$toolchain" rustc "$@"
    else
        rustc "$@"
    fi
}

cat >"$workdir/model.rs" <<'RS'
#[path = "../../src/v2/contracts.rs"]
mod contracts;

pub use contracts::*;
RS

run_rustc \
    --edition=2024 \
    --crate-name base64_ng_v2_contract_model \
    --crate-type rlib \
    -A dead-code \
    "$workdir/model.rs" \
    --out-dir "$workdir"

cat >"$workdir/downstream.rs" <<'RS'
extern crate base64_ng_v2_contract_model as model;

use model::{BackendFault, InputErrorKind, Status};

fn status_id(status: Status) -> &'static str {
    match status {
        Status::NeedInput => "need-input",
        Status::OutputFull(_) => "output-full",
        Status::Complete => "complete",
        _ => "unknown",
    }
}

fn backend_id(fault: BackendFault) -> &'static str {
    match fault {
        BackendFault::SelfTestFailed => "backend-self-test-failed",
        BackendFault::OutputMismatch => "backend-output-mismatch",
        BackendFault::ImpossibleState => "backend-impossible-state",
        BackendFault::ScalarRetryFailed => "backend-scalar-retry-failed",
        _ => "unknown",
    }
}

fn main() {
    assert_eq!(status_id(Status::NeedInput), Status::NeedInput.as_str());
    assert_eq!(
        backend_id(BackendFault::SelfTestFailed),
        BackendFault::SelfTestFailed.as_str()
    );
    assert_eq!(InputErrorKind::TruncatedInput.as_str(), "truncated-input");
}
RS

run_rustc \
    --edition=2024 \
    "$workdir/downstream.rs" \
    --extern base64_ng_v2_contract_model="$workdir/libbase64_ng_v2_contract_model.rlib" \
    --out-dir "$workdir"
"$workdir/downstream"

cat >"$workdir/exhaustive.rs" <<'RS'
extern crate base64_ng_v2_contract_model as model;

fn classify(status: model::Status) -> u8 {
    match status {
        model::Status::NeedInput => 0,
        model::Status::OutputFull(_) => 1,
        model::Status::Complete => 2,
    }
}

fn main() {
    let _ = classify(model::Status::NeedInput);
}
RS

if run_rustc \
    --edition=2024 \
    "$workdir/exhaustive.rs" \
    --extern base64_ng_v2_contract_model="$workdir/libbase64_ng_v2_contract_model.rlib" \
    --out-dir "$workdir" >"$workdir/exhaustive.log" 2>&1
then
    echo "2.0 contracts: exhaustive downstream match unexpectedly compiled" >&2
    exit 1
fi
if ! grep -F -q 'non-exhaustive patterns' "$workdir/exhaustive.log"; then
    echo "2.0 contracts: exhaustive match failed for an unexpected reason" >&2
    cat "$workdir/exhaustive.log" >&2
    exit 1
fi

for enum in \
    Status InputErrorKind InputError BackendFault Failure TerminalError \
    OperationError BackendClass AssuranceClass ProtocolScope Atomicity
do
    if ! awk -v enum="$enum" '
        /#\[non_exhaustive\]/ { pending = 1; next }
        pending && $0 ~ "pub enum " enum "([ {]|$)" { found = 1; exit }
        pending && $0 !~ /^[[:space:]]*#/ && $0 !~ /^[[:space:]]*$/ { pending = 0 }
        END { exit(found ? 0 : 1) }
    ' src/v2/contracts.rs src/v2/contracts/reporting.rs; then
        echo "2.0 contracts: $enum is not non-exhaustive" >&2
        exit 1
    fi
done

if rg -n -F \
    -e unsafe \
    -e 'std::' \
    -e 'alloc::' \
    -e 'Box<' \
    -e '.unwrap(' \
    -e '.expect(' \
    -e 'panic!' \
    src/v2/contracts.rs src/v2/contracts/reporting.rs
then
    echo "2.0 contracts: model gained a forbidden dependency or panic site" >&2
    exit 1
fi

run_cargo test --lib 'v2::contract_tests'
run_cargo test --release --lib 'v2::contract_tests'
run_cargo clippy --lib --all-features -- -D warnings

echo "2.0 contracts: lifecycle, indexing, atomicity, and extension evidence ok"
