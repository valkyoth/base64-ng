# Companion Acceleration Features

Commit 20 adds opt-in feature forwarding, not a new codec or public Rust API.
All defaults, dependency versions, grammars, finite limits and secret contracts
remain unchanged. These features are development-source additions for 2.1.0,
not features of the currently published 2.0.x companions.

| Companion | Forwarded features | Scope and limitations |
| --- | --- | --- |
| `base64-ng-bytes` | `simd`, `checked-backend` | Ordinary owned and eligible incremental blocks; always requires alloc; std remains its default |
| `base64-ng-tokio` | `simd`, `checked-backend` | Ordinary read-all and streaming helpers; already requires std/alloc |
| `base64-ng-serde` | `simd`, `checked-backend` | Ordinary owned/bounded adapters; alloc is the default, std is explicit; secret adapters unchanged |
| `base64-ng-multibase` | `simd`, `checked-backend` | The four strict Standard/URL-safe Base64-family codecs; minimal build remains no_std/no-alloc |
| `base64-ng-pem` | `simd`, `checked-backend` | Ordinary collected body transforms after document checks; always requires alloc; secret parsing unchanged |
| `base64-ng-openpgp` | `simd`, `checked-backend` | Ordinary collected body transforms after armor/checksum checks; always requires alloc; secret parsing unchanged |
| `base64-ng-mime` | Neither | Current wrapping/transport state machines process scalar quanta, so forwarding would not accelerate their transforms |
| `base64-ng-imap`, `base64-ng-password` | Neither | Dedicated alphabets are not admitted Standard/URL-safe SIMD profiles |
| `base64-ng-subtle`, `base64-ng-sanitization` | Neither | Secret comparison/protected memory, not ordinary bulk transformation |
| `base64-ng-derive` | Neither | Compile-time macro, no runtime codec |

Every forwarded `checked-backend` implies both the companion's `simd` and the
core's redundant checking. It does not disable validation or change failure
semantics. No forwarding feature adds std, alloc or secrets beyond the existing
companion requirements. Cargo features remain additive across consumers.

With core std, CPU detection and existing health/length gates select an admitted
backend. A no_std SIMD build needs complete static ISA features; unsupported
targets and ineligible input retain scalar fallback. `simd` alone is therefore
not a guarantee of acceleration. Serde consumers wanting runtime detection
should opt into `std` as well. Tiny fragments and custom/relaxed codecs can
remain scalar even when a companion enables SIMD.

Examples beside each companion's existing usage examples show a path dependency
against a 2.1 checkout. Consumers no longer need a direct core dependency merely
to switch on acceleration. Bytes and Tokio examples still name core codec types,
so they need the usual core dependency for those types, with no extra feature
flags. Existing secret APIs never enter an ordinary route because SIMD was
enabled elsewhere in the dependency graph. Ordinary APIs are not constant-time
or secret containers, including in checked builds.

## Verification

`sh scripts/check-2.1-companion-features.sh` runs the focused local/full-release
gate. Each consumer is a fresh standalone Cargo workspace containing one
companion, not a member of the root workspace. A diagnostic core dependency
has defaults disabled and enables no features. `cargo metadata` must report
exactly the core feature closure requested by that companion. Resolved registry
packages/checksums must be present in the existing root lockfile.

The active compiler and MSRV 1.90 run default, minimal, SIMD, checked,
default-plus-SIMD/checked, and explicit std-plus-SIMD/checked variants. Large companion round trips,
malformed input and destination preservation are tested where the API supports
them. Serde without alloc has no transforming API, so that minimal combination
is a build/feature check, not a claimed decode execution test. No_std variants
also compile for `thumbv7em-none-eabihf`; Tokio is explicitly std-only.

The gate runs each companion's complete tests (including secret-feature tests)
and Clippy independently, plus existing shared encode/decode fault-injection
tests. Feature closure connects these tests to the same checked kernels; it
does not claim injected faults were exercised through every companion wrapper.

On a capable native host, compiler coverage separately proves checked ordinary
kernel execution through all six consumer fixtures, without production hooks:

```sh
rustup component add llvm-tools-preview --toolchain 1.99.0
python3 scripts/check-companion-features.py --toolchain 1.99.0 --coverage
```

This instrumented functional check fails if checked kernels are not reached;
it is not performance evidence. It passed locally on x86_64 for all six
companions. Shared fault tests cover rejection, quarantine and scalar repair.
Default/minimal builds exercise fallback. Native ARM/Windows/RVV release
campaigns and external review remain separate release tasks.

Local checks on 2026-10-05 passed the full focused active/MSRV gate, the native
coverage check for all six companions, full release workspace tests, workspace
Clippy, rustdoc with warnings denied, unchanged API snapshots, feature/CI
mutation tests, formatting and release metadata. Package file-list smoke and
the checked Tokio transfer example are part of the focused gate. Package
publication remains blocked during 2.1 development; file-list smoke is not a
registry publication dry run. No runtime Rust implementation or lockfile changed.

The Commit 21 follow-up isolates workspace commands by compiler under
`target/companion-workspace/<toolchain>` and sets the intermediate build
directory there too. Isolated consumer fixtures retain their own per-toolchain
target and intermediate directories. This prevents the two-toolchain gate from
leaving incompatible Serde metadata in the normal workspace cache; regression
tests run the shell dispatcher with inherited directory overrides and verify
every Cargo invocation plus failure propagation. Existing old caches are not
deleted by the gate.

## CI During Development

Routine pushes and pull requests continue running core checks, native platform
tests, MSRV/target compilation, Miri, sanitizers and independent CodeQL.
Cheap forwarding-contract and mutation tests remain in core checks. The full
2.1 feature matrix and the complete big-endian, RISC-V and SVE QEMU suites are
now opt-in jobs: they do not run on ordinary pushes, PRs or a schedule. Splitting
jobs alone did not reduce the previous hour-long emulated workload.

For the final 2.1 commit, dispatch the complete workflow explicitly:

```sh
gh workflow run ci.yml --ref main -f full_release=true
```

Check that the run's `headSha` is the final commit and that **all four deferred
jobs executed and passed**, not skipped. An ordinary green push is not full
release verification. The same scripts remain available for targeted local
testing before intermediate commits; do not run every exhaustive suite for
unrelated changes. No long fuzz campaign or native evidence requirement is
removed. Do not dispatch this full workflow until final release verification.
