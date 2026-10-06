# Native Windows 2.1 Verification

This checkpoint separates native compatibility, ABI observations and exploratory
performance from release admission. It does not enable a backend or change
ordinary/secret decoding contracts. GNU Windows, ARM64, WSL and Wine are not
covered by an x86_64 MSVC result.

## Prerequisites

Use native 64-bit Windows with PowerShell 7, Git, Rustup, the pinned toolchain
from `rust-toolchain.toml`, Rust 1.90.0, and Visual Studio C++ Build Tools with
the x64 MSVC toolset and Windows SDK. Verification also needs Python 3.11
or newer (`python` must resolve to the interpreter, not the Store alias).
The full workspace interoperability tests also require `openssl` on `PATH`;
Git for Windows provides it under its `usr/bin` directory.
Install the Rust `clippy` and `rustfmt` components for both toolchains. No WSL,
Bash, administrator session or Linux verification tools are required to run
the gate after prerequisites are installed.

Run from a trusted checkout on an otherwise idle host:

```powershell
pwsh -NoProfile -File scripts/test-windows-gate.ps1
pwsh -NoProfile -File scripts/check_windows.ps1 -Full
# Explicit native performance capture, never run by ordinary push CI:
pwsh -NoProfile -File scripts/check_windows.ps1 -Full -Benchmark
```

Without `-Full`, the additional Windows CI step runs focused plain/checked
public-API tests, six forwarding companion suites and the ABI fixture using
the pinned compiler. The existing Windows platform default/all-feature/core
tests remain in place. The full gate adds the MSRV, core/alloc/std combinations,
workspace tests, docs/examples, Clippy, verified core packaging, isolated
companion consumers, available static ISA builds and optimized assembly.
Native command failures stop the gate and retain a failed report.

## Evidence

Each run creates `target/windows-evidence/<unique-id>/report.json`, command
logs and toolchain-separated build outputs. It records OS/CPU information,
Rust versions, CPU/OS feature probes, HEAD, dirty status, source-file SHA-256s,
commands, outcomes and artifact hashes. Source hashes are checked again at the
end. Logs redact the checkout and home paths before hashing. This is an
unsigned development bundle, not an attestation of the host or its toolchain.
Keep the directory under a restrictive inherited Windows DACL; builds and
raw assembly may contain local filesystem paths and should not be published
without review. A dirty capture does not establish exact-commit provenance.

Missing AVX-512 or other instruction support is reported as unavailable, never
as executed coverage. Rust's runtime feature probes include the applicable OS
vector-state checks. Globally ISA-enabled fixture builds run only after the
baseline native probe reports that complete feature bundle. Use a host with
uniform CPU capabilities; do not move the process to incompatible CPUs.

`-Benchmark` reuses `perf/public-api`, its independent output oracle and the
existing paired statistics. It retains 15 alternated pairs per cell for all
four strict profiles, 32/768/65536/1048576-byte payloads, encode/decode,
historical/canonical public calls and available exact backends. Decode also
compares explicit scalar-reference validation with automatic validation.
Both payload and encoded GiB/s are derived from the existing byte accounting.
The scalar rows are same-source controls, not a comparison with 2.0.4; old/new
admission remains Commit 25 work. Noisy or regressing cells are retained, not
discarded or automatically admitted. Measurements are observations, not a CI
speed threshold.

## ABI Fixture And Unsafe Scope

`portability/windows_native` is an unpublished, dependency-free-except-for-core
test consumer, excluded from the shipped crate. Its Win64 callback probe seeds
distinct values in both halves of XMM6-XMM15 and verifies preservation across
ordinary automatic/reference and compiled static encode/decode operations,
including malformed-input returns and output sentinels. Plain and checked
builds run independently. Negative fixtures deliberately clobber each tested
register, and the checker must detect all ten corruptions.

