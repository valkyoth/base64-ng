# Security Policy

`base64-ng` is infrastructure code. Security reports are treated as correctness reports, even when the issue is not directly exploitable.

## Supported Versions

Only the latest released minor line receives security fixes before `1.0`.

## Reporting

Please report suspected vulnerabilities privately to the maintainers. Do not open public issues for memory safety bugs, out-of-bounds behavior, data-dependent behavior in documented constant-time paths, or supply-chain compromise.

Include:

- Affected version or commit.
- Reproducer or input corpus.
- Target architecture and CPU features.
- Whether default, `no_std`, `simd`, or future streaming features are involved.

## Security Bar

Required for release:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo test --no-default-features`
- `cargo +nightly miri test --no-default-features` when nightly Miri is installed
- `cargo deny check`
- `cargo audit`
- `cargo license --json`
- SBOM generation
- Reproducible build check

The local release gate skips only the large deterministic sweep tests under
Miri. Those tests still run in the normal stable test suite; Miri focuses on the
scalar and in-place safety surface that benefits most from interpreter checks.

The scalar library root uses `#![deny(unsafe_code)]`. Unsafe code is allowed
only inside the dedicated private SIMD admission boundary in `src/simd.rs`, and
the release gate verifies that `allow(unsafe_code)` does not appear elsewhere.
The reserved `simd` feature may detect CPU candidates, but it does not activate
an accelerated backend until the SIMD admission policy is satisfied.
`docs/UNSAFE.md` inventories every current unsafe site and its safety
invariants.

Security-sensitive deployments can call `runtime::backend_report()` to record
the active backend, detected candidate, SIMD feature status, unsafe-boundary
status, and current security posture. `runtime::require_backend_policy()` can
enforce scalar-only execution, no-SIMD builds, no detected SIMD candidate, or
the combined `HighAssuranceScalarOnly` policy at process startup.

The scalar encoder avoids input-derived alphabet table indexes, and the scalar
decoder avoids obvious alphabet `match` ladders by using branch-minimized
arithmetic for ASCII classification. The `ct` module provides a separate
constant-time-oriented scalar decode path that avoids secret-indexed lookup
tables while mapping Base64 symbols. Its malformed-content errors are
intentionally opaque and non-localized so error tracking does not reveal the
first malformed byte position or the malformed-content category. Invalid
length, output-buffer capacity, final success/failure, and decoded length are
public API results. Its clear-tail variants clear caller-owned output on error
so rejected sensitive payloads do not leave partially decoded bytes in that
buffer. This reduces easy timing and retention pitfalls, but `base64-ng` does
not currently claim a formally verified cryptographic constant-time encode or
decode API.

The clear-tail encode and decode APIs provide best-effort cleanup for
caller-owned buffers by writing zero bytes over unused tail bytes on success and
over the whole buffer on encode/decode error. Because the scalar crate forbids
unsafe code and has no runtime dependencies, this cleanup uses ordinary Rust
writes plus a compiler fence, not volatile writes or a formally verified
zeroization primitive. Treat these APIs as buffer-retention reduction, not as a
complete secret-erasure guarantee against compiler optimizations, core dumps,
swap, hardware observation, or other process memory disclosure bugs.

The `SecretBuffer` owned wrapper is available with the `alloc` feature for
sensitive encoded or decoded bytes that should not be accidentally logged. It
redacts `Debug` and `Display`, requires explicit reveal methods, and clears
initialized bytes on drop with the same best-effort cleanup helper. It cannot
clean historical copies outside the wrapper or allocator spare capacity.

Streaming wrappers apply best-effort cleanup to their small internal staging
buffers. Encoders clear pending plaintext bytes when those bytes are consumed
and again when the wrapper is dropped. Decoders clear pending Base64 input when
it is consumed or when the wrapper is dropped. `DecoderReader` and
`EncoderReader` use fixed-size internal output queues instead of allocator
backed queues, clear queue slots as bytes are consumed, and clear the full
queue capacity on drop. This is retention reduction for small internal buffers,
not a formal zeroization guarantee.

Public encoded-length helpers report overflow with `Result` or `Option` rather
than panicking. Code that handles untrusted length metadata should use these
helpers before allocating or accepting framed payloads.

Runtime scalar APIs are expected to return `Result` or `Option` for malformed
input and size errors instead of unwinding. Compile-time array encoding is the
exception: it intentionally fails const evaluation when the caller supplies an
incorrect output array length.
`scripts/validate-panic-policy.sh` release-gates new non-test panic-like sites
and requires reviewed exceptions to remain documented in `docs/PANIC_POLICY.md`.

Bounded-memory users should prefer `checked_encoded_len`, `decoded_capacity`,
`decode_slice`, and `decode_in_place` so allocation limits are chosen by the
caller. The `alloc` helper `decode_vec` validates input before allocating the
decoded buffer.

Required before unsafe SIMD stabilizes:

- `allow(unsafe_code)` remains confined to `src/simd.rs`.
- Every unsafe block has a local safety explanation.
- Scalar/SIMD differential tests.
- Fuzz targets covering strict and legacy modes.
- Miri on scalar, in-place, and SIMD dispatch APIs.
- Kani proofs for in-place bounds invariants.
- Architecture-specific CI or documented local evidence.

See `docs/SIMD.md` for the full SIMD admission policy.
See `docs/TRUST.md` and `docs/SECURITY_CONTROLS.md` for adoption-focused
trust and CWE/security-control mapping.
