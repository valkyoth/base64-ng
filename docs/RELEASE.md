# Release Checklist

This checklist is for maintainers preparing a `base64-ng` release.

For 2.0.0, [`2.0_RELEASE_FREEZE.md`](2.0_RELEASE_FREEZE.md) records the exact
candidate boundary. The synchronized publish plan does not authorize release
without Commit 55, final pentest acceptance, green required CI, and the signed
tag.

## Preflight

- Confirm `Cargo.toml` has the intended version.
- Confirm `Cargo.toml` repository and homepage point to `https://github.com/valkyoth/base64-ng`.
- Confirm `CHANGELOG.md` has a section for that version.
- Confirm `license = "MIT OR Apache-2.0"` remains unchanged.
- Confirm `LICENSE-MIT` and `LICENSE-APACHE` are present at the repository root.
- Confirm no unwanted dependencies were added.
- Confirm every package selected in `release-crates.toml` has a frozen
  `api-snapshots/vX.Y.Z/` record.
- Confirm the wasm loader provenance binds its npm version, artifact digests,
  and exact Git source commit.

## Required Gate

During development, run:

```sh
scripts/stable_release_gate.sh check
```

After the source and pentest fixes stop moving, run strict candidate evidence:

```sh
scripts/stable_release_gate.sh candidate
```

Commit 55 then runs `release` as described in the publish section. Both strict
modes refuse pre-release versions and unavailable required evidence tools.
Candidate or release mode may reuse an accepted expensive campaign only when
`BASE64_NG_REUSE_EVIDENCE_FROM` names a clean ancestor and the metadata-only
equivalence gate accepts every intervening path and protected-tree hash.

The release gate covers:

- stable-version enforcement when run in `release` mode
- formatting
- metadata
- documentation version consistency checks
- public API audit status checks
- packaged release script presence, executable-bit, and shebang validation
- dependency graph
- packaged dependency admission policy
- reserved feature placeholder checks with inert-feature and per-feature
  dependency graph validation
- fail-closed wasm wipe policy check and explicit
  `allow-wasm32-best-effort-wipe` opt-in build check
- fuzz-only dependency checks when `fuzz/` is present
- clippy
- feature-mode tests
- Miri no-default-features tests and exact-source manifests; unavailable Miri
  is a hard failure in `candidate` and `release`
- all-features and no-default-features doctests
- all-features and no-default-features docs
- packaged async admission policy while the `tokio` feature remains inert
- installed cross-target `no_std` checks
- no-alloc portability smoke crate checks for installed Linux, FreeBSD, wasm32,
  ARM, and Cortex-M targets
- CI target-matrix no-alloc smoke checks for each installed `no_std` target
- reserved SIMD feature-bundle compile checks for AVX2, AVX-512 VBMI,
  SSSE3/SSE4.1, NEON, and wasm `simd128` when the corresponding Rust targets
  are installed
- cargo-deny policy
- RustSec audit
- license inventory
- dudect-style timing harness compile and dependency checks
- fixed-work timing/source-boundary policy checks
- constant-time assembly evidence generation
- SBOM generation
- reproducible package/build check

## Local Toolchain Setup

Install the cross targets used by `scripts/check_targets.sh` and CI:

```sh
rustup target add aarch64-unknown-linux-gnu x86_64-unknown-freebsd wasm32-unknown-unknown thumbv7em-none-eabihf s390x-unknown-linux-gnu powerpc64-unknown-linux-gnu riscv64gc-unknown-linux-gnu
```

On openSUSE, the big-endian and RISC-V QEMU evidence scripts use user-mode
QEMU plus SUSE-style cross compiler names:

```sh
sudo zypper install qemu-linux-user
sudo zypper install cross-s390x-gcc16 cross-s390x-binutils cross-s390x-glibc-devel cross-s390x-linux-glibc-devel
sudo zypper install cross-ppc64-gcc16 cross-ppc64-binutils cross-ppc64-glibc-devel cross-ppc64-linux-glibc-devel
sudo zypper install cross-riscv64-gcc16 cross-riscv64-binutils cross-riscv64-glibc-devel cross-riscv64-linux-glibc-devel
```

Commit 31 and the stable release gate require both big-endian targets:

```sh
scripts/check_big_endian_qemu.sh --all
```

