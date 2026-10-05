# Decode Composition Checkpoint

Commit 15 adds ordinary per-call execution reports. This is a development
checkpoint, not release admission. External review and CI passed through
`d3c84b2`; Commit 16 corrects the selected-backend comment noted by review.
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
At the initial `93cf09a` checkpoint the loader's scalar artifact was
byte-identical; its regenerated SIMD artifact and embedded digest changed
together and the Node loader suite passed. The follow-up below rebuilds both.

## Initial Development Timing Caveat

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

## Performance Follow-Up

The follow-up retains the classified alphabet family inside the private proof,
bound to the same owned settings. Writing and reporting no longer repeat that
classification. Empty `Auto` input constructs its zero-length proof before
classification/dispatch; `ScalarReference` still executes reference validation.
Inlining hints keep the validator close to preparation and the specialized
vector writer outside the larger preflight writer. No validation, capacity,
checked comparison, health recheck or quarantine is removed.

A public-gate rerun exposed a test-only initialization race: an earlier selection
could choose SSSE3 while another thread's AVX2 KAT was in progress. The test
helper now settles every available ordinary decode backend before comparing
selection snapshots. An isolated-process regression checks those latches.
Production admission remains nonblocking.

Both final captures use the same parent, compiler, features, seven alternating
pairs and CPU-2 affinity described above. A same-binary control put all 168
cases within 5%. With no other agent test jobs running during final capture:

| Result | Capture 1 | Capture 2 |
| --- | ---: | ---: |
| Within 5% | 118 | 121 |
| Improvement signal | 12 | 13 |
| Regression signal | 14 | 14 |
| Inconclusive | 24 | 20 |

All 48 bulk cases (64 KiB and 1 MiB payloads) are within 5% in both captures.
Remaining regression signals are at 0, 3, 32 and 192 bytes, approximately
0.9-17.8 ns slower. Thus the earlier bulk signals are no longer reproduced,
but small-call performance parity is **not** established. Acceptance of these
tradeoffs, external review and CI remain pending. This is local x86 `std,simd`
development evidence, not a native ARM/Mac/RVV/Windows performance claim.

Additional regressions cover retained strict/custom/relaxed families, settings
ownership and empty-input reference work. The active/MSRV composition and
public-decode gates, workspace all-feature release tests, core-only tests,
Clippy, focused Miri, AArch64 QEMU and WASI/Wasmtime checks passed. Both WASM
artifacts and their pins are rebuilt; all 26 Node loader tests passed.

Files under `target/release-evidence/2.1-commit15-composition/`:

- `same-binary-control.json`: `f5d88dd4837a3096bc3e81f57cbf8f2deb018760c6f1c54fe6acd4b5c3355fc7`.
- `followup-1.json`: `b39f1f6e4b4dd1a6f5f189f7eabba08d70a7df50200ebf6f2d2f9b105fd7f47f`.
- `followup-2.json`: `68848fcf06c2ad745aeafdd52c4d60db5f6c6b7e1b381aff3daf4330f359ab39`.

Candidate binary: `ff6e40efdc2bb66511b8b094c0000d23da13837cffa9c94a6b180f14d4325ecf`.
The baseline binary hash is unchanged from the initial capture above.
