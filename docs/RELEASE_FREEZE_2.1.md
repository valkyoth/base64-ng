# 2.1 Release Freeze

Commit 26 prepared the API, documentation, packages and campaign tooling.
This is not release authorization. Its follow-up `335dd35` passed the
maintainer-supplied external retest and GitHub CI before the first campaigns.
The subsequent RVV corrections passed the maintainer-supplied review through
`a022372` and its routine GitHub CI. Final campaigns and release authorization
are still pending; the accepted source and remaining work are recorded below.

## Shipping Surface

The 13 Rust crates, WASM artifact crate and npm loader are version 2.1.0.
Rust 1.99.0 is active; MSRV remains 1.90.0. All 13 API snapshots are under
`api-snapshots/v2.1.0`; the signed 2.0.4 compatibility baseline is unchanged.
No dependency update, ISA admission or threshold change is part of this freeze.

Ordinary Auto, reference validation, checked comparison, per-call reports,
borrowed views, bulk incremental/adapters/in-place and companion feature
forwarding are documented in [migration](MIGRATION.md) and the linked contract
guides. The progressive experiment is removed, not a conditional shipping API.
Secret/CT algorithms are unchanged. Ordinary buffers are not secret storage.

The package inventory excludes engineering docs, CI/release scripts, API
snapshots, proofs and hardware evidence. Core includes two compiled examples
and has a 238-file ceiling. The package gate verifies core from its archive;
companions are inventoried and tested in the workspace, since their exact 2.1
dependencies cannot be registry-resolved until published in dependency order.
Do not report a companion publish dry-run as passed before that happens.

The npm gate builds twice, rebuilds at another path, verifies embedded pins,
tests the exact packed package and binds generated `PROVENANCE.json` to HEAD.
Generated provenance is not a tracked self-referential commit promise; it is
refreshed when packaging the final source. npm's CI-generated provenance is
separate from this artifact digest/source record.

## Freeze Gates

Run from the repository root with Python 3.11+ (including `tomllib`):

```sh
sh scripts/checks.sh --all
sh scripts/check-api-snapshots.sh --check
sh scripts/check-2.1-release-freeze.sh
python3 scripts/test-fuzz-evidence-manager.py
python3 scripts/test-fuzz-shard-evidence.py
python3 scripts/test-evidence-equivalence.py
sh scripts/test-release-readiness.sh
CARGO_TARGET_DIR="$PWD/target/semver-freeze-2.1" cargo +1.98.1 semver-checks \
    check-release --workspace --baseline-rev v2.0.4 --all-features
```

`checks.sh --all` includes active/MSRV feature gates, unchanged downstream
consumer tests, docs, examples, safety policy, npm and browser tests. It is not
the full-duration campaign. `stable_release_gate.sh candidate` is the expensive
collector, reserved for Commit 27. Routine GitHub green excludes the deferred
full-release QEMU/feature jobs; dispatch those for final acceptance as described
in the [plan](2.1.0-release-plan.md).

The maintainer authorized final publication metadata in Commit 26: the
`synced-family` plan selects all 13 Rust crates and npm at 2.1.0, and the
temporary Cargo/npm development blocks are removed. The internal WASM artifact
crate remains unpublished. This makes manual publication technically possible,
not authorized: scripts still require a verified signed tag, and final release
acceptance remains mandatory. Never tag or publish from this checklist alone.
Manifests and publish plans are protected evidence inputs, not permissible
post-campaign metadata changes.

## Campaign Handoff

After external review and CI pass, record the exact clean `git rev-parse HEAD`
and `git rev-parse HEAD^{tree}` in the manager session and each device bundle.
Do not use a moving branch name as evidence identity. Do not change source,
tests, harnesses, manifests, dependencies, tools or compiler during campaigns.

| Assignment | Commit 27 work | Availability / status |
| --- | --- | --- |
| Local Linux x86 | x86 fuzz, backend/checked tests, local deep gates | Revised-source local preparation passed; two one-hour x86 campaigns pending |
| Physical Linux X60 | Native RVV admission bundle | Exact revised-source capture passed and was collected |
| AWS AArch64 | NEON/checked native bundle and portable fuzz | Fresh revised-source native capture and one-hour NEON fuzz pending |
| Apple Silicon | NEON, paired capture, Safari package smoke | Fresh revised-source capture pending |
| Windows x86_64 MSVC | Active/MSRV gate, ABI and performance bundle | Fresh revised-source capture pending |
| Local/CI runtimes | Node, Wasmtime, Chromium, Firefox, QEMU compatibility | Revised-source core/runtime checks passed; full-release deep campaigns pending |

Keep credentials/IPs in local manager state, not committed documentation.
Fill in exact machine capabilities and the candidate hash before launching.
The manager owns 18 one-hour fuzz targets plus the native RVV job; Windows,
Apple Silicon and other native/backend bundles still need their named collectors.
A complete manager menu is not proof that every release requirement ran.

Use `python3 scripts/manage-fuzz-evidence.py` for resumable distribution. Start
native RVV and one local x86 target first; distribute the remaining targets only
after their setup and verification succeed. Preserve every requested duration.
Validate downloaded archives before deleting paid machines. Do not run two
manager jobs concurrently on the same assigned host.

Use a fresh explicit state path for this release, leaving earlier sessions
intact, for example `--state target/fuzz-manager/2.1.0.sqlite3`. Reuse that same
path for status, resumption and finalization; do not reset a running session.

Remote workers require a C/C++ build toolchain, Git, curl, Python 3.11+,
tar/gzip and normal POSIX utilities. The bootstrap checks Python/tomllib before
cloning or launching a candidate. Optional rustup/system-package installation
requires explicit manager confirmation; no Windows provisioning is implied.
On macOS, verify the actual `python3` on PATH after Homebrew installation.
Linux paired-performance isolation additionally requires Bubblewrap and delegated
cgroup-v2 CPU/memory/PID controllers. Windows prerequisites are in
[WINDOWS_2.1.md](WINDOWS_2.1.md).

New remote hosts are not live-tested by offline bootstrap fixtures. Check their
prerequisites and exact source identity before accepting a launch as a campaign.
The earlier Commit 26 ARM endpoint timeout was only a provisioning check.
Subsequent native ARM captures were completed on replacement hosts; no endpoint
or credential is part of this committed handoff.

## Revised Campaign Source

The first campaign used
`335dd35401bbf5fa2fc7a5c5379819ff10cd1377`. Native RVV correctness passed, but
its performance harness repeatedly invoked candidate-only capability probes.
The correction separates candidate correctness testing from production-cached
performance detection; the encode/decode admission thresholds are unchanged.
The follow-up also checks the root library directly under both evidence cfgs
so dependency lint caps cannot hide warnings.