The script accepts SUSE and Debian/Ubuntu cross-toolchain layouts and
preflights target start files and libc objects. On Debian/Ubuntu install
`qemu-user`, `gcc-s390x-linux-gnu`, `libc6-dev-s390x-cross`,
`gcc-powerpc64-linux-gnu`, and `libc6-dev-ppc64-cross`. Per-target diagnostic
modes do not count as complete release evidence. QEMU proves functional
correctness and scalar fallback only; it is not real-hardware performance,
timing, cleanup, or side-channel evidence.

Native operators can produce a structurally checked community report with
`scripts/check_big_endian_hardware.sh` and the schema under
`hardware-evidence/big-endian/`. A report does not admit acceleration without
the separate backend review.

`scripts/check_riscv_qemu.sh` requires the `riscv64gc` path and the `lp64d`
glibc sysroot used by the SUSE cross packages. It runs complete scalar suites
and the isolated Commit 32 RVV candidate at VLEN 128 and 256.
`scripts/generate_rvv_asm_evidence.sh` checks candidate instruction and cleanup
shape. This is not native RVV performance, ABI/signal preservation, timing,
microarchitectural, register-retention, or side-channel evidence. Native
operators use `scripts/check_riscv_hardware.sh` and the checked schema under
`hardware-evidence/riscv/`; normal published builds stay scalar until that
evidence and external review are accepted.

`scripts/check_sve_qemu.sh` builds a static AArch64 Linux test binary with the
bundled `rust-lld`, runs the complete portable fallback suites, and exercises
the isolated Commit 33 SVE candidate at vector lengths 128, 256, and 512.
`scripts/generate_sve_asm_evidence.sh` checks the exact leaf symbols,
structured loads/stores, predicate mapping, register cleanup, and absence of
nested calls or stack use. Cross-host runs require AArch64 binutils
(`binutils-aarch64-linux-gnu` on Debian/Ubuntu or
`cross-aarch64-binutils` on openSUSE). This is emulation and codegen evidence
only. Native operators use `scripts/check_sve_hardware.sh` and the checked schema under
`hardware-evidence/sve/`; public AArch64 dispatch remains admitted NEON or
scalar until evidence from two real SVE systems with different vector lengths
and external review are accepted.

The no-alloc portability smoke crate checks the same installed target list with:

```sh
scripts/check_no_alloc_smoke.sh
```

Install release and deep-check tools:

CI and local release scripts use `scripts/ci_install_rust.sh`; that script uses
`rust-toolchain.toml` as the single source of truth for the active release
toolchain and fails if the selected `rustc` does not exactly match that pin.
MSRV remains Rust `1.90.0` and is checked separately in the compatibility
matrix.

```sh
cargo install --locked cargo-audit --version 0.22.2
cargo install --locked cargo-deny --version 0.20.2
cargo install --locked cargo-license --version 0.7.0
cargo install --locked cargo-sbom --version 0.10.0
cargo install --locked cargo-nextest --version 0.9.140
cargo install --locked cargo-fuzz --version 0.13.2
cargo install --locked kani-verifier --version 0.67.0
```

Verify installation:

```sh
cargo audit --version
cargo deny --version
cargo license --version
cargo sbom --version
cargo nextest --version
cargo fuzz --version
cargo kani --version
```

The release gate detects these as Cargo subcommands, for example `cargo
nextest --version`, not by looking for standalone binaries named
`cargo-nextest`, `cargo-fuzz`, or `cargo-kani` on `PATH`.

Optional local timing evidence for the constant-time-oriented decoder can be
collected with:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

The timing manifest distinguishes thresholded equal-work classes from
informational public-length scaling. It binds a run to the source commit,
lockfile, compiler, target, CPU, release feature set, flags, and output
checksum. Run it on idle native hardware for each target included in a timing
claim.

Generate target-specific release/LTO code-generation evidence with:

```sh
BASE64_NG_CT_ASM_TARGET=aarch64-unknown-linux-gnu \
  scripts/generate_ct_asm_evidence.sh
```

Cross-compiled assembly does not substitute for native timing evidence. The
full claim boundary and reviewer procedure are in
[`2.0_TIMING_AND_CODEGEN.md`](2.0_TIMING_AND_CODEGEN.md).

## Miri Setup

Miri is a nightly Rust component. Install it with:

