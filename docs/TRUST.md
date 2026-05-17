# Trust Dashboard

This dashboard is a concise adoption checklist for security-sensitive users.
It describes the current release posture and should be refreshed before each
stable release.

| Area | Current Status | Evidence |
| --- | --- | --- |
| License | `MIT OR Apache-2.0` | `Cargo.toml`, `LICENSE-MIT`, `LICENSE-APACHE` |
| MSRV | Rust `1.95.0` | `Cargo.toml`, `rust-toolchain.toml` |
| Runtime dependencies | Zero external crates | `scripts/validate-dependencies.sh` |
| Default dev dependencies | Zero external crates | `Cargo.toml` |
| Optional runtime features | `alloc`, `std`, `stream`; reserved `simd`, `tokio`, `kani`, `fuzzing` | `Cargo.toml`, `scripts/check_reserved_features.sh` |
| Unsafe policy | Scalar encode/decode remains safe Rust; audited unsafe is limited to volatile wiping and SIMD prototypes | `src/lib.rs`, `src/simd.rs`, `docs/UNSAFE.md` |
| Active backend | Scalar only | `runtime::backend_report()` tests |
| SIMD status | Reserved prototypes only; no accelerated backend admitted | `docs/SIMD.md` |
| Strict decoding | Default behavior rejects whitespace, mixed alphabets, malformed padding, and non-canonical trailing bits | integration tests |
| Legacy compatibility | Explicit opt-in APIs only | `decode_slice_legacy`, `validate_legacy` |
| Constant-time API | Constant-time-oriented scalar validation/decode and equal-length redacted-buffer comparison helpers exist with isolated dudect-style timing evidence; no formal cryptographic constant-time guarantee | `docs/CONSTANT_TIME.md`, `docs/DUDECT.md` |
| Cleanup posture | Clear-tail APIs, stream cleanup, `EncodedBuffer`, `DecodedBuffer`, and `SecretBuffer` provide best-effort cleanup; `SecretBuffer` also clears vector spare capacity when wrapping and dropping owned vectors | `SECURITY.md`, `docs/UNSAFE.md` |
| Fuzzing | Isolated `cargo-fuzz` harnesses outside the published dependency graph | `fuzz/`, `docs/RELEASE_EVIDENCE.md` |
| Miri | Release gate runs Miri when nightly Miri is installed and writes evidence artifacts | `scripts/check_miri.sh`, `target/release-evidence/miri/` |
| Kani | Harnesses are gated and run when installed/toolchain-compatible; incompatible Kani releases are explicit policy skips, not completed proofs | `scripts/check_kani.sh`, `docs/KANI.md` |
| Bounds invariants | Remaining internal indexing is grouped by documented local invariants | `docs/INVARIANTS.md` |
| Audit | RustSec check required | `cargo audit`, `scripts/checks.sh` |
| License policy | `cargo deny` and `cargo license --json` required | `deny.toml`, `scripts/checks.sh` |
| SBOM | SPDX and CycloneDX SBOM generation in release evidence | `scripts/generate-sbom.sh` |
| Reproducibility | Package/build reproducibility check in release gate | `scripts/stable_release_gate.sh` |

## Deployment Checks

High-assurance deployments should record `runtime::backend_report()` at process
startup and consider enforcing:

```rust
base64_ng::runtime::require_backend_policy(
    base64_ng::runtime::BackendPolicy::HighAssuranceScalarOnly,
)?;
```

Use this policy when deterministic scalar execution matters more than future
acceleration. It requires scalar execution, no detected SIMD candidate, the
`simd` feature disabled, no active accelerated backend, and the unsafe-boundary
check marked as enforced.

## Non-Claims

`base64-ng` currently does not claim:

- formally verified cryptographic constant-time behavior
- formal zeroization of all historical memory copies
- an active hardware-accelerated backend
- async/Tokio support
- serde or bytes integration

Those features remain admission-gated until their evidence is strong enough for
security-sensitive users.
