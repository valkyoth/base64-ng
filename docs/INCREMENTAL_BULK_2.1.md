# Incremental Bulk Checkpoint

Commit 17 accelerates ordinary `EncoderState` and `DecoderState` interior
blocks without changing their state layout or progress/error contract. This
is a development checkpoint, not release admission. External review and CI
passed at `df2fe8f`, including the independent scalar fuzz-oracle follow-up.

## Contract

- Encode calculates accepted input arithmetically, then processes complete
  output-fitting triples through the existing admitted encoder. Rejection
  overwrites the entire planned span through the scalar encoder. Partial
  input and pending output retain the original bytewise behavior.
- Decode plans the whole accepted prefix before draining pending output or
  writing new output. An optional private preflight proof binds the exact
  immutable interior span, settings, validated length and selected backend.
  Remaining scalar quanta must also pass before that proof can be consumed.
- Bulk decode is restricted to the strict Standard/URL-safe families. Custom,
  relaxed and legacy-whitespace settings retain scalar planning. Only one
  bulk attempt is made per update, so malformed fallback remains linear,
  although it can scan rejected input more than once.
- The planner never validates past the accepted prefix. It preserves the
  original extra buffered quantum when output fills, absolute source indexes,
  pending bytes, terminal padding, absorbing failure, reset/clear and retryable
  finish. Derived `Clone`, `Debug` and equality retain the same stored bytes.
- Decoder validation errors leave the complete destination untouched, even
  if pending output was waiting to drain. Earlier successful calls are not
  rolled back: this is per-call transactionality, not whole-message buffering.
- Health is rechecked before writing. Existing quarantine, scalar overwrite
  recovery and independent checked-backend validation/output comparisons
  apply. No unchecked/progressive decoding or new ISA admission is introduced.

Encoder bulk work requires at least 192 aligned input bytes and 256 output
bytes. Decoder bulk planning requires at least 512 interior encoded bytes
that fit output, reserving the final quantum for scalar padding logic. Calls
below 516 input or 384 output bytes use the scalar specialization. Actual SIMD
execution also requires the shared backend's size, feature and health checks;
these planning floors do not lower NEON or other admission thresholds.

The new additive method is per-call, rather than a policy stored in the state:

```rust
use base64_ng::{DecodeValidation, STRICT_STANDARD_PADDED};

let mut decoder = STRICT_STANDARD_PADDED.decoder();
let mut output = [0; 3];
let step = decoder.update_with_validation(
    b"Zm9v", &mut output, DecodeValidation::ScalarReference,
).unwrap();
assert_eq!(step.progress().output_produced(), 3);
assert_eq!(&output, b"foo");
decoder.finish(&mut []).unwrap();
```

`update` uses `Auto`. `ScalarReference` validates newly accepted input through
the independent original state-machine validator; it can still select an
admitted writer. Previously pending output was validated by its original call
and is not retroactively revalidated. `finish` retains its scalar terminal
rules. The independent validator uses a separate scalar entry point so it
cannot recursively call bulk preflight.

These are ordinary, non-constant-time, non-wiping transforms. Enforce protocol
input limits, and use the separate secret/CT types for secret-bearing data.
Secret state machines, dependencies, unsafe kernels and public state sizes
are unchanged. Existing synchronous/async adapters call the ordinary states
and can benefit indirectly; adapter implementation changes and end-to-end
measurements are recorded in [Commit 18](ADAPTER_BULK_2.1.md).

## Verification

`sh scripts/check-2.1-incremental-bulk.sh` exercises active Rust and MSRV 1.90
across core-only, alloc, std, SIMD and checked-backend configurations. Tests
compare original scalar and bulk states after every call, including complete
output sentinels, pending prefixes, malformed tails, partial/zero destinations,
overflow, resets, finish retries and custom/legacy fallback. Allocation tests
require zero heap calls for caller-buffer processing. The shared vector fault
hooks cover rejection, unavailable writers and checked corruption.
Source-guard mutation tests require the split update module to exist and keep
the older decoder gates checking its allocation, panic and unsafe restrictions.

