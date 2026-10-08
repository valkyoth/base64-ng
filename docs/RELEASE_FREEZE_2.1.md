# 2.1 Release Freeze

Commit 26 prepared the API, documentation, packages and campaign tooling.
This is not release authorization. Its follow-up `335dd35` passed the
maintainer-supplied external retest and GitHub CI before the first campaigns.
The subsequent RVV measurement corrections require their own review and CI;
the revised campaign source and remaining work are recorded below.

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
| Local Linux x86 | x86 fuzz, backend/checked tests, local deep gates | Revised-source local preparation passed; old-source x86 fuzz reuse proposed |
| Physical Linux X60 | Native RVV admission bundle | Exact revised-source capture passed and was collected |
| AWS AArch64 | NEON/checked native bundle and portable fuzz | Old-source native/NEON fuzz captures collected; scoped reuse proposed |
| Apple Silicon | NEON, paired capture, Safari package smoke | Old-source capture collected and performance tradeoffs accepted; scoped reuse proposed |
| Windows x86_64 MSVC | Active/MSRV gate, ABI and performance bundle | Old-source capture collected and scoped assembly reviewed; scoped reuse proposed |
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
`06f9e65432631d43f868a95bb6083c18a61a1f39`. Independent review must cover
`335dd35..b3e493e`, followed by CI acceptance. Do not treat the first campaign's
external PASS or green CI as review of these corrections.

The source-equivalence validator rejects reuse of the first campaign across
this range: protected Rust evidence hooks, a performance manifest, harnesses
and tooling changed. The historical correction exception is not extended.
Old captures retain their original identities and are not automatically
current-source release evidence. All 18 fuzz targets currently remain pending
in the new exact-source session. The scope assessment below proposes retaining
the three completed non-RVV shards and native ARM/Windows/macOS captures instead
of repeating them. Until independently approved and supported by per-artifact
collection, the existing strict gate still rejects that reuse.

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

Before provisioning paid workers, review/push the corrections and require green
CI. Remote workers clone the exact commit from GitHub, so a local-only
commit is not launch-ready. A practical allocation is eight Linux x86 workers
for the 15 portable targets in two waves, one ARM worker for NEON/native checks,
and one Windows worker if native reuse is rejected. With the scoped exception
approved and integrated, only the eight portable-fuzz workers are needed from
this allocation: local x86/NEON fuzz and ARM/macOS/Windows captures can be retained.
Run performance captures without concurrent fuzzing or builds
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

### Scoped Reuse Proposal

The earlier blanket rerun recommendation was a consequence of the current
global source policy, not an identified non-RVV regression. A subsequent scope
assessment proposes an exception only for the exact transition from `335dd35`
to `b3e493e`, followed solely by the existing permitted metadata changes.
It does not expand the historical v2.0 correction exception.

The full correction has 11 changed paths and 1,128 unchanged tree entries,
including file modes. Its two changed library files only alter RISC-V-gated
evidence routing/imports. The performance manifest adds a cfg lint declaration;
the changed benchmark body is RVV-only. Remaining changes concern RVV capture,
validation, regression checks and documentation. Non-RVV runtime paths, test
bodies, fuzz targets/helpers/seeds, dependency locks and toolchain are unchanged.
The pinned binary Git diff SHA-256 is
`340f5cff4dbfc3d2a7aafce6a04b3f2d77fef12799c6da9b642d4c23717bf3f1`.

Proposed retained scopes are Linux AArch64 native/paired evidence, Apple Silicon
native/paired/Safari evidence, Windows x86_64 MSVC evidence, and the completed
`x86_encode`, `x86_decode` and `neon` fuzz shards. Their original source remains
`335dd35`; a reuse mapping must never rewrite it to the candidate hash.
All old RVV correctness, performance, codegen and admission claims are excluded.
Use the already verified `b3e493e` RVV bundle instead.

Both retained NEON bundles were revalidated in historical mode, including their
unchanged performance thresholds. All three fuzz shards passed original-source,
architecture, duration, corpus and log-hash checks. ARM, Windows and macOS
archive digests match the collected originals; all 51 Windows command logs
match their recorded hashes and successful exits. Historical validation does
not itself grant candidate applicability, signatures or performance acceptance.
This proposal claims source-scope applicability, not binary identity or identical
performance on every host. Existing checked-mode tradeoffs remain explicit.

The unsigned local proposal is
`target/fuzz-manager/reuse-proposal-335dd35-b3e493e.json`, SHA-256
`79f540228fbf15ca10ac70b54fc216eba10d87290de3d3735f87c9ca15e71027`.
It binds original archive/shard hashes, exact source trees, the changed-file
inventory and the local review-tool hashes. The supporting scope checker,
eight-case regression suite and proposal builder are retained beside it, outside
the frozen source. Tests passed under ordinary Python and `python3 -O` and
reject wrong commits/trees, unsupported scopes/architectures including RVV,
protected edits even when reverted, dirty candidates, metadata symlinks and merges.

Independent review must explicitly accept this proposal before reuse. The final
collector also needs a reviewed per-artifact source mapping that accepts the
original non-RVV captures and the new RVV capture separately. The existing
`BASE64_NG_CAMPAIGN_SOURCE_COMMIT` is not suitable: it binds fuzz and RVV to one
source. No global override, release validator, signature policy, original bundle
or manager completion record was changed for this assessment. Release acceptance
remains blocked until the scoped integration and all other gates pass.

The 15 portable one-hour fuzz campaigns have not run yet and remain new work,
not reruns. Under the proposed exception no additional native AWS/macOS capture
or repeat of the three completed architecture-specific fuzz campaigns is needed
solely because of this RVV correction. Any further protected-source change
invalidates this proposal and requires a fresh scope assessment or recapture.

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

Still required after external review and CI acceptance: the 15 pending one-hour
fuzz targets, approval/integration of scoped reuse or recapture of the remaining
three fuzz/native scopes, full Miri and normal/advanced Kani inventories, release
sanitizer/timing campaigns, deferred QEMU/full-release CI, complete assembly
review and strict evidence aggregation.
Run collection against the detached frozen source. Any later metadata/report
reuse must satisfy the existing signed-evidence equivalence procedure; do not
simply overwrite source hashes in retained bundles.

## Final Acceptance

Retain the signed candidate evidence before making the report-only commit.
`security/evidence-reuse-allowlist.txt` names exact 2.1 metadata paths, not
directories or globs; runtime, tests, harnesses, manifests and tooling cannot
be changed under metadata-only reuse. The separate historical campaign-source
correction exception remains specific to its old reviewed inventory; do not
use it as a generic 2.1 bypass.

Commit 28's only change is `security/pentest/v2.1.0.md`, with PASS bound to its
exact reviewed first parent. Rebuild candidate-bound package/SBOM records and
verify the mechanical evidence-equivalence index, all hashes and signatures.
Require the full-release GitHub run on that exact commit. Only then, with
maintainer authorization, sign/tag and publish through the release runbook.