```sh
rustup toolchain install nightly --component miri
rustup component add rust-src --toolchain nightly
cargo +nightly miri setup
```

Kani may need a one-time setup after `cargo install --locked kani-verifier`:

```sh
cargo kani setup
```

Kani bundles its own Rust compiler and is intentionally documented as a
separate verifier pairing. The current supported local path is Rust `1.90.0`
with `cargo-kani 0.67.0`; `scripts/check_kani.sh` runs the
no-default-features harness set through that toolchain when it is available.
The exact 2.0 release-host advanced set runs with:

```sh
BASE64_NG_KANI_ALL_ADVANCED=1 scripts/check_kani_advanced.sh
```

Both runners enforce bounded resources and retain evidence under
`target/release-evidence/kani/`.
If a future Kani/compiler pairing is incompatible with this crate's
`rust-version`, the script records an explicit skip rather than treating it as
proof.
The Kani compatibility and verifier policy is documented in
[`KANI.md`](KANI.md).

The standard local gate runs isolated dudect, fuzz, and performance harness
compile/dependency checks without running timing campaigns, fuzz campaigns, or
benchmarks. The local release gate also runs:

```sh
scripts/check_miri.sh
scripts/check-2.0-in-place-sanitizers.sh
```

If nightly Miri is not installed, the gate prints an explicit skip message.
`scripts/check_miri.sh` covers the no-default-features scalar surface and the
all-features alloc/stream surface. The large deterministic sweep tests remain
part of the normal stable test suite, but are ignored under Miri because Miri
interprets code and those sweeps are not practical there.
The sanitizer command requires nightly `rust-src` and records the in-place
AddressSanitizer result under `target/release-evidence/`.

## Evidence

Release evidence is generated under:

```text
target/release-evidence/
```

Expected artifacts:

- `base64-ng.spdx.json`
- `base64-ng.cyclonedx.json`

The published crate package includes the core release/check scripts, Rust
toolchain pin, and cargo-deny policy so the documented gate can be inspected
with the packaged source.

Fuzz-only dependencies are included in the standard local gate and can also be
checked directly with:

```sh
scripts/check_fuzz.sh
```

The `fuzz/` package is not part of the published crate. Corpus admission rules
are documented in `docs/FUZZING.md` and checked by
`scripts/check_fuzz_corpus.sh`.

Run the streaming fuzz smoke when changing stream state machines:

```sh
cargo +nightly fuzz run stream_chunks -- -runs=1000
```

Run every bounded campaign with:

```sh
BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh
```

Before the final 2.0 release seal, run the release-duration campaign and retain
its generated evidence:

```sh
BASE64_NG_RUN_FUZZ_RELEASE=1 \
BASE64_NG_FUZZ_SECONDS_PER_TARGET=3600 \
scripts/check_fuzz.sh
```

Review generated local corpus files before committing. Commit only small,
non-sensitive inputs that preserve a regression, protocol boundary, or edge
case not already represented by deterministic tests.

Reserved SIMD feature bundles are checked with:

```sh
scripts/check_simd_feature_bundles.sh
```

Capture local backend evidence with:

```sh
scripts/check_backend_evidence.sh
```

The script writes a backend evidence manifest under
`target/release-evidence/backend/` with toolchain metadata, command status, and
checksums for the captured runtime backend report and inactive SIMD prototype
equivalence output.

## Pentest Evidence

Root `PENTEST.md` is temporary scratch input. Do not commit it.

For the 2.0 final candidate:

1. Finish every pre-seal source, documentation, and checkpoint-record change.
2. Run `scripts/stable_release_gate.sh check`, push, and require green CI.
3. Run the requested recent-range pentests and a preliminary full-range review
   against the exact pre-seal source. Fix every finding before moving forward.
4. From a clean checkout of that accepted commit, run
   `scripts/stable_release_gate.sh candidate`. Candidate mode fails closed on
   missing Miri, sanitizer, release-duration fuzz, dudect, normal and advanced
   Kani, native-backend, assembly, SBOM, and reproducibility evidence. It writes
   `target/release-evidence/FINAL-MANIFEST.txt` bound to clean `HEAD`, plus its
   detached `base64-ng-evidence-v2` SSH signature. Set
   `BASE64_NG_EVIDENCE_SIGNING_KEY` when the configured Git signing key cannot
   be resolved to the authorized private key.
