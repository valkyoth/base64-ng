# 2.1 Release Freeze

Commit 26 prepares the API, documentation, packages and campaign tooling.
This is not release authorization. Commit 25 through `7bcdaf0` passed the
maintainer-supplied external retest and GitHub CI. Commit 26 still needs its
own full-range review and CI acceptance before a campaign hash is frozen.

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
| Local Linux x86 | Initial x86 fuzz, backend/checked tests, local deep gates | Workstation; not started |
| Physical Linux X60 | First native RVV admission bundle | Reconfirm SSH/device before launch |
| AWS AArch64 | NEON/checked native bundle and assigned portable fuzz targets | Reconfirm/provision host; not started |
| Apple Silicon | NEON, assigned native fuzz, paired capture, Safari package smoke | Maintainer-run commands; not started |
| Windows x86_64 MSVC | Frozen active/MSRV gate, ABI and performance bundle | New server required; prior server deleted |
| Local/CI runtimes | Node, Wasmtime, Chromium, Firefox, QEMU compatibility | Final runs pending |

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
The previously supplied AWS ARM endpoint timed out during Commit 26 preflight;
no remote installation or candidate job was attempted.

## Local Verification

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
