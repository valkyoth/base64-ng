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
Two focused fresh-build Neoverse-V2 repeats had no signals outside the existing
five-percent band. Those V2 results neither invalidate V1 nor establish CPU
differences as the sole cause. No allocation increases were observed.

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

This is a production reporting change, not a test-only or metadata-only
correction. The existing pinned campaign-source exception does not authorize
it. Preserve the original campaigns and their original source identities;
the unsigned staged candidate collection is not final evidence for this
runtime change. Release sealing remains blocked pending independent review
and an explicit new campaign/evidence decision. No prior performance, fuzz,
native, sanitizer or formal result is relabeled as a run of this correction.

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
