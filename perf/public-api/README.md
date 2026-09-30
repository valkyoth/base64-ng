# Public API Baseline (2.1)

This unpublished, isolated harness compares complete ordinary public operations
with signed 2.0.4. It does not change production code or add runtime dependencies
to the core crate. The older `perf/` harness remains the retained exact-backend
admission tool; this workspace adds feature-isolated caller-path comparisons.

## Run

Requires Linux, an unprivileged user, Python 3.12+, Git history containing signed `v2.0.4`, native
Rust 1.98.1 (the root pin), and the lockfile dependencies in the Cargo cache.
Bubblewrap with `--size` and `--disable-userns` support, enabled unprivileged user
namespaces, `prlimit`, a system C linker, and a running systemd user manager with
cgroup-v2 CPU/memory/PID delegation are required. A sandbox probe fails
before building if isolation is unavailable; there is no unsandboxed fallback.
The paired runner currently does not run on macOS or Windows. Native evidence
on those systems requires a separately reviewed runner, not disabling isolation.
An administrator can configure the user service with a `[Service]` drop-in
containing `Delegate=cpu memory pids` (see `scripts/ci-benchmark-delegation.conf`).
Apply it before starting the user manager on a disposable benchmark host; do not
restart an active desktop user manager just to run this tool. CI uses a `/run`
drop-in that does not persist across reboot.
On Ubuntu, the launcher also needs an AppArmor user-namespace allowance; CI uses
the binary-specific `scripts/ci-bwrap.apparmor`, not a global AppArmor disable.
Fetch dependencies once with `cargo fetch --locked --manifest-path
perf/public-api/Cargo.toml`. Measurements subsequently build offline.

```sh
# Fast correctness/schema checks, including all six feature configurations:
sh scripts/check-2.1-public-api.sh

# Paired baseline, seven alternating-order samples, current committed HEAD:
python3 scripts/compare-2.1-public-api.py \
  --output target/release-evidence/public-api-2.1

# Smaller end-to-end tooling smoke, not a performance conclusion:
python3 scripts/compare-2.1-public-api.py --smoke --samples 2 \
  --features core adapters --output target/release-evidence/public-api-smoke

# Expanded feature/size/pattern matrix, still not the long fuzz/release gate:
python3 scripts/compare-2.1-public-api.py --full \
  --features core alloc default simd checked adapters \
  --output target/release-evidence/public-api-full
```

Use a fresh output directory each time. Keep the host idle during measurement;
do not run correctness builds or other benchmarks concurrently. Do not compare
absolute results collected on different hosts. `--candidate REV` selects a
committed candidate; baseline is fixed to
`816da2e1e4a66c913057c86d149068f1c88776bf`. Both are unpacked into temporary trees
and built with the same current harness and compiler. `--allow-dirty-harness`
is explicitly diagnostic, records that state, and does not benchmark uncommitted
production changes. Harness hashes include the independent oracle and allocator.
No source checkout, CPU tuning, or persistent host configuration is modified by
the runner. Administrator prerequisite setup is separate.

## Candidate Execution Boundary

Only use a reviewed copy of these Python scripts and harness. Candidate Rust
code (including build scripts) is untrusted to the runner. Compilation and every
binary invocation run in separate Bubblewrap PID/user/network namespaces with
capabilities dropped and nested user namespaces disabled. Source, system tools,
the selected Rust toolchain and registry cache are read-only. Host home, SSH/AWS
credentials, agents, repository, and evidence output are not mounted. Treat the
shared compiler and registry cache as trusted, non-confidential build inputs;
their contents are visible to a build. This is Linux process isolation, not a VM
or a guarantee against host-kernel vulnerabilities.

No caller environment variables reach builds/executables. The complete fixed
build/runtime environments, tool paths and compiler/Cargo/linker/launcher hashes
are recorded. Ambient wrappers, Cargo configuration in the host home, loader
injection variables, proxies, and target/profile flags cannot silently alter a
capture. Candidate manifests remain source-bound inputs. The lockfile is adapted
only for local crate versions, without re-resolving external packages.
The launcher inventory includes Bubblewrap, prlimit, systemd-run, systemctl and
Python. Recorded aggregate budgets use the same policy constants as enforcement.

Builds use a 1 GiB target/home tmpfs; executions use 64 MiB. A separate 64 MiB
temporary filesystem is provided. Per-process limits are 4 GiB virtual address
space for builds / 512 MiB for execution, 600 CPU seconds, 128 processes per UID,
256 file descriptors, 64 MiB files, and no cores. Each invocation additionally
uses a transient cgroup-v2 scope: 6 GiB aggregate build memory / 1 GiB execution
memory, zero swap, 128 tasks and a 200% CPU quota (two cores of aggregate CPU
time). Kernel control files are verified before launching candidate code; missing
controllers or limits fail closed. Baseline and candidate use identical budgets;
CPU throttling is part of this exploratory measurement environment. Both pipes
are drained incrementally: measurement stdout
and runtime stderr have 64 KiB limits, diagnostics 8 MiB, exported executables
64 MiB, and compiler stderr 4 MiB. A 600-second wall timeout or overflow kills
the process group; the PID namespace also disposes of detached descendants.
Only the bounded executable/diagnostics are exported by the trusted parent.
Git archives reject links, traversal and special files and are limited to 128 MiB
and 10,000 members, checked before extraction.

