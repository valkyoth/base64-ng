#!/usr/bin/env sh
set -eu

document="docs/2.0_OPERATION_REPORTING.md"
for required in \
    'encode_backend' \
    'strict_decode_backend' \
    'secret_decode_backend' \
    'capability-oriented reports' \
    'AVX-512 from 192 bytes' \
    'scalar-constant-time-oriented' \
    'wasm-simd128-artifact' \
    'wasm-host-runtime-unidentified' \
    'WipedBestEffort' \
    'WipedAttested' \
    'unknown-no-address-retained' \
    'no pointer' \
    'pending teardown stage and journal substage'
do
    if ! grep -F -q "$required" "$document"; then
        echo "2.0 operation reporting: documentation is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub encode_backend: OperationBackendReport' \
    'pub strict_decode_backend: OperationBackendReport' \
    'pub secret_decode_backend: OperationBackendReport' \
    'OperationBackendReport::secret_decode()' \
    'stably_scalar(self.encode_backend)' \
    'wasm_artifact_posture: wasm_artifact_posture()'
do
    if ! grep -F -q "$required" src/runtime/report.rs; then
        echo "2.0 operation reporting: runtime report is missing: $required" >&2
        exit 1
    fi
done

for required in \
    'pub fn operation_report(' \
    'PhysicalProtection::ProtectionUnknown' \
    'ProtectedOperationReport::live(' \
    'self.provider.physical_protection'
do
    if ! grep -F -q "$required" src/v2/assurance/protected.rs; then
        echo "2.0 operation reporting: exact allocation report is missing: $required" >&2
        exit 1
    fi
done

echo "2.0 operation reporting: ordinary backend correlation"
cargo test --all-features --lib 'runtime::tests'
cargo test --all-features --test rfc4648 'runtime_backend'

echo "2.0 operation reporting: allocation and teardown snapshots"
cargo test --all-features --lib 'v2::assurance::tests'
cargo test --all-features --test v2_assurance

echo "2.0 operation reporting: high-assurance snapshots"
RUSTFLAGS='--cfg base64_ng_require_high_assurance' \
    cargo test --all-features --test v2_assurance

echo "2.0 operation reporting: portable feature matrix"
cargo check --no-default-features --lib
cargo check --no-default-features --features secrets --lib
cargo check --all-features --all-targets

scripts/validate-panic-policy.sh
scripts/validate-unsafe-boundary.sh

echo "2.0 operation reporting: per-operation and false-assurance evidence ok"