The revised campaign source is
`b3e493e64a583245f7aab542d983b7b914c67eb9`, tree
`06f9e65432631d43f868a95bb6083c18a61a1f39`. On 2026-10-09, the maintainer
supplied a clean external review of `335dd35..a022372`: no Critical, High,
Medium or Low findings. That range includes all RVV corrections. The only
change after `b3e493e` through the reviewed head is this freeze document;
the protected implementation, tests, harnesses, dependencies and tools match.

[Rust CI](https://github.com/valkyoth/base64-ng/actions/runs/37820778022) and
[CodeQL](https://github.com/valkyoth/base64-ng/actions/runs/37820776864) both
completed successfully for exact reviewed head
`a022372e1e3e9be6840a6db34d704dfe158a3c79`. This is not a claim that CI ran on
`b3e493e` itself. The push run deliberately skipped the 2.1 feature gates and
big-endian/RVV/SVE QEMU jobs; final full-release CI remains mandatory.

The source-equivalence validator rejects reuse of the first campaign across
this range: protected Rust evidence hooks, a performance manifest, harnesses
and tooling changed. The historical correction exception is not extended.
Old captures retain their original identities and are not automatically
current-source release evidence. All 18 fuzz targets currently remain pending
in the new exact-source session. The maintainer chose fresh campaigns instead
of adopting the proposed non-RVV exception. Repeat the three old-source
architecture-specific shards and native ARM/Windows/macOS captures; run the
15 portable shards for the first time. No collector integration for the proposed
exception is planned, and no old-source result is relabeled.

The replacement RVV bundle already uses the revised source. Native tests,
signal/thread/ABI checks, generated assembly and the full 15-sample performance
matrix passed, without compiler warnings. Its archive SHA-256 is
`780fed93ece37873251dd1b60f5414f9ef91e10738ca1a7b235008dc22671efb`.
The archive and all bundle hashes were verified after download; the unchanged
performance validator passed locally. No further RVV rerun is needed while
the protected source stays unchanged.

Keep the first manager database intact. The revised local database is
`target/fuzz-manager/2.1.0-b3e493e.sqlite3`, session
`b3e493e64a58-e4229995`. Its RVV entry contains the imported, revalidated
exact-source bundle, not a relabeled earlier result. Run job operations from
the detached `target/campaign-b3e493e` worktree; this prevents documentation
edits in the main worktree from interrupting clean-source checks. From the main
repository root, inspect status with:

```sh
python3 scripts/manage-fuzz-evidence.py \
    --state target/fuzz-manager/2.1.0-b3e493e.sqlite3 --status
```

To launch or collect jobs, enter `target/campaign-b3e493e` and use
`--state ../fuzz-manager/2.1.0-b3e493e.sqlite3` with the same manager script.

The corrected campaign commit is available on GitHub, and the covering review
and routine CI have passed. Remote workers must clone the literal `b3e493e`
commit recorded above, not the moving documentation HEAD. A practical allocation
is eight Linux x86 workers
for the 15 portable targets in two waves, one ARM worker for NEON/native checks,
and one Windows worker. The two x86-specific fuzz targets run locally; Apple
Silicon capture remains maintainer-run. Run performance captures without concurrent fuzzing or builds
on that host. The full target assignment is retained locally in
`target/fuzz-manager/aws-plan-b3e493e.json`; every target appears exactly once.

### Retained Native Review

The following reviews apply only to the first `335dd35` captures, not to the
revised source or final release authorization:

- ARM revisions contain 54,720 observations and 1,824 cells; controls contain
  9,600 observations and 320 cells, all with 15 pairs. Artifact hashes and
  recomputed summaries agree, with no timed allocation increases. The version
  comparison has 858 improvement and 292 regression signals; controls compare
  different operations and must not be counted as version regressions.
- ARM default/SIMD/adapters have no large (at least 64 KiB), valid, warm
  regression signals. The 24 large checked regressions are canonical/owned
  encode and validation-only calls. All 24 also appeared in the earlier
  Commit 25 capture at comparable ratios: canonical encode about 0.31x,
  owned encode about 0.34x, validation about 0.94x. The documented independent
  checks and diagnostic recovery remain intact; this is not blanket approval
  of every small, cold or malformed-input result.
- Windows scoped ABI/cleanup review covers 19 functions per compiler for
  Rust 1.99 and 1.90, including both alphabet variants. Every recorded XMM
  callee-save slot has a matching restore. After normalizing symbol and
  read-only constant-label spelling, their instruction sequences match the
  previous reviewed capture. Constant bytes are not proved by this comparison.
  The ordinary stack-copy, rejected-first-block cleanup and incoming-register
  limitations in [the Windows review](WINDOWS_2.1.md#native-assembly-review)
  remain. This is not general assembly equivalence or a secret-erasure claim.
- Windows performance summaries were recomputed from all 5,760 observations
  (192 cells, 15 pairs). There are 157 improvement signals and 15 regression
  signals, with no timed allocations. All 15 regressions compare 32-byte
  historical/exact operations to scalar in the same source; they are not
  version regressions or justification for changing admission thresholds.

The Windows assembly SHA-256 values are
`ff9173dee2a59625520e592cb2c0b64f38f6d447f54ec4bfb158e83cfeaf212f`
(1.99) and
`34a72d46d87afe4d591bd79be727700c674ab1076f83e806d671a6aa6944ca8a`
(1.90). The local unsigned review record is
`target/fuzz-manager/native-review-335dd35.json`. Raw captures are not shipped
in the crate. The ARM/Windows AWS hosts were cleared for shutdown only after
collection and verification; historical review does not require them to stay up.

### Reuse Proposal Not Adopted

On 2026-10-09, the maintainer selected full exact-source campaigns instead of
the scoped non-RVV reuse exception. The local proposal and earlier captures
remain historical records, not release inputs for the revised campaign.
No release validator, global override, signature policy or original bundle was
changed. Do not enable `BASE64_NG_CAMPAIGN_SOURCE_COMMIT` for that proposal.
The completed `b3e493e` RVV bundle and local checks already match the frozen
source and do not need repeating solely because this decision was documented.

## Earlier Freeze Verification

Completed freeze checks include all 13 API snapshots against 2.0.4, package
inventories, a verified core archive/publish dry-run, active/MSRV policy-example
runs, release-plan/equivalence/readiness mutation tests and exact npm tests.
The npm dry-run contains 10 files; no upload was performed. Both WASM artifacts
remain byte-identical to the previously reviewed source.

Workspace `cargo-semver-checks 0.49.0` passes with all features against signed
`v2.0.4` using installed Rust 1.98.1 for rustdoc generation. Its parser does
not support Rust 1.99's v61 rustdoc format, so that compiler is not claimed for
this tool. Active 1.99/MSRV 1.90 build and feature checks are separate.
The standard suite passed in sections: the initial `--all` run completed the
development matrices but was blocked by sandboxed user-systemd bus access at
the public-API isolation gate. That entire gate passed outside the sandbox,
followed by a successful `checks.sh --core` run. This includes browser/package
tests, default/all-feature/core-only tests, Clippy, doctests, dependency policy
and RustSec scanning. Workspace rustdoc also passed with warnings denied.
No full-duration fuzz, fresh native-device campaign or deferred full-release CI
run is claimed by these local results.

## Revised-Source Local Preparation

On 2026-10-08, the clean detached `b3e493e` worktree completed:

- `checks.sh --core`, all 13 API snapshots, the release-freeze gate, and
  active 1.99/MSRV 1.90 validation and companion feature matrices. Strict
  Clippy, package inventories, dependency policy and RustSec scanning passed.
- The extended 2.1 security gate: four focused Miri tests, both Kani validator
  and in-place geometry proofs, 13 AddressSanitizer integration tests with leak
  detection enabled, and 1,000 seeded executions each for in-place and
  incremental fuzz smoke. These are not the complete release-duration campaigns.
- RVV production/candidate cross-build and generated-code checks. Local x86
  does not execute RVV; native execution is covered by the separately collected
  exact-source X60 bundle above.
- WASM deterministic/path-independent artifacts, all 26 npm loader tests,
  Wasmtime, Chromium and Firefox checks. This is not a fresh Safari result.
- Workspace semver checks against `v2.0.4` using the supported Rust 1.98.1
  rustdoc producer, SPDX/CycloneDX SBOM generation, reproducible library/package
  checks and the core `cargo publish --locked --dry-run`. No upload occurred.
- Fifteen-sample exact x86 encode/decode performance gates. Admitted kernels
  met the unchanged 1.02x scalar threshold; automatically selected AVX-512
  encode sizes also met 1.05x AVX2. AVX-512 decode remains observational/static.
- SIMD assembly generation and its pattern checks for x86 and cross-built
  NEON. This does not replace the complete final assembly review.

The verified core package contains 238 files, 1.9 MiB unpacked and 378.9 KiB
compressed. All 13 publishable Rust packages, the internal WASM artifact and
the npm loader remain at 2.1.0. Optional external OpenPGP interoperability was
skipped locally because the required GnuPG/Sequoia pairing was unavailable.
Nightly Cargo emitted existing redundant-homepage manifest warnings; active
and MSRV strict Clippy passed. Neither omission is relabeled as a completed test.

The local revision comparison contains 54,720 observations (1,824 cells), and
same-source controls contain 11,520 observations (384 cells), each with 15
alternating pairs. Capture inventories, exact source/harness identities, tool
hashes, sample counts and recomputed summaries were verified. Neither comparison
increased timed allocations or changed diagnostic/destination outcomes.

Revision results contain 813 improvement signals, 301 regression signals,
597 within five percent, 97 noisy and 16 inconclusive cells. Default, SIMD and
adapter groups have no large (at least 64 KiB), valid, warm regression signals.
All 24 large checked regressions also occurred in the earlier Commit 25 local
capture: canonical encode is about 0.45-0.46x, owned encode 0.47-0.49x, and
incremental decode 0.82-0.84x baseline throughput. These are the existing
[checked-mode tradeoffs](POLICY_2.1.md#explicit-tradeoffs), not new evidence
that checks should be removed. Small, cold and rejected-input regressions remain
in the complete results. Noisy or inconclusive cells are not counted as passes.
Controls compare different operations in the same source and are not version
regressions. These exploratory desktop measurements are not formal hardware
admission or a claim about every deployment.

Logs and captures are retained locally under
`target/campaign-b3e493e/target/pre-aws` and `target/pre-aws-supplement` within
that worktree. The unsigned analysis is
`target/fuzz-manager/pre-aws-review-b3e493e.json`. These local paths are not
shipped evidence or signed release authorization.

Still required: all 18 one-hour fuzz targets on `b3e493e`, fresh ARM/macOS/Windows
captures, full Miri and normal/advanced Kani inventories, release sanitizer/timing
campaigns, deferred QEMU/full-release CI, complete assembly review and strict
evidence aggregation. The clean scoped pentest and routine green CI above do
not substitute for these campaigns or the final report-only release acceptance.
Run collection against the detached frozen source. Any later metadata/report
reuse must satisfy the existing signed-evidence equivalence procedure; do not
simply overwrite source hashes in retained bundles.

## Native Inventory Correction

The exact `b3e493e64a583245f7aab542d983b7b914c67eb9` campaigns have now
completed all 18 one-hour fuzz targets, native ARM/macOS/Windows collection,
the separate desktop Safari run, the five Miri groups, sanitizer and release
timing runs, and full big-endian, RISC-V and SVE QEMU gates. The big-endian
campaign ran on Ubuntu after the local PowerPC cross-linker prerequisite
failed; both the original failure and complete replacement capture remain.
The local backend, target-build and CT/RVV/SVE assembly-generation gates also
passed. Generated assembly checks are not the complete manual assembly review.

The 45-harness normal Kani inventory passed with one explicitly approved
per-harness CLI unwind bound of 5 for
`incremental_padded_decoder_progress_and_retry_are_bounded`; its original
bound of 12 exhausted both 8 GiB and 16 GiB limits. All unwinding/safety checks
remained enabled, and a bound-1 negative control failed. The other 44 harnesses
retained their annotated bounds; all 19 configured advanced harnesses passed.
This does not claim that the original unmodified bound-12 command passed.
The original failures, negative control, override and full rerun are retained.

Strict inventory assembly exposed a tooling error: Commit 53 still selected
the historical 2.0 NEON bundles, which correctly failed source freshness.
Fresh Apple Silicon and Linux ARM bundles have separately passed the original
strict admission validator, on the exact frozen commit, without runtime-drift
overrides. Both are now required under
`target/release-evidence/neon-native-admission/`; historical tracked bundles
are neither replaced nor relabeled as fresh evidence.

The maintainer authorized a dedicated tooling correction and retention of the
completed runtime campaigns. `validate-campaign-source-equivalence.py` pins
both the frozen commit and tree and the exact original correction
`0ace376a0e737b0ead1926920e403a5b448e7988`. Independent review found that the
original path-only correction check did not authenticate that identity. The
hardening in `427e938165fc9eef6fef02cf9131985d9714e1e2` changed six enumerated
validator/test scripts. The later CI fix
`a0dd4e9638f56100f0a790b199d052a00b3cedb3` changed only the fuzz-shard test fixture:
it now supplies both exact-source NEON bundles instead of the historical marker.
The final policy binds both follow-ups by exact identity, signature, order and
file inventory, with only its own validator and regression test allowed to change.
It requires a new independent review and an out-of-band full commit hash through
`BASE64_NG_REVIEWED_CAMPAIGN_POLICY_COMMIT`; the old hardening anchor cannot
authorize this extension. All four signatures must verify under the frozen
release-signer policy. The trusted
bootstrap procedure is in `docs/RELEASE_EVIDENCE.md`; running a candidate's
self-check is not itself a trust bootstrap. Only the
two named evidence documents may accompany that correction. Every intervening
commit is checked, including changes later reverted. Runtime, dependencies,
toolchains, tests, fuzz inputs, ABI code and benchmark harnesses remain outside
the exception. The existing metadata-only allowlist is unchanged, and the
historical 2.0 exception is not broadened.

Final aggregation retains original per-artifact source identities and records
`runtime_campaign_commit` separately from the release candidate. Candidate
SBOM/package records must still be regenerated; all outcome, hash and signing
requirements remain. The correction requires its own independent review
before release acceptance, not a declaration that old commands passed.

The Neoverse-V1 default in-place encode slowdown remains an explicit release
disposition item: eight large cells were 5.9-9.8 percent slower than 2.0.4.
Two focused fresh-build Neoverse-V2 repeats in that earlier campaign had no
signals outside the existing five-percent band. Those historical V2 results
neither invalidate V1 nor establish CPU differences as the sole cause. No
allocation increases were observed. The later corrected-source V2 recapture
has a separate [accepted limitation](#accepted-neoverse-v2-performance-limitation).

Still pending: independent correction review, complete assembly review,
performance disposition, candidate-bound package/SBOM refresh, signed evidence
aggregation, deferred full-release CI and final report-only acceptance.

## Runtime Reporting Correction After Freeze

Independent review of the reporting-test stabilization found a production
policy defect: selected AVX2 health could be attributed to an AVX-512
candidate, and a quarantined upper tier could hide a lower tier still
initializing when checking `ScalarExecutionOnly`. The correction binds health
to the named backend and captures terminal scalar status across every
automatically available tier for each operation. Deterministic tests cover
all three-tier state/availability combinations, both operations, selected
health attribution and snapshot identifiers.

A follow-up review corrected an overly conservative inventory: automatic
strict decode must exclude exact/static-only AVX-512 even when its KAT is
healthy. The per-operation inventories are regression-tested against the
x86 dispatch admission probes; explicit initialization retains all tiers.

This is a production reporting change, not a test-only or metadata-only
correction. The existing pinned campaign-source exception does not authorize
it. Preserve the original campaigns and their original source identities;
the unsigned staged candidate collection is not final evidence for this
runtime change. Release sealing remains blocked pending independent review
and an explicit new campaign/evidence decision. No prior performance, fuzz,
native, sanitizer or formal result is relabeled as a run of this correction.

### Focused Revalidation Proposal (Not Authorization)

The proposed reporting correction boundary is
`498748bf57018d527de38077cc2da83388fdcc3f` through
`f921e97184859bea4efe1fea410f3500c9a33cc7`, including the preceding reporting-test
stabilization. Fresh native captures must bind their actual source commit and
tree, compiler identities, feature combinations, executed test counts, command
outcomes and log hashes. A clean checkout and fresh compiler-specific build
directories prevent stale-cache results from being mistaken for new checks.

The initial Apple Silicon capture exposed a separate test-build gap: the
SIMD-only NEON unit test used a `std` printing macro, and three `std`-only test
helpers had broader feature gates than their callers. The follow-up aligns
these test gates without changing production kernels. The NEON gate now
checks test targets without `std` and executes the affected encode test under
both active Rust and MSRV when native execution or QEMU is available. Retain
the failed capture; do not convert it to a successful result or silently omit
that configuration from the replacement capture.

The next no-`std` Apple Silicon run exposed a reporting-test race: its bounded
initialization wait was enabled only by the crate's `std` feature, despite the
unit-test harness already using threads. The test-only wait and its
deterministic lower-tier regression now also run in no-`std` SIMD tests using
the harness's standard library. Production initialization remains nonblocking;
neither its algorithm nor its availability policy changes.

The proposed impact assessment is:

| Evidence | Required disposition |
| --- | --- |
| Runtime reports, scalar policy and backend-health state tests | Fresh native x86, Apple Silicon, Linux AArch64 and Windows MSVC checks on active Rust and MSRV; compact active-Rust RISC-V supplement with its scope explicit. |
| NEON test-build correction | Fresh no-`std` SIMD and checked test builds and execution on ARM; retain QEMU as supplemental, not native evidence. |
| Codec fuzzing, codec geometry proofs and secret/CT checks | Candidates for scoped reuse only after review confirms their implementation, harnesses, inputs and dependencies are unchanged. Reporting and health-dependent assertions are excluded from blanket reuse. |
| Native ABI, assembly, sanitizer, timing and performance campaigns | Assess each campaign's dependency on changed reporting/health code and compiler output. Unchanged codec source alone is not proof of identical linked machine code or timing; rerun affected checks or record an explicitly reviewed limitation. |
| SBOM, packages, reproducibility and final evidence index | Regenerate for the final candidate; never relabel older candidate-bound artifacts. |

All historical captures retain their original identities. Focused captures
are unsigned development evidence, not substitutes for the entire frozen
campaign. Their private capture tooling must accompany the logs and be
reviewed before admission. Compact RISC-V checks do not claim a full MSRV,
RVV hardware, performance or long-fuzz campaign.

Acceptance requires an independent decision on the exact correction commits
and evidence inventory, followed by a separately reviewed validator policy
with an out-of-band trust anchor. The current validator remains unchanged and
fails closed for the reporting correction. No evidence has been sealed and no
full-release CI, final review, signature or publication requirement is waived.

### Completed Reporting Rechecks and Review Packet

The following results bind the actual tested source
`29cc6ef2c4dd60f67e0692591b9dabf2753b0bc1`, tree
`7688efc6fce5aab0a06bbc68b7af234ed178a601`. They do not change the original
campaign identity `b3e493e64a583245f7aab542d983b7b914c67eb9`, tree
`06f9e65432631d43f868a95bb6083c18a61a1f39`.

The maintainer supplied a clean independent SAST report for the inclusive
range `f921e97^..29cc6ef`. That review included the production automatic-tier
inventory correction and both test-only corrections. Its ARM checks were
cross-build/codegen checks, not native execution. The separately collected
native results below supply focused execution coverage. Neither the SAST
report nor these captures approves reuse of the older campaigns.

| Native platform | Toolchains | Passing test executions |
| --- | --- | ---: |
| Linux x86_64 | 1.99.0 and 1.90.0 | 214 |
| Windows AMD64 MSVC | 1.99.0 and 1.90.0 | 214 |
| Linux AArch64 | 1.99.0 and 1.90.0 | 192 |
| macOS Apple Silicon | 1.99.0 and 1.90.0 | 192 |
| Linux RISC-V | 1.99.0, compact supplement only | 40 |

The total is 852 executions, including repeated tests across features and
compilers, not 852 unique tests. The archive retains the earlier failed
captures, intermediate successes, capture helpers and their regression tests.
Only the final successful reports at the source above contribute to this
table. Log hashes, report hashes, source trees, exit codes and execution counts
were checked against the retained files. Local archive:
`target/reporting-revalidation-29cc6ef.tar.gz`, SHA-256
`d3b3af1aabce3438d48065d26811ced55c6d257c91dddd5d3923903e79fd9830`.

Additional local checks use the same source, sanitized compiler environments
and fresh private build directories:

| Supplement | Result and scope |
| --- | --- |
| Miri | Four exact reporting/policy tests passed; not the complete Miri campaign. |
| ThreadSanitizer | Ten runtime tests, one concurrent-first-use test and one assurance-report test passed; not the complete sanitizer inventory. |
| Assurance | Eleven unit tests and seven integration tests passed on each of 1.99.0 and 1.90.0: 36 executions. |
| Companion feature isolation | All six companions, eight configurations and both toolchains passed: 96 configurations, 90 test executions; the remaining configurations are compile-only. |
| WASM reporting consumer | Fresh SIMD module built from a committed-source export; the runtime-dispatch smoke ran successfully in Node/V8 and Wasmtime. This is not a Safari rerun or an npm release gate. |

Miri and ThreadSanitizer used nightly `a30aa9064` (2026-10-08), not the older
campaign's nightly `c36f14571` (2026-10-01). These are supplemental results,
not identical-toolchain or identical-binary evidence. The initial Miri attempt
failed while creating its sysroot cache under a read-only sandbox; that failed
report remains retained alongside the successful permitted retry.

Local supplemental archive: `target/reporting-followup-29cc6ef.tar.gz`,
SHA-256 `ddfac6c4a8f25141a18cb624217639065054e70f7631194b3d54248d0c1e5496`.
It contains the supplemental capture helpers, their imported native helper,
reports and logs, including the failed attempt. Both archives are unsigned,
operator-retained development evidence, not repository-hosted or sealed release
bundles. Supply the actual archives to the independent reviewer; hashes in this
document alone do not establish correctness, provenance or approval. Review
private command metadata before redistributing captures publicly.

### Campaign Impact Assessment for Review

The source comparison below is between the full frozen and tested commits
above, not merely the last test-only commit. Exactly these protected Rust
paths differ:

```text
src/decode_backend/policy/tests.rs
src/encode_backend/policy/tests.rs
src/runtime/mod.rs
src/runtime/operation.rs
src/runtime/report.rs
src/runtime/tests.rs
src/simd/mod.rs
src/simd/neon.rs
src/simd/tests.rs
src/v2/backend_health.rs
src/v2/backend_health/tests.rs
```

The production delta adds operation-specific terminal-scalar reporting and
binds health, generation and faults to the named backend. Atomic health-state
transitions, KATs, codec algorithms, dispatch admission, SIMD kernel bodies,
secret processing and wiping implementations are unchanged. The NEON gate
changes affect test compilation/execution, not production kernel instructions
by construction; this is not a claim that every newly linked binary is equal.

Git object equality was checked for `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `crates/`, `fuzz/`, `kani/`, `dudect/`, `perf/`,
`portability/`, `tests/` and `packages/`. Proof implementations under
`src/kani*_proofs.rs` are also unchanged. Equality of these inputs does not
establish equality of their entire linked dependency or machine-code closure:
each still builds against the core crate, whose reporting implementation changed.

| Retained campaign or artifact | Assessment and proposed disposition, pending approval |
| --- | --- |
| 18 one-hour codec fuzz campaigns | Harnesses, corpora, dependency locks and codec implementations are unchanged. Retain as historical coverage of those codec properties; do not claim the new reporting policy was fuzzed or that these binaries were built at `29cc6ef`. Recommend scoped reuse of codec coverage only. |
| Kani codec, geometry, secret and assurance-model proofs | Proof sources and modeled algorithms are unchanged; source inspection found no calls to the changed reporting functions in the proof files. Recommend reuse of the original proof obligations with their exact original bounds and limitations, not a new proof of runtime reporting or linked native code. |
| Miri and sanitizer campaigns | Retain old codec/ownership checks as historical coverage; add the fresh reporting Miri, concurrency TSan and assurance results above. The supplement does not rerun ASan/LSan, every Miri test or every target. Accepting the unchanged codec checks requires an explicit scope decision. |
| Assurance-context reporting | `src/v2/assurance/report.rs` calls `runtime::backend_report()`. Refreshed unit/integration and focused TSan results are required; do not infer this consumer is unaffected merely because its own source is unchanged. |
| Companion consumers | `portability/companion_features/common.rs` observes the runtime report. The refreshed isolated feature matrix above supersedes reliance on old reporting assertions, without claiming new package publication evidence. |
| WASM and browser captures | The runtime-dispatch smoke calls `backend_report()` and has been refreshed in Node/Wasmtime. The old Safari and loader campaigns retain their original source/artifact identities. Final package provenance and hashes must be regenerated and checked; the supplement cannot authorize a different browser artifact. |
| Performance and native admission | Both `perf/src/main.rs` and `perf/public-api/src/main.rs` collect runtime reports outside their measured loops. Preserve old measurements as historical threshold evidence, not freshly measured performance of this correction. Unchanged loops do not prove identical linked layout or timings. The accepted V1 slowdown remains disclosed. |
| Native ABI, assembly and secret/CT timing | Original wrappers, kernels and timing harnesses are unchanged, but linked/compiler-output equivalence has not been established for this correction. Retain only with an explicitly reviewed original-source limitation, or recapture affected checks. The focused native reporting runs are not replacements for ABI or timing campaigns. |
| QEMU and full-release CI | RVV/SVE tests contain reporting assertions. Require the final full-release workflow, including deferred emulation gates, on the exact final candidate. No old CI result substitutes for this run. |
| SBOM, package inventories, reproducibility, final index and signatures | Regenerate candidate-bound records after policy approval. Never copy an old source identifier into a new artifact or seal the currently staged collection as final. |

This is a source-impact assessment, not a formal call-graph proof. In
particular, no binary/timing equivalence is inferred from a text search.

### Required Independent Decision

The recommendation is to reuse only the unchanged property coverage described
above, preserve all original source/toolchain identities and bounds, and attach
the fresh focused checks as a separately identified supplement. Native ABI,
assembly, timing and performance limitations require explicit acceptance or
recapture; they are not silently included in codec-source reuse.

The reviewer needs the original retained campaign inventory, both supplemental
archives, the source diff and this assessment. Review must cover the exact
additional correction history: `c8ca4da`, `498748b`, `f921e97`, `6c8b109` and
`29cc6ef`, as well as the already pinned historical corrections. A clean SAST
verdict alone is not that decision. Record acceptance or required recaptures
per campaign, including whether the capture helpers and their actual logs are
admissible; do not approve evidence from this narrative alone.

Only after that decision should a new scoped validator policy be implemented
and independently reviewed, with an out-of-band policy anchor. It must bind
exact signed correction identities/order, changed-file inventories, the fresh
supplement source and hashes, complete file inventories, and each authorized
claim. It must reject missing/failed supplements, source drift, substituted
corrections and extra or linked evidence entries. Do not merely add runtime
paths to the existing metadata exception or retain its assertion that runtime
source is unchanged.

The existing validator was retested: its mutation suite passes, and the actual
frozen-to-`29cc6ef` check still fails with `2.1 native-inventory correction
inventory mismatch`. No validator, workflow or evidence-acceptance rule was
relaxed by this assessment. Final signing, full-release CI, independent final
review, tagging and publication remain separate requirements.

### Independent Review Disposition

The maintainer supplied the separate reviewer session's completed
`evidence_reuse_review`. This is independent process review by a session other
than the evidence collector, not a third-party audit or certification. Its
decision supersedes the pending proposal above only for the stated claims;
it does not authorize release, waive recaptures or approve a new validator.

The reviewed packet SHA-256 is
`1f16c9c4ab1eea0d9758a46064a63ec0d6a3c0100451176de3a819e57ff395da`;
its `MANIFEST.json` SHA-256 is
`fd343d249af9774f1705abe121f06c3771e7880163eb4f39488857b4babd2ce6`.
Both were independently recomputed against the collector's original retained
packet, whose packet digest was recorded before receiving the review, rather
than trusting a digest copied from the review alone. This is a local provenance
cross-check, not a reviewer signature. The packet and supplements remain
unsigned. The reviewer checked all 1,557 manifest-listed files; including the
manifest itself, the transport has 1,558 files and 276 historical directories.
The two supplement archive hashes are recorded in the preceding section.

| Campaign | Reviewed disposition |
| --- | --- |
| All 18 one-hour codec fuzz campaigns | Accept scoped reuse of unchanged codec, format, incremental, adapter, kernel and assurance-model properties. Exclude the corrected reporting policy and report consumers. |
| 45 normal and 19 advanced Kani harnesses | Accept original proof obligations and original bounds only. The incremental progress/retry proof remains unwind 5, not the unsuccessful bound 12. No proof of new reporting or native machine code is implied. |
| Historical five-group Miri and fresh four-test reporting Miri | Accept scoped reuse plus the separately identified fresh supplement; not a complete current-source Miri campaign. |
| Historical ASan and LSan | Accept unchanged codec, ownership and allocation coverage, not exact-current-binary coverage. |
| Historical TSan plus fresh focused TSan | Accept original unchanged concurrency properties plus the fresh reporting/backend-health tests; not a full new sanitizer inventory. |
| Five native reporting captures at `29cc6ef` | Accept the fresh focused results and explicit RISC-V compact/active-only limitation. They are not full native ABI or admission captures. |
| Fresh assurance and companion checks | Accept the 36 assurance executions and 96 companion configurations/90 executions as supplements. |
| Fresh Node/Wasmtime reporting smoke | Accept that smoke only, not final npm or browser artifact acceptance. |
| Historical big-endian QEMU | Accept unchanged scalar/endian codec coverage only. |
| Historical WASM SIMD kernel/codegen properties | Accept unchanged source-property coverage only, not a newly packaged browser artifact. |
| Native ABI/cleanup and complete native/CT assembly review | Require recapture and review of generated output, especially Windows active/MSRV. |
| Dudect/CT timing | Require a fresh exact-source release-binary timing campaign. |
| Performance and native admission | Require native Windows, Linux ARM, Apple Silicon and RVV recaptures. Old results remain historical context, including the V1 regression. |
| RVV and SVE QEMU | Require recapture because these suites contain reporting assertions. |
| npm/WASM loader, Chromium, Firefox and Safari | Require newly generated, reproducible packages and runtime/browser checks. |
| SPDX/CycloneDX, packages and reproducibility | Regenerate for the new candidate. The staged `af49843` records are not transferable. |
| Exact-final-candidate full-release CI | Not reviewed; still required. |
| Final evidence index and signatures | Not reviewed; still required after complete inventory validation. |

The review retained the original frozen source/tree and fresh `29cc6ef`
source/tree recorded above. Accepted historical results must never be relabeled
as executions at the fresh source. Capture helpers and logs provide scoped
development evidence; they do not authenticate compiler distributions, machine
ownership or Cargo caches. Some follow-ups were not offline. Private operator
path metadata in the WASM supplement needs review before public redistribution.

### Required Recapture Source

Use the fixed clean source `516f785294732bb3a61ebcf9c8eb9a52b17fdf09`, tree
`7e502a7992120690e11aa2aca50b77a074b10281`, for the required recaptures. Its only
change after the fresh tested source `29cc6ef` is the evidence-assessment
documentation. Runtime, tests, harnesses, packages, tooling, dependencies and
toolchain match the reviewed/tested source. Both ordinary Rust CI and CodeQL
completed successfully for `516f785`; this is not the required final
full-release CI run.

Collectors must use separate clean checkouts and fresh output directories,
preserving the old campaigns and failed attempts. The local pinned checkout is
`target/recapture-516f785`; collector-generated evidence is under its own
`target/release-evidence`. Local CT/subtle/x86 SIMD assembly generation and
the WASM loader/package gate have passed there. These successes do not claim
the complete manual assembly review, browsers, timing, emulation, native
admission or final packaging inventory is complete.

Use one Linux ARM and one Windows host in addition to the existing Apple
Silicon and physical X60 hosts. Do not restart the 18 accepted fuzz campaigns
or the accepted Kani inventory. Keep Safari separate when the SSH build user
has no desktop session. Collect and verify each host's evidence before its
shutdown; a remote script exit alone is not release acceptance.

Record source identities for every recapture. Any later production, test,
toolchain, harness or dependency change needs a new impact assessment. A
future claim-specific reuse policy still requires independent review and an
out-of-band anchor; this documentation does not extend the existing policy.
Do not seal the mixed historical staging tree. Complete the required
recaptures, final-candidate CI and reviewed inventory/signatures first.

### Accepted Neoverse-V2 Performance Limitation

The maintainer explicitly accepted the corrected-source Neoverse-V2 slowdown
as a documented 2.1 performance limitation. This decision is separate from the
earlier Neoverse-V1 acceptance and applies to source
`516f785294732bb3a61ebcf9c8eb9a52b17fdf09` versus 2.0.4 on the measured host.

Two fresh-build focused repeats each measured 48 cells with 15 alternating
pairs per cell. They covered default and SIMD in-place, canonical and historical
encoding, all four strict profiles, and 64 KiB/1 MiB random inputs with warm
calls. Repeat one flagged four default-mode in-place encode cells at
5.26-5.62 percent higher paired latency; repeat two flagged two at
5.67-5.74 percent. No SIMD regression signals or allocation increases occurred
in this focused set. This is not a claim that every ARM workload is unchanged.

The retained focused archive SHA-256 is
`77fc0d7f3e13914d24675fd32aead0159a9b006e511c3399321e4ede0917a19d`.
Its transport digest, capture file hashes and recomputed summaries were checked.
The unsigned raw captures and the unfavorable classifications remain intact;
acceptance does not relabel these cells as passing the five-percent band.

Retain the shared implementation rather than introducing an ARM-specific 2.0.4
rollback. Follow up with a targeted investigation of default-mode in-place
encoding, using these cases as the regression baseline before accepting an
optimization. The cause is not established. This exception changes no routing,
benchmark threshold, correctness check, security requirement or evidence-reuse
policy, and does not authorize release or waive the remaining gates.

### Corrected-Source Recapture Status

The following recaptures use the fixed clean `516f785` source/tree recorded
above. They are unsigned operator-retained evidence, not a new final manifest
or authorization to reuse every historical campaign. The earlier collection
notes remain historical; the results below supersede their pending statuses
only within the stated scopes.

| Required work | Current result and boundary |
| --- | --- |
| Windows AMD64 MSVC | Native active/MSRV tests, ABI probes and 192 same-source performance control cells completed. Download verified against its SSH transport digest, source inventory and indexed artifact/log hashes. The 38 reviewed ABI/cleanup function bodies cover 19 per compiler, not arbitrary linked code. Controls are not a Windows version-regression comparison. |
| Linux ARM and Apple Silicon | Native validation, workspace checks, assembly capture and admission bundles completed. Both exact-source NEON bundles passed the existing two-percent admission floor. Full paired policy captures and unfavorable classifications are retained; the separate accepted ARM limitation above does not waive correctness or admission. |
| Physical SpacemiT X60 | Native correctness, signal/thread/ABI checks, assembly and 15-sample admission capture completed and verified. The exact X60 profile remains required; no other RVV CPU is admitted. |
| RVV and SVE QEMU | Both complete unmodified gates passed. RVV exercised VLEN 128/256 and SVE 128/256/512 plus fallback/static checks. These are emulation correctness results, not native timing, ABI or admission evidence. SVE remains non-admitted. |
| Assembly review | Fresh release/LTO and concrete CT-consumer inventories were reviewed across x86, ARM, Apple Silicon, Windows, RISC-V, s390x and PowerPC64, with production WASM IR inspection. The comparison record covers 295 function bodies across configurations, plus the Windows scope and nine RVV/five SVE leaf instruction streams. This is scoped operator review, not an independent whole-binary or microarchitectural proof. |
| Dudect | Fresh release run passed the existing floors: 20,000 samples per case, 64 iterations, 1,000 warmups and threshold 10. Nine equal-work cases among 12 total had maximum absolute t of 2.275. Unrelated VM/desktop activity was present; do not describe this as a controlled quiet-host campaign. Public-length cases are informational. |
| WASM/npm and browsers | Fresh package, reproducibility/path-independence, 26 loader tests, Wasmtime, Chromium, Firefox and real Safari checks passed. Safari's sender-supplied archive digest matches the collected bytes and its artifacts match the native Mac package capture. |
| SBOM and Rust reproducibility | SPDX/CycloneDX and reproducible build/package collectors passed at `516f785`. These records still must be regenerated for the final policy/tag candidate; they are not automatically transferable. |
| Local x86 paired policy capture | Complete: 1,824 revision cells and 576 same-source control cells, each with 15 alternating pairs. Raw summaries, exact case/sample coverage, diagnostics, harness hashes and complete capture-file inventories verified. No allocation increases or large valid warm regression signals in default/SIMD/adapter revision groups. Unrelated host workloads were present; this is not a quiet-host admission campaign. |

The collected transport SHA-256 values are:

| Capture | SHA-256 |
| --- | --- |
| Windows complete | `45cf5830a103c86718fc6e2f27b9cbf36fe531fe86beae67807993c25b7956f1` |
| Linux ARM complete | `e552a40ffecfe355dde475a0c1314c2745caec6079c32c02b6520645efcef433` |
| Apple Silicon complete | `32553d5c42d350e6ba164aefb418b1d8d2235388de477f145df2348e02849f6f` |
| Physical X60 complete | `e62a6b2fcd41ca562f3568ba3326acd3237be81a250e514cb2bf443843c40e88` |
| Safari desktop session | `3161a818bf238d236bc30cd5f6d95ee3ee6e10a74b921abc72cb777cd565287c` |

These hashes bind retained private archives, not signatures or public download
locations. The Apple Silicon archive preserves its initial failed assembly
invocation as well as the successful explicit-target follow-up. Intermediate
assembly comparisons remain retained; only the corrected v4 inventory supports
the manual comparison record. Private path and host metadata require review
before public redistribution.

The local x86 revision classifications are 795 improvement signals, 602 within
the five-percent band, 289 regression signals, 120 noisy and 18 inconclusive.
Retain all classifications. Large checked canonical encode ratios are
0.455-0.469 (about 2.1-2.2 times the old latency); large checked incremental
decode ratios are 0.815-0.822 (about 22 percent slower). These and the small,
fragmented and rejected-input costs remain within the categories disclosed in
[the explicit policy tradeoffs](POLICY_2.1.md#explicit-tradeoffs), not a promise
that every call improves. No threshold, routing or verification change follows
from this capture. The 576 controls compare operations on one source and must
not be counted as version-regression evidence.

Local review record `target/x86-policy-review-516f785.json`, SHA-256
`c5a05b2c4ffb29822f7dc8ec682b0894e32c67d29f5f65c63354d01de9866681`,
retains both complete capture inventories and every flagged cell. It is an
unsigned review aid, not a release-admission result.

The first local x86 attempt failed before measurement because its execution
sandbox could not connect to the host user bus. Its log and failure report are
retained separately. The successful host-access retry kept the existing
candidate Bubblewrap sandbox and verified aggregate resource limits; it did
not fall back to unrestricted candidate execution.

Historical approved-property records were copied without rewriting their
source IDs after checking the independent review packet's transport digest,
manifest digest, complete file/directory inventory and all 1,557 indexed file
hashes. The staged Miri, sanitizer, fuzz, Kani, big-endian and WASM SIMD records
remain original-source evidence. Fresh reporting supplements retain `29cc6ef`;
fresh native admission, timing, assembly and RVV/SVE reports retain `516f785`.
An outcomes-checker success is not approval of this mixed-source collection.

The old source-equivalence validator still rejects this history with
`2.1 native-inventory correction inventory mismatch`. Do not bypass that failure
or assign one global source identity to all campaigns. A separately reviewed,
claim-specific policy must bind the exact correction history, original claims,
fresh supplements, required recaptures and complete inventories. Remaining
release steps include that policy's independent review/out-of-band anchor,
final-candidate metadata refresh, exact-final-candidate
full-release CI, evidence index/signatures and final report-only acceptance.

### Scoped Policy Preflight Proposal

`security/campaign-claims-2.1.json` inventories the exact signed history through
`636d3e8`, three distinct source identities and fourteen narrowly scoped claims.
It pins both complete private review transports, not only selected successful
logs. Failed attempts, performance regressions and claim exclusions remain in
the retained bytes. The recapture packet SHA-256 is
`e0742e369ac07826e2dc5ec16b0a5400578855d749f1ff58021609c857020997`;
its manifest SHA-256 is
`b41422afef25adae18a3ee3e61aa1e0e0afc0ade9f5083c2e770672809cf62e6`.

The read-only `scripts/validate-scoped-campaign.py` preflight verifies this
proposal without extracting archives, executing bundled code, copying records,
changing the finalizer or approving release. It checks transport digests before
parsing, bounded complete file/directory inventories, all indexed file hashes,
source/status metadata and claim attestations. It consumes exactly one gzip
member through its checksum and EOF, rejecting additional transport bytes,
concatenated members, nonzero member padding, hidden tar trailers, missing end
markers and excessive zero padding. It drains the tar reader's own buffer so
prefetched trailing data cannot evade inspection. It rejects links, special
entries, duplicate paths/JSON keys and non-canonical names. Ordinary CI runs synthetic
mutation tests only, not the expensive historical or native campaigns.

Proposal inspection is explicitly unapproved and makes no current-candidate
claim. Reviewed mode additionally requires an independently supplied exact
policy commit, its authorized signature, exact parent/path inventory and a
clean matching HEAD. No later commit, including a revert, is implicitly allowed.
Follow the trusted bootstrap in `RELEASE_EVIDENCE.md`; the validator's own drift
check is not a trust root. The release account must remain isolated from other
same-user writers. Archives still contain private metadata and are not public
release assets.

The old blanket source-equivalence validator and finalizer remain unchanged and
blocked. This preflight is deliberately not wired in as an acceptance bypass.
Independent review of the policy and recaptures must precede claim-aware final
manifest integration. Final-candidate metadata, full-release CI, signatures and
maintainer authorization remain required. Neither preflight mode emits a final
manifest or reports `release_authorized=true`.

The separate bounded `security/campaign-attestations-2.1.json` contains the
operator's scope assessments. Each attestation binds a campaign, source commit
and tree, disposition, exact scope/exclusions, result and required evidence
inventory digest. The policy pins both the entire summary file and each
canonical attestation. The reviewed policy signature binds these summaries;
inspection mode only checks their consistency, not independent approval.
Redirecting a native claim to historical fuzz evidence no longer satisfies it.

Nested reporting supplements have top-level summaries of their complete inner
inventories and the exact five passing report identities each. Earlier failed
reports remain retained but excluded. These summaries were prepared by checking
the retained archives, not by relabeling earlier runs. Their `passed` result
means the recorded scoped operator assessment passed, not that every benchmark
cell improved. Performance exceptions, failed attempts and independent-review
requirements remain explicit. No unsigned summary can authorize finalization.

## Final Acceptance

Retain the signed candidate evidence before making the report-only commit.
`security/evidence-reuse-allowlist.txt` names exact 2.1 metadata paths, not
directories or globs; runtime, tests, harnesses, manifests and tooling cannot
be changed under metadata-only reuse. The separate historical campaign-source
correction exception remains specific to its old reviewed inventory. The
separate pinned 2.1 native-inventory correction above is not a general waiver
for future tooling changes.

The policy hardening also rejects extra native-NEON root entries and unindexed
empty directories, linked files and special entries throughout retained
campaigns. Regression fixtures cover signed correction identities and invalid
signatures as well as FIFO/socket/hard-link inventory mutations. These are
tooling changes only: original frozen runtime campaigns retain their identities.
The unsigned `0ace376` index is superseded, not approved for sealing. Commit and
independently review the hardening before selecting its exact policy hash and
regenerating candidate-bound evidence. No tag or publication is authorized by
these changes.

Commit 28's only change is `security/pentest/v2.1.0.md`, with PASS bound to its
exact reviewed first parent. Rebuild candidate-bound package/SBOM records and
verify the mechanical evidence-equivalence index, all hashes and signatures.
Require the full-release GitHub run on that exact commit. Only then, with
maintainer authorization, sign/tag and publish through the release runbook.