5. Complete the checkpoint table and immutable workflow/evidence references,
   then request final acceptance review of that exact pre-seal commit. The
   pentester decides whether this is a full-range rerun or a focused delta over
   the already accepted full review. Do not change those records in Commit 55.
6. Fix every final-review finding, repeat steps 2-5 as needed, delete temporary
   root `PENTEST.md`, and make Commit 55 add only the normalized
   permanent report at `security/pentest/v2.0.0.md`.
7. Push Commit 55, require green CI and CodeQL, then run release mode from a
   clean checkout. A new exact campaign uses
   `scripts/stable_release_gate.sh release`. When all intervening commits are
   reviewed release-process metadata and protected repository contents are
   unchanged, set `BASE64_NG_REUSE_EVIDENCE_FROM=<campaign-commit>`. The reuse
   path preserves original campaign provenance and regenerates package, SBOM,
   and reproducibility evidence for the tag candidate. Both paths validate the
   report-only final commit and run
   `scripts/validate-2.0-checkpoint-record.py --final`, which rejects any
   remaining `Pending` cell, non-`PASS` pentest disposition, or missing exact
   Commit 1-54 hash. The self-referential Commit 55 row uses the frozen
   `Report-only release commit (HEAD)` marker and is resolved by the separate
   report-only-commit check. Reuse never permits scripts, workflows, signer
   policy, or the reuse allowlist to change; those changes require a new exact
   campaign.

Before candidate mode, capture the two required native NEON bundles from the
same clean frozen source commit:

```sh
# Apple Silicon macOS
scripts/capture-2.0-neon-admission.sh \
  performance-baselines/dispatch-2.0-neon-apple-silicon

# Server-class AArch64 Linux
scripts/capture-2.0-neon-admission.sh \
  performance-baselines/dispatch-2.0-neon-aarch64-linux
```

Commit both complete directories before candidate mode. The strict hardware
gate independently validates their schema, checksums, source ancestry,
runtime-source identity, host class, metadata allowlist, statistical policy,
and common source commit. Publishable bundles exclude hostnames, UUIDs, home
paths, and unrelated system-wide `sysctl` state. The retained AMD AVX-512
campaign supports exact-host wording; a second AVX-512 microarchitecture is a
2.0.1 corroboration target, not a 2.0.0 claim.

The permanent pentest report commit must only change the report file. The
report must contain `Status: PASS`, `Reviewed-Commit:`, `Tester:`, `Scope:`,
and `Date:` metadata. CodeQL or GitHub security findings that affect the
release decision belong in the same permanent report.

## Publish

The strict pre-seal evidence command is:

```sh
scripts/stable_release_gate.sh candidate
```

After the report-only Commit 55 exists, run the exact-tag-candidate gate or
the fail-closed metadata-equivalent form:

```sh
scripts/stable_release_gate.sh release

BASE64_NG_REUSE_EVIDENCE_FROM=<campaign-commit> \
  scripts/stable_release_gate.sh release
```

Both commands require the authorized evidence-signing private key. By default
the gate resolves `git config user.signingkey` and removes a trailing `.pub`;
otherwise set `BASE64_NG_EVIDENCE_SIGNING_KEY=/path/to/private-key`. The key is
used only by `ssh-keygen -Y sign` and is never copied into release evidence.

An exact strict campaign is intentionally expensive. It requires Miri, sanitizers,
one-hour-per-target release fuzz campaigns, dudect timing, all normal and
advanced Kani harnesses, native NEON evidence, generated assembly, SBOMs,
reproducibility, and the standard local gate. Metadata-equivalent mode reuses
only unchanged expensive campaigns, reruns ordinary checks and current package
evidence, and records both commits. `release` additionally enforces the
permanent pentest metadata, reviewed-parent, and report-only Commit 55.

After the full release gate passes, push the commit, wait for GitHub to become
green, then create and push the immutable signed release tag:

```sh
git tag -s v2.0.0 -m "base64-ng 2.0.0"
scripts/verify-release-tag.sh v2.0.0
git push origin v2.0.0
```

Publish only from the tagged commit:

```sh
scripts/release_crates.py --check
scripts/release_crates.py --dry-run
scripts/release_crates.py
```

