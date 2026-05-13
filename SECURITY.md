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

The scalar encoder avoids input-derived alphabet table indexes, and the scalar
decoder avoids obvious alphabet `match` ladders by using branch-minimized
arithmetic for ASCII classification. This reduces easy timing pitfalls, but
`base64-ng` does not currently claim a formally verified cryptographic
constant-time encode or decode API.

The clear-tail encode and decode APIs provide best-effort cleanup for
caller-owned buffers by writing zero bytes over unused tail bytes on success and
over the whole buffer on encode/decode error. Because the scalar crate forbids
unsafe code and has no runtime dependencies, this cleanup uses ordinary Rust
writes, not volatile writes or a formally verified zeroization primitive. Treat
these APIs as buffer-retention reduction, not as a complete secret-erasure
guarantee against compiler optimizations, core dumps, swap, hardware
observation, or other process memory disclosure bugs.

Public encoded-length helpers report overflow with `Result` or `Option` rather
than panicking. Code that handles untrusted length metadata should use these
helpers before allocating or accepting framed payloads.

Runtime scalar APIs are expected to return `Result` or `Option` for malformed
input and size errors instead of unwinding. Compile-time array encoding is the
exception: it intentionally fails const evaluation when the caller supplies an
incorrect output array length.

Bounded-memory users should prefer `checked_encoded_len`, `decoded_capacity`,
`decode_slice`, and `decode_in_place` so allocation limits are chosen by the
caller. The `alloc` helper `decode_vec` validates input before allocating the
decoded buffer.

Required before unsafe SIMD stabilizes:

- Scalar/SIMD differential tests.
- Fuzz targets covering strict and legacy modes.
- Miri on scalar, in-place, and SIMD dispatch APIs.
- Kani proofs for in-place bounds invariants.
- Architecture-specific CI or documented local evidence.
