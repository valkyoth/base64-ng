# Fuzzing Policy

`base64-ng` keeps fuzzing isolated from the published crate. The root crate
remains dependency-free; fuzz-only dependencies live under `fuzz/` and are
checked by the standard local gate while remaining outside the published crate.

Run fuzz harness checks with:

```sh
scripts/check_fuzz.sh
```

Run corpus policy checks directly with:

```sh
scripts/check_fuzz_corpus.sh
```

## Targets

Current fuzz targets:

- `decode`: arbitrary strict, strict line-wrapped, legacy, and
  constant-time-oriented decode input plus wrapped encode slice/alloc
  equivalence; it also derives malformed cases from canonical encodings to
  exercise invalid-byte positions, mixed alphabets, early padding,
  non-canonical trailing bits, and clear-tail error behavior
- `in_place`: in-place encode/decode, legacy compaction behavior, and strict
  line-wrapped in-place compaction behavior
- `stream_chunks`: fragmented stream reader/writer state machines, adjacent
  framed payload boundaries, and stream state-helper invariants
- `differential`: canonical output comparison against the established Base64
  behavior used by the harness, plus static RFC 4648 ground-truth vectors so
  the differential oracle is not the only source of truth
- `x86_encode`: forced SSSE3/SSE4.1 and AVX2 Standard and URL-safe encode
  comparison against the independent `base64` oracle on hosts that report the
  complete runtime feature bundle; input is capped at 64 KiB per iteration
- `x86_decode`: forced SSSE3/SSE4.1 and AVX2 Standard and URL-safe strict
  decode comparison against the scalar public contract, including exact error
  equality and rejected-output retention; canonical input is also checked
  against the independent `base64` oracle
- `neon`: forced little-endian AArch64 NEON Standard and URL-safe encode and
  strict-decode comparison against the scalar public contract and independent
  canonical oracle; input is capped at 64 KiB and the target is a no-op on
  non-AArch64 fuzz hosts
- `mime_body`, `pem_document`, `multibase_family`, `imap_payload`,
  `password_records`, and `openpgp_armor`: bounded complete-protocol parsing,
  generation, incremental partitioning, canonical regeneration, and malformed
  input checks for every 2.0 protocol companion
- `v2_runtime_codec`: runtime-alphabet construction and rejection, every
  runtime padding/trailing-bit policy combination, one-shot, in-place, append,
  allocation-limit, and transactional-output behavior against an independent
  encoder
- `v2_incremental`: arbitrary encoder/decoder partitions, absorbing malformed
  state, legacy whitespace, WHATWG forgiving decode, short counted sinks, and
  formatter errors with exact confirmed progress; unwind-capable unit tests
  separately cover caller formatter panic propagation
- `v2_async`: manually polled Tokio reader and writer adapters under one-byte
  I/O, short writes, alternating `Pending`, shutdown, malformed failure, and
  cancellation-by-drop schedules
- `v2_assurance`: finite default-provider admission, invalidation,
  maintenance, retry, exhaustion, assured encode/decode, and every teardown
  fault stage through a fuzz-only unsafe provider whose assertions enforce
  ordering, exact-once effects, zero-before-teardown, quarantine, tombstoning,
  and claim separation

`cargo fuzz` executes with abort-on-panic behavior. Fuzz-target panics are
therefore reserved for violated invariants and are campaign failures;
deliberate caller-panic injection belongs in unwind-capable unit tests.
`scripts/check_fuzz.sh` rejects unwind-catching APIs inside fuzz targets so a
test stimulus cannot be mistaken for a recoverable fuzz input.

`tests/v2_fuzz_properties.rs` supplies deterministic exhaustive-small
properties for 64 runtime alphabet rotations, padded and unpadded policies,
and all small input/output partition combinations. The target-specific x86 and
NEON harnesses reach every runtime-supported admitted native backend directly.
Wasm `simd128` cannot be executed by the native libFuzzer process; its admitted
path remains covered by the dedicated Node, Wasmtime, Chromium, Firefox, and
Safari runtime/browser differential gates.

`scripts/check_fuzz.sh` also runs the fuzz workspace supply-chain gates:

