# base64-ng 2.1.0 (Unreleased)

Development checkpoint, not a release authorization.

Commit 1 synchronizes all Rust packages and the WASM loader at `2.1.0` with
publication blocked. It retains the signed `v2.0.4` compatibility baseline,
Rust `1.98.1`, and MSRV `1.90.0`.

Through Commit 10, ordinary decoding has explicit validation policy, immutable
preflight, portable table validation and health-gated x86/NEON validation and
writing. Measured bulk gains and tiny-input tradeoffs are documented separately
from correctness evidence. CT/secret contracts are unchanged.

Commit 11 extends the ordinary strict route to WASM simd128. The npm SIMD
artifact and integrity pin are refreshed; the scalar artifact, JavaScript
options, ownership rules, limits and transactional destinations are unchanged.
Local Node, Wasmtime, Chromium and Firefox checks pass. Safari operator
execution and external review remain pending. See the
[WASM measurements](../docs/PERFORMANCE_2.1_WASM.md) for paired guest and loader
results, including small-input regressions and JIT limitations.

Remaining work is tracked in the [commit plan](../docs/2.1.0-release-plan.md).
Final release notes, acceptance, and hardware claims will be recorded before
publication is enabled; existing evidence is not a 2.1.0 performance claim.