The bounded `incremental_bulk_miri_proof_and_pending_boundaries` test exercises
proof lifetime and pending-state behavior under Miri. The `incremental_bulk`
downstream target tests production-linked per-call transactionality and verifies
that malformed unaccepted suffixes are not inspected. The incremental fuzz
target mixes tiny and large fragment schedules and compares results, complete
destinations and state after each update against the original scalar planner
and writer. A hidden hook available only under cargo-fuzz's `cfg(fuzzing)`
bypasses bulk planning entirely; selecting `ScalarReference` alone would still
share the bulk planner and writer. Five committed seeds start at bulk-sized
inputs for every strict profile, including a late-invalid input. The focused
gate replays these seeds through the actual fuzz harness and verifies that the
oracle invokes neither vector validation nor vector writing.

Completed locally: the active/MSRV gate, full workspace release/all-feature
tests, both focused encoder/decoder Miri boundary tests, 10,000 seeded
incremental fuzz executions, checked AArch64 QEMU and Wasmtime bulk tests,
production-linked AArch64 integration tests, RISC-V all-feature test
cross-compilation and core-only `thumbv7em-none-eabihf` compilation. The rebuilt
scalar/SIMD WASM artifacts match their loader pins and pass all 26 Node tests.
Emulation and cross-compilation are not native hardware performance evidence.
The complete repository check sequence also passed, including package/browser
checks, Clippy, documentation, dependency policy and RustSec. The isolation
gate required host user-bus access outside the coding sandbox; both legacy
decoder source guards were updated and mutation-tested for the module split.

## Measurements

The trusted `scripts/measure-2.1-incremental.py` runner compares frozen parent
and candidate public-API harness executables. It includes state construction,
all updates and finish, seven alternating paired samples, four strict profiles,
32-byte through 1-MiB decoded payloads and 7/256/4096/1048576-byte fragment
limits applied to both input and output. It checks zero allocations and hashes
both executables before and after capture. It is a development timing tool,
not an untrusted-code sandbox or release-evidence admission gate.

Local raw captures are retained under
`target/release-evidence/2.1-commit17-incremental/`. Native measurements compare
`98cafa42c2924f126e43aeab93e19827bd66e701` against this implementation on the
same Ryzen 9 9950X3D, Rust 1.99.0, `std,simd` harness, pinned to CPU 2. No native
ARM/Mac/RISC-V performance or long release-fuzz claim is made here.

Paired parent/candidate time ratios across all four profiles (larger is better):

| Decoded payload | Fragment limit | Encode ratio | Decode ratio |
| --- | --- | --- | --- |
| 32 B | 7 B | 1.00-1.04x | 0.93-0.97x |
| 768 B | 4 KiB | 19.65-20.72x | 68.54-70.49x |
| 64 KiB | 4 KiB | 31.53-31.79x | 188.09-193.04x |
| 1 MiB | 4 KiB | 29.67-30.78x | 181.67-196.19x |
| 1 MiB | 1 MiB | 94.33-100.42x | 342.61-353.23x |
| 1 MiB | 7 B | 0.99-1.00x | 0.91-0.94x |
| 1 MiB | 256 B | 1.00-1.01x | 0.90-0.91x |

These large bulk ratios compare against the previous bytewise incremental
decoder, not against other libraries or the already optimized one-shot APIs.
They must not be advertised as a general Base64 speedup. Standard padded
1-MiB/4-KiB-fragment decode changed from about 35.39 ms to 191 us; encoding
changed from 2.806 ms to 91 us. All timed incremental operations allocated zero.

Tiny fragments do not qualify for bulk decode and retain a measured regression:
roughly 3-11% more time in these cases, despite keeping a scalar specialization.
For 32-byte Standard padded input with a large destination, complete incremental
decode was about 841 ns before and 903 ns after. Tiny encoding is near the old
cost. Applications feeding small fragments should not expect this optimization
to help; end-to-end buffering/adapter work remains separate.

The same frozen public harness also captured canonical, historical, allocating,
append and string decode controls in `one-shot-decode.json`. Bulk control ratios
were at least 0.989x; some tiny operations regressed by single-digit or low
double-digit nanoseconds, for example canonical URL-safe unpadded 3-byte decode
from 23.4 ns to 32.3 ns. These code-generation/layout tradeoffs remain visible,
not classified as no regression. Results include allocator and complete-call
overheads where applicable, and are not portable performance guarantees.
