# base64-ng 2.1.0 (Unreleased)

Development checkpoint, not a release authorization.

Commit 1 synchronizes all Rust packages and the WASM loader at `2.1.0` with
publication blocked. It retains the signed `v2.0.4` compatibility baseline
and MSRV `1.90.0`. Commit 13 updates the active compiler to Rust `1.99.0`.

Through Commit 10, ordinary decoding has explicit validation policy, immutable
preflight, portable table validation and health-gated x86/NEON validation and
writing. Measured bulk gains and tiny-input tradeoffs are documented separately
from correctness evidence. CT/secret contracts are unchanged.

Commit 11 extends the ordinary strict route to WASM simd128. The npm SIMD
artifact and integrity pin are refreshed; the scalar artifact, JavaScript
options, ownership rules, limits and transactional destinations are unchanged.
Local Node, Wasmtime, Chromium and Firefox checks pass, as does the
operator-supplied Mac package/Safari rerun. External review and CI passed. See the
[WASM measurements](../docs/PERFORMANCE_2.1_WASM.md) for paired guest and loader
results, including small-input regressions and JIT limitations.

Commit 12 adds vector-length-agnostic RVV validation and shared ordinary decoding
for the already admitted exact Linux/X60 profile. Availability, secret/CT paths,
scalar diagnostics, transactional output, and health recovery remain unchanged.
See the [RVV measurements](../docs/PERFORMANCE_2.1_RVV.md) for native bulk gains
and small historical-call regressions. External retest and CI passed through
`0ef1dcb`.

Commit 13 accelerates canonical ordinary encoding through existing admitted
Standard/URL-safe kernels, including allocating/bounded and append/formatting
surfaces. Exact sizing, rollback, checked recovery, and short/custom fallback
remain in place. The [encoding checkpoint](../docs/PERFORMANCE_2.1_ENCODE.md)
records complete-call measurements and limitations. Both WASM artifacts are
rebuilt with Rust 1.99.0; JavaScript contracts and secret/CT algorithms are
unchanged. External review and CI passed at `d0a5f16`; local sandbox isolation
tests also passed after controller delegation was enabled.

Commit 14 retains private validation results across decode append reservation
and eligible historical owned decoding. Existing canonical allocating/bounded
routes already validate once. New wrapper tests cover allocation ordering,
limits, exact diagnostics, rollback and backend recovery. The
[routing audit](../docs/FORWARDING_2.1.md) keeps protocol grammar, streaming and
in-place work explicit and separate. External review and CI passed, including
the core-only allocation-test feature-gate follow-up at `51ac4b5`.

Commit 15 adds opt-in per-call execution reports and transactional static-token
policy methods. Reports distinguish requested validation from actual validator
and writer execution, including checked comparison and scalar recovery. Existing
methods, deployment checks and secret reporting are unchanged. See the
[composition checkpoint](../docs/DECODE_COMPOSITION_2.1.md). External review and
CI passed through `d3c84b2`, with a comment-only correction applied in Commit 16.
The follow-up retains alphabet
classification inside the proof, removes repeated writer/report classification
and stabilizes test-only backend initialization. Both WASM artifacts are rebuilt.
Local bulk timings recover parity within 5%; documented small-input tradeoffs
still require acceptance.

Commit 16 adds `Base64Ref`, binding immutable borrowed input to owned codec
settings and a checked decoded length. Auto can reuse grammar validation;
explicit ScalarReference revalidates each decode. Health changes, checked
output comparisons and scalar recovery remain enforced. `Base64String`
borrowing is fallible and validates rather than trusting encode policy.
See the [borrowed-view checkpoint](../docs/BORROWED_BASE64_2.1.md) for contracts,
tests and separate parse/reuse measurements. External review and CI passed at
`98cafa4`.

Commit 17 adds bulk interior processing to ordinary incremental states and
per-call `DecoderState::update_with_validation`. Accepted/produced counts,
absolute diagnostics, pending bytes, per-call transactionality, finish/reset
and checked recovery remain intact. No secret state or adapter implementation
is changed; existing adapters can benefit through the shared states, with
dedicated adapter integration and measurements reserved for Commit 18. See the
[incremental checkpoint](../docs/INCREMENTAL_BULK_2.1.md). External review and CI
are pending.

Remaining work is tracked in the [commit plan](../docs/2.1.0-release-plan.md).
Final release notes, acceptance, and hardware claims will be recorded before
publication is enabled; existing evidence is not a 2.1.0 performance claim.
