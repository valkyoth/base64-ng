# 2.1 Security Regression Checkpoint

Commit 23 adds verification coverage, not a new runtime algorithm or public
API. Ordinary decoding remains input-dependent and non-wiping. Secret/CT
validation, output gates, cleanup and protected-memory policies are unchanged.
Base64 is not encryption or authentication; callers still need message limits.

## Coverage

| Boundary | Checks |
| --- | --- |
| Strict table validator | Kani calls production `Family::validated_len` and the separate scalar validator for all byte values, lengths 0-8, both alphabets and padding modes; checks acceptance and exact decoded length |
| In-place compaction | Kani models one inductive chunk step over full `usize` input/read values, using the production chunk limit; proves aligned progress, output behind consumed input and bounded repair suffix |
| Borrowed proof reuse | Independent `base64` fuzz oracle for all four strict profiles, both validation policies, repeated destinations, short-buffer retry, misalignment and unchanged sentinels; bounded Miri retry and health-generation tests |
| Malformed input | Explicit interior-padding, high-bit and noncanonical-tail fixtures reject before capacity handling or any destination write; existing exhaustive scalar and vector tests cover all lanes and terminal values |
| SIMD boundary | Existing x86/NEON/RVV assembly/IR mutations reject missing reductions, disconnected results, wider reads and unexpected stores; native x86 guard pages and checked fault injection test real execution |
| Incremental and adapters | Existing fuzz-only scalar oracle compares output, progress and complete state; seeded bulk inputs cross routing thresholds; Tokio pending/cancellation/retry and exact-frame regressions retained |
| Companion isolation | Existing isolated consumer fixture matrix verifies feature closure and forwarding independently of workspace feature unification |
| Secret separation | Existing deterministic secret input-limit, fixed-work, withholding, wipe and cleanup tests rerun; new proof-access helpers are `cfg(kani)` only |

The in-place geometry harness is an arithmetic model, **not** a proof of the
production loop, memory copies, register clearing or SIMD instructions. It
assumes the existing vector writer returns a quad-aligned consumed length no
larger than the staged chunk. Native fault injection and Miri exercise the
real preserved-source and scalar-repair paths separately. The table proof is
bounded to two quanta; longer inputs remain covered by differential tests and
fuzzing, not an unbounded theorem.

Miri exercises scalar/fallback behavior and Rust borrowing, not native SIMD.
Generated-code mutations test the checkers, not hardware. Native guard tests
only cover the instructions available on the host. No result here admits new
AVX-512, ARM, Windows, WASM or RISC-V hardware behavior.

## Commands

```sh
sh scripts/check-2.1-security.sh
sh scripts/check-2.1-security.sh --extended
```

The default gate runs the active/MSRV deterministic and mutation checks with
toolchain-separated caches. It belongs to the deferred development CI suite,
not ordinary push CI. `--extended` additionally requires installed nightly
Miri/rust-src, cargo-fuzz and compatible Kani (invoked via Rust 1.90.0), plus
native x86_64 Linux for ASan. Missing tools fail; no installer or silent skip
turns absence into a pass.

Miri uses its own target cache with `CARGO_BUILD_BUILD_DIR` unset and
`CARGO_INCREMENTAL=0`. This avoids runner incremental-lock failures on its
read-only execution filesystem. These settings are scoped to Miri; subsequent
ASan and fuzz commands retain their separate target and build directories.
Routing mutation tests reject either missing Miri environment control, but
real Miri execution remains necessary to verify compiler/runner compatibility.

Extended checks run two selected Kani harnesses with five-minute per-harness
timeouts and an 8 GiB virtual-memory limit. They run the borrowed-view and
in-place Miri cases, the public decode/view/incremental/in-place tests under
ASan, and 1,000 libFuzzer executions each for `in_place` and `v2_incremental`.
The smoke runner uses a private temporary corpus with explicit 4 KiB valid,
high-bit and tail-bit seeds plus the committed incremental bulk seeds. It
does not modify the retained corpus or release evidence manifests. A crash
remains a failure and libFuzzer retains its usual crash artifact.

These are development checks, not an hour-long fuzz campaign or signed release
evidence. Full release campaigns, native target checks and independent review
remain separate gates. The historical 2.0 proof results do not count as fresh
2.1 results; the two new harnesses are also included in the normal release
inventory for the final candidate run.

## Local Results

On 2026-10-06, the complete extended gate passed on native x86_64 Linux:

- Rust 1.99.0 and MSRV 1.90.0: core-only/plain-SIMD/checked integration
  regressions, native guard pages, checked fault recovery, secret cleanup,
  Tokio bulk cancellation/framing, fuzz-library regressions and Clippy passed.
- Isolated companion consumers passed both toolchains' feature matrices.
- Nightly Miri (`c36f145719`, 2026-10-01): the bounded borrowed-view retry,
  both retained-health tests and staged in-place repair passed (four tests).
- Kani 0.68.0 / CBMC 6.11.0: table refinement passed all 774 checks (nine
  unreachable) in about 140 seconds; the shared-limit chunk model passed in
  under one second. Both ran inside the configured limits. Whole-crate
  unsupported-construct warnings were unreachable in these harnesses.
- AddressSanitizer: all 13 selected integration tests passed.
- ASan/libFuzzer: `in_place` completed 1,000 runs in 47 seconds with peak RSS
  417 MiB; `v2_incremental` completed 1,000 runs in 3 seconds with peak RSS
  138 MiB. No crash, sanitizer error or oracle mismatch occurred.

Full all-feature release workspace tests and all-target workspace Clippy passed.
Public API snapshots are unchanged. Production LLVM IR contains neither the
new Kani accessors nor the fuzz-only scalar oracle. Gate routing/failure,
formatting, metadata, line-budget, unsafe, panic and constant-time policy checks
also passed. No manifests or lockfiles changed.

These local results are not independent review or final-release evidence;
external pentest and CI remain pending.

### Miri Runner Follow-Up

The follow-up review identified an incremental-lock failure with the custom
Miri build directory. After unsetting that directory and disabling incremental
compilation in a Miri-only subshell, the complete extended gate passed again
on 2026-10-06: all four real Miri tests, both bounded Kani proofs, 13 ASan
integration tests and both 1,000-run fuzz smokes. Five gate regression tests
also passed, including inherited-environment handling, mutations removing
either Miri control, and restoration of subsequent tools' build settings.
This fix changes only tooling and documentation, not runtime code or dependencies.