## Coverage And Interpretation

The candidate-only `validation-policy` feature adds `historical-reference` and
`canonical-reference` rows using the explicit scalar-reference decode option.
Enable it for standalone current-tree measurements, not the transplanted 2.0.4
paired runner (that release predates the option). Encoding in these rows is
unchanged. Canonical `Auto` now uses specialized portable scalar checks for
strict Standard/URL-safe settings; `ScalarReference` retains the original
validator. Historical policies still share their existing validation paths.

- Four strict Standard/URL-safe padded/unpadded presets; historical slice,
  canonical slice, validation-only, owned allocation, reusable append, in-place,
  incremental, real sync writer, Tokio writer, and Bytes state adapters.
- Exact available backends and exact-pinned `base64 0.23.0` / `base64ct 1.8.3`
  comparators. They are comparison tools, not dependencies of the shipped core.
  Local path requirements span the baseline and candidate versions; source
  commits, not a registry's latest compatible release, select those crates.
- Core-only, alloc-only, default alloc/std, std/SIMD, SIMD/checked, and
  SIMD/stream/adapters builds are separate binaries. The measurement executable
  itself uses std, including when the library under test is no_std/core-only.
  Default does not enable SIMD or the optional stream feature.
- Existing historical/vector routes retain scalar prevalidation at this
  checkpoint. `scalar` means full scalar execution; it is **not** a simulated
  `ScalarReference` selector. Use the explicit `*-reference` rows for that
  policy. Checked and exact-backend rows
  must not be described as interchangeable execution contracts.
- Boundary-sized, empty, small and 64 KiB payloads; `--full` adds SIMD-boundary
  neighbors, 1 MiB, all-zero, and structured byte patterns. Pseudorandom bytes
  are reproducible test data generated at runtime, not secret randomness.
- Malformed beginning, block-boundary and final-symbol inputs are latency-only.
  Detailed error formatting is outside the timing path. Separate diagnostic
  fixtures compare error variants, positions, precedence and full destination
  mutation against 2.0.4, including simultaneous grammar and capacity failures.
- The independent RFC oracle supplies expected bytes. Success/error count is
  checked for every timed call; output is checked before and after warm timing,
  including the final timed result. Mutations test incorrect output, error-only
  calls, accepted malformed input, bad schemas, missing pairs and noisy samples.
- Cold rows use one codec call per new process with setup/process launch excluded.
  No backend discovery or report is requested before that timer. Warm rows run
  a verified operation first. This separates first-call overhead without claiming
  to isolate KAT cost from every other first-call effect.
- In-place rows include restoring input; owned rows include replacing/dropping
  the previous owned output. Append and adapter output allocations are reused.
  Incremental/adapter construction and completion are timed; Tokio runtime
  construction is not. Short writes/fragments of 1 and 7 bytes are separate rows.
  Tokio uses a current-thread executor and ready short writes: these are throughput
  tests, not cancellation/backpressure safety proofs or network latency tests.
- Allocation counts include alloc/realloc/zeroed calls, not live bytes or peak
  memory. The existing observational allocator is reused. Dispatch fields are
  **capability reports**, not claims about the backend used for a short call or
  canonical/adapter operation. Exact-backend rows name the requested backend.

## Artifacts And Thresholds

`manifest.json` records commits, compiler, host, feature sets, command, harness
and executable digests. The two lockfiles retain exact comparator versions and
registry checksums; mismatched external resolution fails. `samples.jsonl` retains
paired raw nanoseconds, operation count, both byte lengths, and allocations.
`summary.json` reports ns/operation and both **decoded payload** and **encoded
representation** GiB/s for successful operations (including validation-only
scan rates, not output generation). Rejection rows have no throughput values.
Diagnostic transcripts and `SHA256SUMS.json` complete the local bundle. A failed
capture leaves partial files but no completed checksum inventory. Keep the bundle
outside Git under `target/release-evidence/` or archive it before cleaning target.

Frozen exploratory criteria: at least seven paired samples, median paired
latency ratio outside a 5% band, relative median absolute deviation at most 10%
on both sides, and a one-sided paired sign test at p <= 0.05. Fewer samples,
noise, or inconsistent wins cannot establish improvement. Signals are **review
prompts**, not automatic admission: the many rows are not corrected for multiple
comparisons, short timings include loop/clock/instrumentation overhead, and the
runner does not pin CPUs or control host load/frequency. Reproduce suspected
changes with longer measurements on the same idle hardware. A completed run
means semantic checks passed and measurements were collected, not that every
row improved or that all possible regressions were excluded. CI runs deterministic
correctness/schema tests, not performance thresholds or expensive campaigns.