```sh
cargo audit --file fuzz/Cargo.lock
scripts/cargo-deny-check.sh fuzz/Cargo.toml fuzz/deny.toml
```

The isolated `fuzz/deny.toml` permits the `libfuzzer-sys` license exception
needed by the harness. The published crate remains governed by the stricter
root `deny.toml`.

## Corpus Admission

Committed corpus inputs are allowed only under:

- `fuzz/corpus/decode/`
- `fuzz/corpus/in_place/`
- `fuzz/corpus/stream_chunks/`
- `fuzz/corpus/differential/`
- `fuzz/corpus/x86_encode/`
- `fuzz/corpus/x86_decode/`
- `fuzz/corpus/neon/`
- one directory matching each remaining target name listed above

Each committed corpus input must be:

- small enough to review manually, with a hard local limit of 64 KiB
- relevant to a previously fixed bug, a protocol boundary, or an edge case not
  already represented by deterministic tests
- non-sensitive and safe to publish
- named or documented well enough that reviewers can understand why it exists

Generated crashes, hangs, and local artifacts must stay out of commits. The
release gate rejects files under `fuzz/artifacts/` other than `.gitignore`.

## Running Local Campaigns

Install nightly and cargo-fuzz:

```sh
rustup toolchain install nightly
cargo install --locked cargo-fuzz --version 0.13.2
```

Run bounded smoke campaigns before release-sensitive stream or decode changes:

```sh
BASE64_NG_RUN_FUZZ_SMOKE=1 scripts/check_fuzz.sh
```

Use `BASE64_NG_FUZZ_RUNS=<n>` to change the per-target run count. The default
is `1000` runs for each target.

Run the release-duration campaign with:

```sh
BASE64_NG_RUN_FUZZ_RELEASE=1 \
BASE64_NG_FUZZ_SECONDS_PER_TARGET=3600 \
scripts/check_fuzz.sh
```

The same campaign may be distributed across machines without weakening its
one-hour-per-target policy. Every worker must use the same clean Git commit.
The recommended operator interface is the persistent session manager:

```sh
scripts/manage-fuzz-evidence.py
```

Its numbered menu records the exact source identity, all 18 fuzz target states,
and a separate native RISC-V admission job in
`target/fuzz-manager/state.sqlite3`. On a later invocation it offers to continue
that session or start a new one. A target can run locally or over SSH. Local
workers remain detached after the menu exits, with an atomic lock preventing a
second local campaign. Remote setup clones the exact session commit, reuses the
pinned Rust toolchain when it is already installed or installs only its minimal
profile otherwise, installs the pinned `cargo-fuzz` version for fuzz jobs,
starts a detached worker, and records its host, SSH port, PID, work directory,
and start time. If rustup is absent,
the official TLS bootstrap is used only after explicit operator approval. The
manager separately checks for the C compiler and command-line tools needed to
build `cargo-fuzz`. With explicit approval, it installs a fixed prerequisite
set through `apt-get`, `dnf`, `yum`, `zypper`, or `apk`, using root directly or
passwordless `sudo`. It rechecks every command and fails before cloning or
starting a campaign if the host remains incomplete.

Selecting a running target checks its persisted status. Successful remote
evidence is copied back and passed through the exact-source shard validator
before the menu marks it complete; the remote machine can then be terminated.
Failed and interrupted jobs remain visible and can be retried. One remote host
and the coordinator can each run at most one managed target at a time. The
private key is never copied or read by the manager: only its local path is
stored in the ignored SQLite database. Set reusable defaults without modifying
the repository:

```sh
export BASE64_NG_FUZZ_SSH_USER=ubuntu
export BASE64_NG_FUZZ_SSH_PORT=22
export BASE64_NG_FUZZ_SSH_KEY="$HOME/.ssh/fuzz-worker.pem"
```

The RISC-V job runs `scripts/capture-2.0-riscv-admission.sh` rather than
LibFuzzer, reuses the exact installed project toolchain without installing
nightly or `cargo-fuzz`, and stores its independently validated bundle outside
the fuzz shard directory. When all fuzz targets and that hardware job are
locally verified, the menu exposes final aggregation of the exact 18 fuzz
shards.
Headless status and finalization are also available:

```sh
scripts/manage-fuzz-evidence.py --status
scripts/manage-fuzz-evidence.py --finalize
```

Starting a new session replaces only the active SQLite state. Prior ignored
session directories are preserved for operator-controlled retention. A reset is
refused while jobs are marked running. SSH uses accept-new host-key checking
with a manager-owned ignored file under `target/fuzz-manager/`. Before a new
job, any stale entry for the selected ephemeral cloud IP is removed from that
file; the newly observed key is then pinned for all progress checks and downloads
during the job. The manager does not alter `~/.ssh/known_hosts`.
High-assurance operators should verify each new worker's host-key fingerprint
out of band before launching evidence.

The lower-level per-target interface remains available for manual orchestration.
Capture one target into an ignored collection with:

```sh
BASE64_NG_FUZZ_MACHINE_LABEL=worker-name \
scripts/capture-fuzz-shard.sh decode target/fuzz-shards 3600
```

The capture script refuses unknown targets, dirty source, durations below one
hour, pre-existing output, crash artifacts, missing final LibFuzzer statistics,
and source changes during execution. `x86_encode` and `x86_decode` additionally
require native AVX-512F, BW, VL, and VBMI; `neon` requires native
little-endian AArch64. All other targets are portable native-host campaigns.

Copy each completed target directory to one coordinator's collection using an
integrity-preserving transport such as `rsync -a`. Workers may run at different
times and in any order. Inspect resumable progress and build the compatible
release manifest with:

```sh
scripts/check-fuzz-shard-progress.sh target/fuzz-shards
scripts/aggregate-fuzz-shards.sh target/fuzz-shards
```

Aggregation requires all 18 targets exactly once and binds every bundle to the
coordinator's current commit, Git tree, lockfiles, fuzz manifest, and individual
harness hash. It rechecks timing, architecture, zero artifacts, corpus archive,
environment, and campaign-log hashes before writing
`target/release-evidence/fuzz/MANIFEST.txt` atomically. Missing, duplicate-like,
unknown, altered, shortened, dirty, mixed-commit, or wrong-architecture evidence
fails closed.

After all bundles validate, import them into the strict candidate gate instead
of rerunning the monolithic campaign:

```sh
BASE64_NG_FUZZ_SHARD_DIR=target/fuzz-shards \
scripts/stable_release_gate.sh candidate
```

The release mode defaults to one hour per target. Both modes require zero
crash artifacts and record tool identities, parameters, final LibFuzzer
statistics, corpus counts and hashes, output hashes, and minimization status in
`MANIFEST.txt`. Generated corpus changes must be reviewed deliberately. Keep
only non-sensitive inputs that improve coverage or preserve a regression.

Opt-in smoke campaigns write release evidence under:

```text
target/release-evidence/fuzz/
```

Expected files:

- `decode.txt`
- `in_place.txt`
- `stream_chunks.txt`
- `differential.txt`
- `profiles.txt`
- `x86_encode.txt`
- `x86_decode.txt`
- `neon.txt`
- one `<target>.txt` file for each of the 18 targets
- `MANIFEST.txt`

Smoke campaigns use temporary corpus and artifact directories under
`target/release-evidence/fuzz/` so ordinary release smoke runs do not leave
generated files under committed `fuzz/corpus/` or `fuzz/artifacts/`.

## Assurance Scope

Base 2.0 has no persistent teardown provider. The assurance target therefore
models one volatile provider-instance generation and treats generation
termination as final: it exposes no restart, import, or resume parser that
could honestly be fuzzed. A future persistent provider must add a separate
authenticated parser and rollback, replay, corruption, torn-write,
compaction, and capacity campaign before making a persistence claim.

The fuzz-only unsafe provider is not a supported extension example. It exists
to reject invalid hook order and lifecycle claims. A deliberately unwinding
unsafe provider is exercised only in an isolated subprocess because unwinding
from those teardown hooks violates their unsafe contract and may abort during
double-panic cleanup.
