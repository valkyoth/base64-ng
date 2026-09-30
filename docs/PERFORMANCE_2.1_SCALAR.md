# Commit 5 Portable Scalar Measurements

This is a bounded development experiment, not release admission, a comparison
with other libraries, or a general hardware performance guarantee.

## Scope And Method

On 2026-09-30, compare accepted Commit 4
`ed85af23c126f06ea2c91d4cd94d34833dca7b44` with the Commit 5 working tree on an
AMD Ryzen 9 9950X3D, Linux x86_64, Rust 1.98.1 / LLVM 22.1.8. Use the unchanged
`perf/public-api` harness, release mode, `validation-policy`, and
`RUSTFLAGS='--cfg base64_ng_perf_evidence'`. Build separately with default
features and `--no-default-features`. Neither library configuration enables
SIMD. The harness itself uses std even for the core-only library.

Measure `canonical decode` for profiles `sp`, `su`, `up`, `uu`, with random
payload lengths 0, 3, 32, 1024 and 65536. Use seven paired samples, alternating
before/after order. Each invocation uses warm mode, fragment size
`max(payload_length, 1)`, and 100000 iterations for lengths up to 32, 200 for
1024, or 20 for 65536. For example:

```sh
target/commit5-after-default/release/base64-ng-public-api-perf \
    canonical decode sp 65536 random 65536 20 warm
```

The harness verifies bytes against an independent RFC oracle before and after
timing and checks every call's success/length. Its `diagnostics` output from
both binaries was byte-identical to the retained 2.0.4 transcripts for each
feature configuration, including errors and whole-destination mutations.
This local experiment executes reviewed before/after binaries directly; it
does not replace the sandboxed revision-comparison runner or its provenance.

## Results

Selected default-feature Standard padded results, median nanoseconds per call:

| Payload bytes | Commit 4 | Commit 5 |
| --- | ---: | ---: |
| 0 | 21.92 | 11.99 |
| 3 | 106.84 | 26.37 |
| 32 | 983.41 | 35.67 |
| 1024 | 29391.99 | 419.84 |
| 65536 | 3127344.60 | 25662.35 |

All 20 default-feature cases showed an improvement signal under the existing
paired-sample classifier. Core-only had 18 improvement signals and two noisy
cases (URL-safe padded lengths 0 and 1024); treat those two as inconclusive.
No case produced a regression signal. All measured decode calls allocated zero
heap blocks. A preliminary run exposed empty-input overhead; an empty fast path
was added and the complete matrix was repeated, producing the results above.

The large relative change removes per-symbol linear alphabet search and the
three-byte scratch state-machine validation loop on this specific API. It is
**not** an improvement claim for historical `Engine` decoding, incremental
states, custom/relaxed codecs, CT/secret APIs, or all workloads. Rejected inputs
still rerun the original validator for diagnostics and are not covered by this
successful-input throughput comparison. Reference validation remains available.

## Footprint

`size` on the complete measurement executables (not isolated library size):

| Configuration | Before text bytes | After text bytes | Before total | After total |
| --- | ---: | ---: | ---: | ---: |
| Default | 529802 | 526090 | 550852 | 546756 |
| Core-only | 511470 | 508210 | 530360 | 530380 |

The lookup tables occupy 512 static bytes. No input-sized stack or heap storage
is introduced. In default-build disassembly, `ordinary_decode::prepare` reserves
120 stack bytes plus six saved registers, versus 232 bytes plus six registers
before. This is only its direct frame: it excludes return addresses and callees,
and the original validator is now a separate fallback function. It is not a
whole-call-graph stack bound or a cross-target assembly guarantee.

## Local Artifacts

The capture retains source/binary hashes, compiler/CPU identity, diagnostic
hashes, all 560 samples, and classifications under
`target/release-evidence/2.1-commit5-portable-final/`. The preliminary capture is
retained separately under `2.1-commit5-portable/`; these local files are not
published evidence bundles. Final executable SHA-256 values:

```text
core before    e925fccfe3018883c45d844ceda99e87a18618f5b2d489ed3dbffbb4cba025fe
core after     5e49c105519468fbdf5e2588849a81053d68f4c9f841081875dc18ab1f99218e
default before 52fc278fd8089d97cd1714e944d823afde1b7aa053915bb5acdaf6a563069e4f
default after  868666c420a84035accd05d154ba02a083c0a8a57fbd0225e1de60b2a58a557a
```
