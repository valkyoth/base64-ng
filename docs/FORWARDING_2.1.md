# 2.1 Owner and Forwarding Checkpoint

Commit 14 removes redundant complete-input validation from canonical decode
append and eligible historical owned decoding. It adds no public API, unsafe
code, dependency, new CPU admission, or weaker validation policy. External
review and CI acceptance of this checkpoint remain pending.

## Proof Reuse

`Base64::decode_append` now retains the private `Preflight` across fallible
reservation, resizing, and writing. The proof borrows the original immutable
source and owns its exact settings and checked output layout. Writing consumes
it without accepting replacement input or settings. Malformed input precedes
reservation; errors and unwinding restore the destination's original prefix
and length. Capacity growth and ordinary spare-capacity bytes are not rolled
back or wiped. This is not a secret-container contract.

`Engine::decode_vec_with_validation(Auto)` retains the same proof for exact
Standard/URL-safe tables and inputs longer than one encoded quantum. Short
calls retain the historical path. Rejection recovers historical diagnostics
before allocation; custom alphabets and `ScalarReference` keep the previous
implementation. The legacy vector allocator remains infallible; this does not
add the canonical API's returned allocation-error contract to `Engine`.

The shared writer rechecks backend health and retains checked comparison,
quarantine, and full scalar overwrite after rejected or corrupt backend output.
Reservation does not grant continued permission to execute a quarantined backend.

Canonical allocating and bounded decoders already retained one proof through
their write, so no extra implementation is added there. `Base64String` continues
to validate once per decode invocation before reservation, including after
parsing or adopting a string. It gains no cached proof, self-reference, or new
mutable access. Encode padding and decode acceptance remain distinct; the
sealed builder rejects incompatible combinations. The borrowed reusable view
is added separately in [Commit 16](BORROWED_BASE64_2.1.md), not this checkpoint.

## Routing Audit

The table describes decode routes after this checkpoint. Encoding integration
is recorded in [the Commit 13 checkpoint](PERFORMANCE_2.1_ENCODE.md). A row that
reaches the ordinary core uses acceleration only when its feature, settings,
size, CPU, and health gates qualify; forwarding is not a promise of SIMD.

| Public surface | Validation and write route | Preserved boundary / later work |
| --- | --- | --- |
| Canonical caller slice, owned Vec, output-limited Vec, runtime bounded array | One private proof, shared writer; custom/relaxed settings use reference validation | Full-input validation before capacity/limit/reservation; transactional caller output |
| Canonical decode append | Same proof retained across reservation | Prefix/length rollback, including unwind; no rollback of capacity |
| `Base64String` parse/adopt/decode | Validate before copy/adoption; each decode forwards to canonical owned decode | Adoption does not copy; no persistent validity cache or health snapshot; reusable borrowing in Commit 16 |
| Historical `Engine` owned Vec and top-level `decode` | Eligible Auto calls retain proof; tiny, custom, explicit reference paths unchanged | Historical errors and allocation behavior; `decode_secret` still wraps this ordinary result with cleanup, not CT semantics |
| Historical slice, clear-tail and `DecodedBuffer` | Existing eligible ordinary fast route, historical error fallback | Exact old error precedence/partial writes; clear-tail cleanup unchanged |
| Historical wrapped/legacy slice, Vec, profiles and buffers | Full grammar validator then indexed scratch chunks via historical backend | Original source indices, exact whitespace/line grammar, scratch cleanup; repeated grammar work remains deliberate |
| Canonical legacy whitespace and web forgiving decode | Dedicated incremental grammar used for measurement and writing | No strict proof is reused for a different grammar; WHATWG and whitespace rules unchanged |
| Ordinary canonical in-place | Complete validation then bounded copied-source compaction | Commit 19 privately binds the exclusive buffer to validation; copied chunks permit SIMD writing and scalar repair without aliasing |
| Historical in-place | Eligible strict profiles use the shared compactor | Rejection recovers historical diagnostics before mutation; other profiles retain the previous bounded scratch path |
| Secret/CT and staged in-place | Dedicated fixed-work and protected-storage paths | Overlap rejection, fixed-work result gates, wiping and provider accounting unchanged |
| Core sync streams, bytes fragmented drivers, Tokio readers/writers | Shared incremental state machines | Prefix commitment, backpressure, finish/failure, pending buffers, cancellation and original indices unchanged; bulk core in Commit 17, adapters in Commit 18 |
| Tokio one-shot helpers | Canonical allocating methods | Ordinary full-result ownership, existing I/O error mapping |
| Serde strict Vec/bounded adapters | Canonical limited Vec/bounded methods after input-limit gate | Serde limit precedence and redacted errors; body adapters keep dedicated wrapped grammar |
| Multibase, IMAP, password records | Family/protocol checks then selected canonical alphabet methods | Prefix translation, limits, record grammar; some prevalidation remains duplicated across crate boundaries |
| MIME, PEM, OpenPGP | Protocol/body parsing and exact limits before body transforms | Container labels, checksums, line endings and mapped indices cannot be replaced by strict Base64 acceptance |
| Sanitization, subtle, derive | Existing protected/CT, comparison, or compile-time surfaces | No secret/CT optimization or proc-macro change in this checkpoint |

Companion acceleration features are audited in [Commit 20](COMPANION_FEATURES_2.1.md). The table
now includes the subsequent [adapter integration](ADAPTER_BULK_2.1.md) and
[in-place compaction](IN_PLACE_BULK_2.1.md) changes. It does not introduce
progressive decoding semantics. Companion opt-ins preserve every routing contract above.