The fixture's unsafe boundary consists of a temporary exclusive context-pointer
borrow, same-size integer/SIMD bit conversions and an inline assembly call.
The call reserves Win64's 32-byte shadow space, uses the compiler-guaranteed
stack alignment, restores `rsp`, declares ABI clobbers and every tested vector
output, and does not permit unwinding through assembly. Broken naked callbacks
are test-only and invoked solely through that explicit clobber boundary.
These rules follow the [Microsoft x64 ABI](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention)
and [Rust inline-assembly contract](https://doc.rust-lang.org/reference/inline-assembly.html).

The probe checks low-128-bit preservation, not upper YMM/ZMM preservation,
constant-time execution, secret erasure or exhaustive compiler correctness.
Running the Win64 callback fixture on Linux tests the fixture but is not a
native Windows result. Optimized native `.s` outputs must also be reviewed for
callee-save spill/restore and cleanup ordering under the
[SIMD checklist](SIMD_ACTIVATION_CHECKLIST.md). The automatic report deliberately
leaves assembly review pending rather than equating artifact generation with
review. Linux Miri/Kani/sanitizer/fuzz results are explicitly not Windows proof.

## Native Assembly Review

The 2026-10-06 native all-feature release outputs were inspected for Rust
1.99.0 and 1.90.0. Their SHA-256 digests are:

| Compiler | Assembly SHA-256 |
| --- | --- |
| 1.99.0 | `77837465309d5cbfdda42f3e8f4b281a306e0c7f1b07f4dab30bf67d62016df3` |
| 1.90.0 | `cefa24f2b2e5bae1158c613b5120e2551ab01f97ded49bb91ff948371baa308e` |

Review covered both alphabet instantiations of the SSSE3/AVX2/AVX-512
full-block encode/decode functions, the ordinary SSSE3/AVX2 writers, the
AVX2 classifiers and their ordinary validation wrapper. The inspected
callee-save slots hold entry register values, saved before processing input;
the loops do not repurpose those slots for input-derived intermediates.
Every inspected callee-save slot has a matching low-128-bit restore. Ordinary
writers and full-block encoders restore entry values after cleanup; classifier
leaves restore before the enclosing validation wrapper performs cleanup.
AVX transitions
include `vzeroupper`; AVX-512 cleanup explicitly clears ZMM0-ZMM31 before
restoring the required incoming XMM state. The ordinary writers and validation
wrapper join successful and rejected-input processing through cleanup.

This is a scoped inspection, not an all-stack or every-fault-path wiping
certificate. In particular:

- Rust 1.99 emits separate 16-byte stack argument copies in the ordinary
  SSSE3 validation wrapper. Those copies are not wiped by vector-register
  cleanup. Ordinary decoding has no secret-storage cleanup guarantee.
- The older scalar-prevalidated SSSE3/AVX2 `decode_full_blocks_*` helpers clear
  registers only after nonzero successful progress. Their first-block rejection
  branch does not establish cleanup. Canonical, immutable prevalidated input
  cannot normally take that branch with a correct classifier; this capture
  does not certify erasure after an internal classification fault. The new
  ordinary writers use a separate, unconditional cleanup join.
- Caller-owned incoming nonvolatile values must be restored for ABI correctness;
  preserving them is not evidence that the caller's earlier secrets were erased.

These observations do not change dispatch, the secret/CT APIs, or the release
admission policy. Repeat inspection for any newly admitted compiler/ISA/build
configuration; do not promote this compatibility result to a stronger retention
claim.

## Current Status

The native gate passed on 2026-10-06 with Windows Server 2025 Datacenter
10.0.26100, an Intel Xeon 6975P-C exposing two cores/four logical processors,
and both Rust 1.99.0 and 1.90.0. All 49 recorded commands succeeded, including
workspace Clippy/docs, verified packaging, the 48-mode companion matrix per
compiler, plain/checked ABI probes and all three available static ISA builds.
Both compilers detected SSSE3/SSE4.1, AVX2 and AVX-512 F/BW/VL/VBMI support.
The deliberately broken ABI fixtures detected corruption of each XMM6-XMM15
register. Existing optional external-tool interoperability skips are not
promoted to tool-specific coverage claims.

The [retained performance summary](evidence/windows-2.1/summary.json) summarizes
5,760 raw observations into 192 cells (15 alternating pairs per cell). Its
same-source comparisons produced 144 improvement signals, 11 regression
signals, 30 noisy cells, six within five percent and one inconclusive cell.
All timed allocation counts were zero. All 11 regression signals concern
32-byte historical or exact-backend comparisons against the scalar control;
they are not evidence of a regression from 2.0.4. For orientation only, the
1 MiB Standard-padded canonical decode versus scalar cell measured 4.74
payload GiB/s and a 22.96 paired median ratio. This does not admit any threshold
or establish a cross-platform performance claim; Commit 25 must evaluate the
complete admission matrix and retain these small-input tradeoffs.

The private raw bundle is retained locally under
`target/windows-evidence/native-20261006/`, including `evidence.zip` and its
extracted `capture/` directory. Command-log hashes, the retained assembly and
performance hashes, all sample summaries and source hashes were rechecked
after download. Executables/build caches were not copied into this bundle.

| Retained object | SHA-256 |
| --- | --- |
| `evidence.zip` | `708228f54f722ee71e0953f228624d88e75e7307bcc611a173748403e00b70fa` |
| `report.json` | `ec0e0653901233c316f1f485811bf78b7524fba2fb9ad0b549a15639d5a6fdde` |
| `performance.json` | `62071559ffedf546e25b36899aec58b6b2539665a5d76c8e9aff8a8b9ac64933` |

Source binding is base `bc4ad6a2ff0d17976b283c15c0746df11c1cd497` plus the
manifest's dirty-overlay file hashes, not a claim that the base commit contains
the new gate. Runtime source was unchanged. Subsequent local changes only
finalize this documentation/plan, retain the summary, and add the already
tested measurement-parser regression to the lightweight CI step. External
pentest and GitHub CI for this commit remain pending. Repeat the focused native
gate on the final release candidate; no Windows performance admission or
blanket cleanup certification is claimed here.