`scripts/release_crates.py` reads `release-crates.toml`, validates workspace
crate versions and dependency order, refuses real publishing unless `HEAD`
matches a verified signed `v<version>` tag, runs the standard local gate and
`cargo publish --dry-run` for each selected crate, publishes `base64-ng` first,
waits for crates.io visibility, and then publishes all selected dependent
companions in the order recorded by `release-crates.toml`: IMAP, MIME,
multibase, password-record, OpenPGP, PEM, sanitization, derive, Serde, bytes,
subtle, and Tokio.

The publish helper intentionally does not rerun Kani by default. Kani belongs
to the pre-tag stable release gate so a verifier failure does not happen after
an immutable GitHub tag has already been created. If a release manager wants to
rerun the expensive gate immediately before publishing, use:

```sh
scripts/release_crates.py --full-gate
```

This post-tag rerun uses `scripts/stable_release_gate.sh candidate`: it repeats
strict evidence but intentionally does not require that the release tag be
absent. The same evidence-reuse environment variable may be used when the
signed tag still satisfies the metadata-only equivalence policy. Pentest
readiness was already enforced by `release` before tagging.

For manual fallback, publish the core package first, wait until crates.io serves
the new version, then publish every companion in dependency order:

```sh
cargo publish -p base64-ng --dry-run
cargo publish -p base64-ng
cargo publish -p base64-ng-imap --dry-run
cargo publish -p base64-ng-imap
cargo publish -p base64-ng-mime --dry-run
cargo publish -p base64-ng-mime
cargo publish -p base64-ng-multibase --dry-run
cargo publish -p base64-ng-multibase
cargo publish -p base64-ng-password --dry-run
cargo publish -p base64-ng-password
cargo publish -p base64-ng-openpgp --dry-run
cargo publish -p base64-ng-openpgp
cargo publish -p base64-ng-pem --dry-run
cargo publish -p base64-ng-pem
cargo package -p base64-ng-sanitization
cargo publish -p base64-ng-sanitization --dry-run
cargo publish -p base64-ng-sanitization
cargo package -p base64-ng-derive
cargo publish -p base64-ng-derive --dry-run
cargo publish -p base64-ng-derive
cargo package -p base64-ng-serde
cargo publish -p base64-ng-serde --dry-run
cargo publish -p base64-ng-serde
cargo package -p base64-ng-bytes
cargo publish -p base64-ng-bytes --dry-run
cargo publish -p base64-ng-bytes
cargo package -p base64-ng-subtle
cargo publish -p base64-ng-subtle --dry-run
cargo publish -p base64-ng-subtle
cargo package -p base64-ng-tokio
cargo publish -p base64-ng-tokio --dry-run
cargo publish -p base64-ng-tokio
```

Publish the supported JavaScript package separately from the signed tag after
its exact tarball gate passes:

```sh
scripts/release_wasm_loader.sh check
scripts/release_wasm_loader.sh dry-run
scripts/release_wasm_loader.sh publish
```

Real npm publication always uses `npm publish --provenance`; provenance is not
an optional release-manager toggle. Configure npm trusted publishing in the
release environment. Never store an npm token in the repository. Both the npm
and crates.io publishers verify the signed tag against the exact SSH principal
and public key in `security/release-signers`; a signature that is merely valid
under a maintainer's ambient keyring is insufficient.

This order is required because companion crates depend on the same released
`base64-ng` version from crates.io while using a local path only during
repository development.

The publish sequence is intentionally kept out of
`scripts/stable_release_gate.sh`, because publishing updates the crates.io index
and requires release credentials. If publishing fails because of a crates.io or
credential issue after the tag exists, keep the tag and rerun the publish
helper. If publishing fails because the tagged source is wrong, do not move the
tag; cut a new patch release.

## Gate Modes

- `check` is the development gate. It may skip unavailable optional executors,
  but prints every skip and never treats generated dirty-tree artifacts as
  release evidence.
- `candidate` is the strict pre-seal evidence gate. Required tools, full
  campaigns, clean-source provenance, native backend evidence, and the final
  evidence index are mandatory.
- `release` enforces candidate requirements and additionally validates the
  final report-only commit. Expensive campaigns may be retained only through
  the explicit metadata-equivalence gate; no required outcome may be skipped.
