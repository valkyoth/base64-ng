#!/usr/bin/env sh
set -eu

document="docs/2.0_ASSURANCE_AND_PROTECTED_MEMORY.md"
workdir="target/2_0_assurance"
mkdir -p "$workdir/src"

for required in \
    'A token alone never authorizes an ordinary `&mut [u8]`' \
    '`!Sync`, `!UnwindSafe`, and' \
    'allocation-free' \
    'Indeterminate disposal is never retryable' \
    'Base 2.0 ships no persistent teardown provider'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 assurance: documentation is missing: $required" >&2
        exit 1
    fi
done

cat >"$workdir/Cargo.toml" <<'TOML'
[package]
name = "base64-ng-2-0-assurance-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
base64-ng = { path = "../..", default-features = false, features = ["alloc", "secrets"] }
TOML

expect_compile_failure() {
    label="$1"
    toolchain="$2"
    log="$workdir/$label-$toolchain.log"
    if [ "$toolchain" = "current" ]; then
        if cargo check --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
            echo "2.0 assurance: $label unexpectedly compiled on current Rust" >&2
            exit 1
        fi
    elif cargo +"$toolchain" check --quiet --offline --manifest-path "$workdir/Cargo.toml" >"$log" 2>&1; then
        echo "2.0 assurance: $label unexpectedly compiled on Rust $toolchain" >&2
        exit 1
    fi
}

write_token_clone_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::AssuranceContext;

fn main() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let _duplicate = token.clone();
}
RS
}

write_token_copy_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::AssuranceContext;

fn require_copy<T: Copy>(_: T) {}

fn main() {
    let context = AssuranceContext::new();
    require_copy(context.best_effort_token());
}
RS
}

write_private_state_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::Uninitialized;

fn main() {
    let _state = Uninitialized { _private: () };
}
RS
}

write_provider_access_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::ProviderAccess;

fn main() {
    let _access = ProviderAccess { _private: () };
}
RS
}

write_ordinary_slice_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::{STRICT_STANDARD_PADDED, assurance::AssuranceContext, secret::SecretInput};

fn main() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let mut ordinary = [0_u8; 8];
    let _ = STRICT_STANDARD_PADDED.decode_assured(
        &token,
        &mut ordinary,
        &SecretInput::new(b"c2VjcmV0"),
    );
}
RS
}

write_token_sync_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::AssuranceContext;

fn require_sync<T: Sync>(_: &T) {}

fn main() {
    let context = AssuranceContext::new();
    require_sync(&context.best_effort_token());
}
RS
}

write_protected_send_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::{
    AssuranceContext, BestEffortProvider, ProtectedSecret, ProviderLimits,
};

fn require_send<T: Send>(_: T) {}

fn main() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<1>::new(ProviderLimits {
        max_identities: 1,
        max_logical_bytes: 8,
        max_effective_pages: 2,
        max_registry_entries: 1,
        max_retry_attempts: 1,
        max_maintenance_work: 1,
        page_size: 8,
    }).unwrap();
    require_send(ProtectedSecret::try_new(&provider, &token, 8).unwrap());
}
RS
}

write_protected_unwind_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::{
    AssuranceContext, BestEffortProvider, ProtectedSecret, ProviderLimits,
};
use core::panic::UnwindSafe;

fn require_unwind<T: UnwindSafe>(_: T) {}

fn main() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<1>::new(ProviderLimits {
        max_identities: 1,
        max_logical_bytes: 8,
        max_effective_pages: 2,
        max_registry_entries: 1,
        max_retry_attempts: 1,
        max_maintenance_work: 1,
        page_size: 8,
    }).unwrap();
    require_unwind(ProtectedSecret::try_new(&provider, &token, 8).unwrap());
}
RS
}

write_protected_ref_unwind_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::{
    AssuranceContext, BestEffortProvider, ProtectedSecret, ProviderLimits,
};
use core::panic::RefUnwindSafe;

fn require_ref_unwind<T: RefUnwindSafe>(_: &T) {}

fn main() {
    let context = AssuranceContext::new();
    let token = context.best_effort_token();
    let provider = BestEffortProvider::<1>::new(ProviderLimits {
        max_identities: 1,
        max_logical_bytes: 8,
        max_effective_pages: 2,
        max_registry_entries: 1,
        max_retry_attempts: 1,
        max_maintenance_work: 1,
        page_size: 8,
    }).unwrap();
    let allocation = ProtectedSecret::try_new(&provider, &token, 8).unwrap();
    require_ref_unwind(&allocation);
}
RS
}

write_handle_send_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::BestEffortHandle;

fn require_send<T: Send>() {}

fn main() {
    require_send::<BestEffortHandle>();
}
RS
}

write_handle_sync_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::BestEffortHandle;

fn require_sync<T: Sync>() {}

fn main() {
    require_sync::<BestEffortHandle>();
}
RS
}

write_handle_unwind_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::BestEffortHandle;
use core::panic::UnwindSafe;

fn require_unwind<T: UnwindSafe>() {}

fn main() {
    require_unwind::<BestEffortHandle>();
}
RS
}

write_handle_ref_unwind_case() {
    cat >"$workdir/src/main.rs" <<'RS'
use base64_ng::assurance::BestEffortHandle;
use core::panic::RefUnwindSafe;

fn require_ref_unwind<T: RefUnwindSafe>() {}

fn main() {
    require_ref_unwind::<BestEffortHandle>();
}
RS
}

run_compile_fail_matrix() {
    toolchain="$1"
    for case_name in \
        token_clone \
        token_copy \
        private_state \
        provider_access \
        ordinary_slice \
        token_sync \
        protected_send \
        protected_unwind \
        protected_ref_unwind \
        handle_send \
        handle_sync \
        handle_unwind \
        handle_ref_unwind
    do
        "write_${case_name}_case"
        expect_compile_failure "$case_name" "$toolchain"
    done
}

echo "2.0 assurance: portable feature matrix"
cargo check --no-default-features --features secrets --lib
cargo check --all-features --lib

echo "2.0 assurance: operation and fault-injection tests"
cargo test --all-features --lib 'v2::assurance'
cargo test --all-features --test v2_assurance

echo "2.0 assurance: attested operation"
RUSTFLAGS='--cfg base64_ng_require_high_assurance' \
    cargo test --all-features --test v2_assurance

echo "2.0 assurance: current-toolchain compile-fail matrix"
run_compile_fail_matrix current

if rustup run 1.90.0 rustc --version >/dev/null 2>&1; then
    echo "2.0 assurance: MSRV compile-fail matrix"
    run_compile_fail_matrix 1.90.0
else
    echo "2.0 assurance: skipping local MSRV compile-fail matrix; Rust 1.90.0 is not installed"
fi

echo "2.0 assurance: build and unsafe policy"
scripts/check_high_assurance_policy.sh
scripts/validate-unsafe-boundary.sh

echo "2.0 assurance: tokens, protected typestates, teardown, and attestation ok"
