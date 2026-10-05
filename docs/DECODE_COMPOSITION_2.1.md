# Decode Composition Checkpoint

Commit 15 adds ordinary per-call execution reports. This is a development
checkpoint, not release admission; independent retest and CI remain pending.
No new ISA, dependency, secret API, or unsafe boundary is introduced.

## Public Surface

`Base64::decode_into_with_report(input, output, validation)` returns
`Result<(usize, DecodeReport), OneShotError>`. Validation and capacity checks
precede writes; every returned error leaves the complete destination unchanged.
Reports are returned only on success, not as mutable global last-call state.

`DecodeReport` exposes:

- `requested_validation()`: the caller's `Auto` or `ScalarReference` choice.
- `validator()`: `Empty`, `ScalarReference`, `ScalarTable`, or `Vector(Backend)`.
  Vector validation includes scalar tail grammar checks.
- `selected_backend()`: the eligible backend selected before preflight, if any.
- `output_backend()`: the backend that produced the retained interior bytes;
  tails remain scalar and discarded vector output is not credited.
- `checked_validation()`: independent reference verification of vector validation.
- `checked_output()`: redundant output comparison was attempted, even if it
  detected a mismatch and required scalar recovery.
- `fallback()`: no writer replacement, backend unavailable before writing, or
  backend rejection/output mismatch followed by complete scalar replacement.

Initial scalar selection (small input, custom settings, unsupported CPU,
unhealthy backend or unavailable static route) is not described as a failed
writer. These cases are visible as no selected backend and the actual scalar
validator/writer. The API deliberately does not infer an exact selection reason
from a later, potentially changed health snapshot.

`ScalarReference` never runs the fast classifier to accept input. It does not
force scalar output: eligible vector writing still receives mandatory checked
comparison when that feature is enabled. Ordinary reference validation is not
constant-time. No report is a secret-processing or deployment attestation.

## Static Tokens

`StaticBackendToken::decode_standard_with_report::<PAD>` and
`decode_url_safe_with_report::<PAD>` accept the same validation policy and
return canonical transactional results. Their explicit names distinguish them
from historical token methods, which retain `DecodeError` and historical
short-scalar mutation behavior. No existing signature or routing is replaced.

Only the token's exact backend is considered, and its generation/health is
checked before selection. The new methods conservatively use scalar processing
when the ordinary safe wrapper cannot establish availability. In particular,
an AVX-512 token does not silently downgrade to AVX2, and deployment-only no_std
attestation does not bypass the ordinary wrapper's feature checks. Existing
direct-token APIs retain their prior AVX-512/deployment support. Static calls
need at least one full vector interior block; automatic crossover thresholds
remain unchanged. Tokens remain non-forgeable and thread-bound.

## Health And Recovery

Validation and writing deliberately share the `StrictDecode` latch for each
backend. The KAT exercises both alphabet classifiers and direct decoding, plus
invalid padding, whitespace, high-bit and wrong-alphabet symbols in every lane.
A classifier failure therefore quarantines that backend's strict decode, not
just its validator. Encoding has its own latch; secret algorithm generations
and secret reports remain independent.

False rejection recovered by the reference validator returns a backend error
before mutation. Checked builds likewise reject false acceptance before a proof
can authorize output. Actual invalid input is not a backend-health fault.
Backend availability/health is checked again at writing. Rejected kernels and
checked mismatches quarantine the backend and overwrite the entire interior
with the scalar table writer, including previously written vector chunks.

Canonical scalar recovery consumes the immutable input/configuration/layout
proof and is infallible: it does not call a fallible decoder after partial output.
There is consequently no reachable scalar-retry error to inject on this path.
The older historical checked decoder's fallible retry and `ScalarRetryFailed`
classification are not changed or reclassified by this checkpoint.

Health admission remains nonblocking and process-local. A concurrent first-use
KAT can cause scalar fallback. Generation checks are admission snapshots, not
synchronous cancellation of in-flight calls. The existing PID-based fork refresh
is unchanged; a completed report is a value, not a reusable authorization token.

## Verification

`sh scripts/check-2.1-decode-composition.sh` runs active Rust and MSRV matrices:
core-only integration compilation, alloc/std, SIMD, checked comparison, static
policy tests, KAT mutation tests, health-state tests, Clippy and safety policies.
Tests also compare complete destinations and error precedence, custom/relaxed
fallbacks, tails, report value independence and allocation-free reporting.

Fault tests invoke real canonical and static operation boundaries. They cover
false acceptance/rejection, backend unavailability and output corruption under
both policies. An isolated native x86 subprocess changes the real irreversible
quarantine/generation state between preflight and writing and verifies scalar
recovery and invalidation of an existing token. This is not a fork conformance
test and is not presented as native evidence for other architectures.

Cross-target functional checks and development timings are recorded with the
commit verification results. They do not replace the later native hardware,
independent review, or release fuzz campaign.

Local verification passed on Rust 1.99.0 and MSRV 1.90.0, including the complete
workspace all-feature release suite, core-only integration target, feature
matrices and Clippy. Focused reports passed Miri; checked composition passed
AArch64 QEMU and WASI/Wasmtime, and RISC-V checked test targets cross-compiled.
An explicitly AVX2-compiled no_std checked test build exercised static methods.
These are functional checks, not fresh native ARM, Mac, Windows or RVV evidence.
The loader's scalar artifact is byte-identical; its regenerated SIMD artifact
and embedded digest changed together and the Node loader suite passed.

## Development Timing Caveat

Unreported writers use a zero-sized observer captured by value. Native release
assembly inspection confirmed that the ordinary writer entry point no longer
constructs or passes a report. Reported calls retain their own observation state.
This removes the explicit bookkeeping cost, but does not prove timing parity.

The trusted public-API harness was built identically with Rust 1.99.0,
`std,simd` and `--cfg base64_ng_perf_evidence` against parent
`51ac4b5d58e97b6ac76bc411a96acc32852f6b59` and this checkpoint. The comparison
uses seven alternating-order pairs, four profiles, six existing decode surfaces
and seven sizes, pinned to CPU 2 on the local Ryzen 9950X3D desktop. No reported
API is substituted for an existing call in this comparison.

The 168 rows include 72 within 5%, 67 regression signals, 22 inconclusive and
7 noisy rows. Several bulk regressions are approximately 6-8%; small-call
signals often add roughly 2-14 ns. Results across builds varied, but even pinned
runs retained signals. Do not dismiss them as noise or claim this checkpoint
is performance-neutral. Investigating or explicitly accepting these changes
remains required before performance admission; no automatic thresholds change.

Raw data: `target/release-evidence/2.1-commit15-composition/native.json`.
SHA-256: `a5fa0237f8d5be51cf444f87575c58921ad60c6b83bbd15fd10ebeb5fd5b9bf4`.
Baseline binary: `a1faeb1e51f822cce45ddcf82e9a62181d362e6580c78499cfc1ec0df7eb76b7`.
Candidate binary: `9d38cce89afe204eafa2bcbb1965c89d827cf03dc2a802b6abd7b9a2295bcfc2`.