## Regression Checks

`sh scripts/check-2.1-forwarding.sh` exercises Rust 1.99.0 and MSRV 1.90.0 with
no-default, alloc, std, SIMD and checked combinations. Tests count validation
passes, inject reservation failure, preserve append prefixes on returned errors
and unwind, check late-invalid rejection before reservation, and check bounded
tails and owner limits. Historical errors are compared with the retained
reference policy at small/bulk sizes and malformed positions for all four
RFC 4648 profiles plus a custom alphabet family.

Wrapper-level fault injection covers validation rejection before reservation,
backend unavailability after reservation, partial-store rejection and checked
output corruption. Allocation counters separately check zero allocations with
reused append capacity, one nonempty owned output allocation, no-copy adoption,
and zero allocations on invalid input or rejected output limits. Existing
workspace/companion, staged overlap, whitespace-index and stream partition tests
remain required. These checks do not replace final hardware or fuzz admission.

Local verification passed: the focused active/MSRV gate, full workspace
all-feature tests (including companions and doctests), workspace Clippy, frozen
API snapshots and the unchanged downstream baseline. The focused proof-retention
test passed Miri. All nine forwarding tests passed on checked Rust 1.99 AArch64
under QEMU and WASI simd128 under Wasmtime; these are functional emulator/runtime
checks, not native ARM evidence. The public-API harness gate passed, including
all 12 sandbox tests with the host's delegated cgroup controllers. No new native
hardware campaign or long fuzz campaign was run.
Both loader WASM artifacts rebuild byte-for-byte unchanged and all 26 Node
loader tests pass. RISC-V checked/SIMD cross-compilation also passes; no new
native RISC-V execution is claimed for this checkpoint.

## Complete Call Measurements

The trusted development harness now includes `historical-owned` and
`string-owned`, in addition to canonical owned and reused append operations.
`string-owned` decode includes UTF-8 checking, parse/copy, decode and dropping
the temporary owner; it is not a cached-view benchmark. Both compared builds
must use this same harness. Seven alternating-order samples compare the accepted
Commit 13 source with this checkpoint using Rust 1.99.0 and `std,simd`.

The existing capture script keeps its filename for compatibility and accepts
`--direction decode`. Output includes both binary hashes, raw samples,
allocation counts, and exploratory paired classifications. Timing includes
full wrapper work; oracle verification is outside timing. An unlocked desktop
capture is not a universal no-regression or final release claim.

### Native Results

The baseline is `d0a5f1619ad9ecc996a3393fd7b1e291f4043429`. Both binaries use
the updated identical harness (including the two new owned operations) and
`RUSTFLAGS='--cfg base64_ng_perf_evidence'`. The development host is the Ryzen 9
9950X3D desktop, without CPU affinity or locked clocks. Samples cover four
strict profiles, six operations and seven raw payload sizes, totaling 2,352
rows. Reused append has zero timed allocations; nonempty historical-owned and
canonical-owned have one, while parse-plus-decode string-owned has two.

| Raw payload bytes | Decode append speedup | Historical owned decode speedup |
| --- | --- | --- |
| 3 | 1.26-1.34x | 0.98-1.03x |
| 32 | 1.32-1.38x | 3.48-4.49x |
| 192 | 1.37-1.40x | 2.52-3.30x |
| 768 | 1.41-1.42x | 1.57-1.64x |
| 65,536 | 1.36-1.41x | 1.37-1.41x |
| 1,048,576 | 1.36-1.38x | 1.35-1.42x |

Ratios are paired baseline/candidate latency ranges across the four profiles.
The final capture has 64 improvement signals, 82 rows within 5 percent,
16 inconclusive and six noisy cases, with no regression signals. Empty
historical-owned ratios span 0.91-0.99x and empty append 0.92-1.09x; these
tiny-call observations are not a universal no-regression guarantee. Some
unchanged controls also move: string-owned at 1 MiB is 1.08-1.13x. That is
not evidence of eliminated parse work; it illustrates layout/host sensitivity.
No checked-backend, cold-start, native ARM, browser, or secret throughput claim
is made from this capture.

Raw evidence: `target/release-evidence/2.1-commit14-forwarding/native-accepted.json`
(local development capture, not release admission), SHA-256
`262af723a790c7db535acc4b166f6283d15bd3c047d0724b26e4d07dd317a1b9`.
Baseline executable SHA-256:
`536c52bad3d80406aef9bba6589622e287d8fe2e923033701684a5df1dc12ab8`.
Candidate executable SHA-256:
`1983ccead96fe8f38d2502b4a7cccc96f4947ebd1c6cd7b11c52cd591455df82`.

Reproduction: export the baseline with `git archive`, copy this checkpoint's
`perf/public-api/src/main.rs` and `operations.rs` into that baseline's harness,
then build each with:

```sh
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo build --locked --release \
  --manifest-path perf/public-api/Cargo.toml --no-default-features --features std,simd
python3 scripts/measure-2.1-encode-public-api.py \
  /path/to/baseline/perf/public-api/target/release/base64-ng-public-api-perf \
  perf/public-api/target/release/base64-ng-public-api-perf --direction decode
```

The capture script executes trusted binaries; it is not the untrusted-source
sandbox runner. Keep both builds on Rust 1.99.0 for this comparison. For a new
compiler or harness, capture both sides again rather than comparing old timings.
