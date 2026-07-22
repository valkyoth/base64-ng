# Security Controls And CWE Mapping

This document maps `base64-ng` security controls to common weakness classes.
The mapping is practical adoption guidance, not a certification claim.

## Mitigated Or Reduced By The Crate

| Weakness Class | Relevant Risk | `base64-ng` Control |
| --- | --- | --- |
| CWE-20 Improper Input Validation | Accepting malformed Base64, mixed alphabets, invalid padding, or non-canonical trailing bits | Strict decoding is default; validation-only APIs use the same checks as decode APIs. |
| CWE-125 Out-of-bounds Read | Decoder reads past malformed or short input | Scalar decode validates lengths and chunk shapes before reading chunk contents. |
| CWE-787 Out-of-bounds Write | Encoder/decoder writes beyond caller output | Slice APIs check required output length before writing; in-place APIs validate buffer capacity and decode-to-front invariants. |
| CWE-190 Integer Overflow | Encoded or wrapped length calculation wraps `usize` | Public length helpers return `Result` or `Option`; wrapped lengths use checked arithmetic. |
| CWE-400 Uncontrolled Resource Consumption | Allocating based on attacker-controlled encoded input | `decode_vec` validates input before allocating; caller-owned APIs and checked length helpers are available for hard limits. |
| CWE-209 Information Exposure Through Error Messages | Constant-time-oriented malformed decode reveals exact failure location or category | `ct` malformed-content errors are intentionally opaque and non-localized. |
| CWE-226 Sensitive Information in Resource Not Removed Before Reuse | Caller-owned output buffers or crate-owned staging buffers retain partial sensitive data | Clear-tail APIs, stream cleanup, `EncodedBuffer`, `DecodedBuffer`, and `SecretBuffer` provide best-effort initialized-byte and spare-capacity cleanup. |
| CWE-327 Use of Broken Or Risky Cryptographic Algorithm | Treating Base64 as encryption | Documentation describes Base64 as encoding, not encryption; secret wrappers are retention/logging helpers only. |
| CWE-532 Insertion of Sensitive Information Into Log File | Accidentally logging sensitive encoded or decoded material | `SecretBuffer` redacts `Debug` and `Display` output and requires explicit reveal calls. Strict `DecodeError` values may include input bytes or indexes; log `DecodeError::kind()` for secret-adjacent input. |
| CWE-208 Observable Timing Discrepancy | Sensitive comparison exits early on the first different byte | `SecretBuffer`, `EncodedBuffer`, and `DecodedBuffer` intentionally avoid `PartialEq`/`==` so best-effort comparison cannot be mistaken for a formal cryptographic primitive. Use explicit `constant_time_eq_public_len` for dependency-free equal-length scans; length mismatch returns immediately and is public. |
| CWE-829 Inclusion of Functionality From Untrusted Control Sphere | Runtime dependency compromise | Published crate has zero external runtime and default dev dependencies; dependency admission is documented and checked. |

The `constant_time_eq_public_len` helpers are dependency-free hardening aids,
not audited MAC, bearer-token, password-hash, or authentication-secret
comparison primitives. Their implementation is release-evidence-gated through
generated assembly review, including LTO symbol-presence checks for the helper,
but no formal cryptographic constant-time comparison guarantee is claimed.
High-assurance applications that can admit a comparison dependency should use a
reviewed primitive such as `subtle` at the protocol boundary. The shorter
`constant_time_eq` name was removed before `1.0.0` so the public-length
comparison contract stays visible at call sites.

## Caller Responsibilities

The caller still owns:

- authentication, encryption, signature verification, and key management
- protocol-level framing and maximum accepted payload size
- choosing strict, legacy, wrapped, MIME, PEM, bcrypt-style, or custom profiles
  intentionally
- avoiding accidental copies of secrets before wrapping them in `SecretBuffer`
- treating
  `into_exposed_unprotected_array_caller_must_zeroize` calls as boundaries
  where redacted formatting and crate-owned drop-time cleanup intentionally
  stop for returned bare arrays. These methods are safe Rust escape hatches,
  but they have a quasi-unsafe cleanup contract: callers must zero the returned
  array with their own approved mechanism before it leaves scope.
- understanding that stack-backed buffers can clear their own backing arrays
  but cannot clear historical stack-frame copies made by compiler spills,
  caller code, panic machinery, crash handlers, or operating system capture
- treating `ExposedSecretVec::into_exposed_unprotected_vec_caller_must_zeroize`
  as a boundary where redacted formatting and crate-owned drop-time cleanup
  intentionally stop. High-assurance reviews should grep for every
  `into_exposed_unprotected_*_caller_must_zeroize` call site and verify the
  returned value is cleared by the caller's approved cleanup mechanism.
- treating
  `ExposedSecretString::into_exposed_unprotected_string_caller_must_zeroize` as
  a boundary where redacted formatting and crate-owned drop-time cleanup
  intentionally stop. These methods are intentionally verbose because they
  transfer cleanup responsibility out of `base64-ng`.
- process-wide memory hygiene such as core-dump policy, swap policy, crash
  handling, allocator behavior, and log retention
- deciding whether the constant-time-oriented API is sufficient for the local
  threat model
- enforcing protocol-level maximum input sizes before calling allocation
  helpers or constant-time-oriented decode on untrusted input. `decode_vec` and
  `encode_vec` allocate proportionally to input size, and the `ct` decoder
  deliberately spends fixed work scanning all 64 alphabet entries per symbol.
  Use streaming adapters or stack-backed `decode_buffer::<MAX>()` style APIs for
  bounded services.
- running the release gate and reviewing release evidence for the exact
  version being adopted

## High-Assurance Deployment Checklist

Use this checklist for military, HSM-adjacent, enclave-adjacent, multi-tenant,
or other high-assurance deployments. It is operational guidance, not a
certification claim.

- Confirm `runtime::backend_report().memory_lock_posture()` reports
  `MemoryLockPosture::NotProvided`, then wrap long-lived key material in the
  platform's approved memory-locking and dump-control policy (`mlock`,
  `VirtualLock`, RTOS locked SRAM, disabled or encrypted swap, hibernation
  controls, and crash-dump suppression as applicable).
- Call
  `runtime::require_backend_policy(runtime::BackendPolicy::HighAssuranceScalarOnly)`
  during startup and treat failure as a hard stop for sensitive deployments.
  On AArch64, failure is expected unless the operator has attested CSDB
  effectiveness with `--cfg base64_ng_aarch64_csdb_attested`; do not use that
  cfg without processor, BSP, or certification evidence.
  Deployment CI for a certified AArch64 target should run the application or
  integration test suite with the same `RUSTFLAGS` and assert that this startup
  policy passes. Generic project CI must not set that cfg because it would turn
  an operator attestation into an unreviewed build default. RISC-V and 32-bit
  ARM targets should expect this policy to fail unless the platform provides
  external Spectre-v1 mitigation evidence.
- Decode tokens, wrapped keys, MAC-adjacent payloads, and equivalent
  secret-bearing Base64 through `base64_ng::ct`. Use
  `ct::CtEngine::decode_slice_staged_clear_tail` when the caller-owned output
  buffer could be observed during the decode call, such as shared memory,
  sandbox boundaries, enclave-adjacent code, or multi-principal processes.
- Do not treat `SecretBuffer::try_from`, `SecretBuffer::from_str`, or related
  conversion traits as constant-time-oriented decode APIs. They use the normal
  strict `STANDARD` decoder and provide redacted storage plus best-effort
  cleanup after decoding. Use `base64_ng::ct`, `base64-ng-derive`, or
  `base64-ng-sanitization` when malformed-input timing posture matters.
- Treat in-place clear-tail APIs as destructive cleanup boundaries. On error,
  they clear the full caller-provided buffer, including original plaintext or
  encoded input. Preserve a separate audit/retry copy before calling them if
  recovering the original input is part of the protocol.
- Prefer fixed-size secret types generated by the `base64-ng-derive`
  `#[derive(Base64Secret)]` companion crate when protocol key sizes are known.
  The generated decode path uses staged CT decode, redacted `Debug`, and
  drop-time cleanup by default.
- For long-lived decoded key material on supported native targets, consider
  `base64-ng-sanitization` with its `high-assurance` feature. That path uses
  exact-pinned `sanitization` `=2.0.3` hardened native controls, including
  memory locking, strict random canaries, and strict assembly comparison, and
  exposes helpers that decode directly into
  `LockedSecretBytes` or `LockedSecretVec` without first landing in a normal
  `Vec`. The feature selects compiled controls; it does not prove every
  preferred platform operation succeeded. For fixed-size secrets,
  `decode_locked_secret_bytes_checked` establishes required memory-lock, dump,
  and fork controls before plaintext decode. The built-in dynamic
  `decode_locked_secret_vec_checked` uses a protected-capacity constructor that
  establishes the same controls before its decode closure runs. External trait
  implementations must override the compatibility default for that guarantee.
  Inspect `protection_report()` before admitting a value returned by a
  non-checked compatibility helper.
- Use `CtDecodeSanitizationProtectedExt` where monitoring must distinguish an
  unavailable protection control from a canary-integrity event. Prefer
  `decode_locked_secret_vec_checked_bounded::<MAX>` for attacker-reachable
  dynamic input so the decoded-capacity limit is enforced before mapping
  allocation, protection setup, or decoder invocation.
- Prefer `LockedSanitizationCtEqExt` for locked comparisons so applications can
  emit incident telemetry and apply an explicit abort policy on
  `CanaryCorruptedError`. The source-compatible `SanitizationCtEqExt` path now
  has a reviewed panic boundary and must never translate corruption into an
  ordinary comparison mismatch; panic unwinding is not a substitute for an
  application-controlled abort policy.
- Keep dependency review split by package. The core `base64-ng` crate has zero
  runtime dependencies; optional companion crates such as `base64-ng-serde`,
  `base64-ng-bytes`, `base64-ng-subtle`, and `base64-ng-tokio` intentionally
  admit ecosystem dependencies and should be reviewed at the integration layer.
- Use the optional `base64-ng-subtle` companion crate when protocol boundaries
  require a reviewed `subtle::ConstantTimeEq` comparison primitive instead of
  the dependency-free best-effort comparison helpers in the core crate.
- Set protocol-level maximum input sizes before allocation helpers or CT decode
  are reachable from untrusted traffic. Prefer caller-owned buffers, fixed-size
  `decode_buffer::<MAX>()` patterns, or streaming adapters for bounded services.
  For async services using the optional `base64-ng-tokio` companion crate,
  prefer `encode_reader_to_writer_limited` and
  `decode_reader_to_writer_limited` at request or frame boundaries controlled
  by a peer for non-secret payloads. Do not use those Vec-backed helpers as the
  high-assurance token, wrapped-key, or MAC-adjacent decode boundary. For
  secret-bearing async frames, collect a bounded frame under the application's
  approved memory policy and decode through `base64_ng::ct`, staged CT decode,
  `base64-ng-derive`, or `base64-ng-sanitization`. The Tokio companion's
  `EncoderReader`, `DecoderReader`, `EncoderWriter`, and `DecoderWriter`
  adapters are true streaming state machines with fixed internal buffers and
  drop cleanup. Writer adapters finalize pending Base64 tails during shutdown,
  not flush. Decoded output from valid leading quanta can still reach the
  caller before a later malformed quantum is observed.
- Log redacted error classifications such as `DecodeError::kind()` for
  secret-adjacent inputs. Strict decode errors can carry exact offsets and
  offending input bytes for diagnostics.

## Control Boundaries

### Input Validation

Strict APIs reject:

- non-alphabet bytes
- mixed standard and URL-safe alphabets
- invalid lengths
- malformed padding
- trailing data after padding
- non-canonical trailing bits

Strict errors are diagnostic objects. `DecodeError::InvalidByte` carries the
offending byte and `DecodeError` variants can carry exact input indexes. Do not
log full strict errors verbatim when the input may contain secrets,
secret-adjacent tokens, or attacker-probed fragments of those values. Use
`DecodeError::kind()` for redacted logging, or use the `ct` module for opaque
malformed-content errors.

```rust
let err = base64_ng::STANDARD
    .decode_buffer::<32>(input)
    .unwrap_err();
tracing::warn!(error_kind = %err.kind(), "base64 decode failed");
```

Legacy whitespace handling is opt-in through explicitly named APIs. Wrapped
profiles are strict about the configured line ending and non-final line width.

### Memory Retention Reduction

Cleanup APIs are best-effort initialized-byte cleanup. They are implemented
without runtime dependencies using audited volatile wipe helpers, an
architecture-gated inline assembly barrier, hardware store-ordering fences on
supported native architectures, and compiler fences as inventoried in
[UNSAFE.md](UNSAFE.md). They do not claim formal zeroization against compiler
behavior, historical stack-frame copies, allocator internals, copies made
outside the wrapper, core dumps, swap, hibernation images, cold-boot
remanence, CPU registers, cache lines, write buffers, or arbitrary process
memory disclosure vulnerabilities.
On `wasm32`, the wipe barrier uses only a compiler fence; downstream wasm
runtime JIT behavior is outside this crate's control. `wasm32` builds fail
closed by default; deployments must enable `allow-wasm32-best-effort-wipe` to
accept compiler-fence-only cleanup. High-assurance wasm deployments should
apply their own memory strategy around `EncodedBuffer`, `DecodedBuffer`,
`SecretBuffer`, and caller-owned output buffers.
Unsupported native architectures without an implemented hardware wipe barrier
also fail closed by default; deployments must enable
`allow-compiler-fence-only-wipe` only after reviewing the weaker cleanup
posture and applying platform memory controls.
For high-assurance secret handling, use the clear-tail APIs promptly and pair
them with operating system and deployment controls that reduce crash dumps,
swap, hibernation, paging, allocator reuse, and broad memory disclosure
exposure. Examples include locked memory where available (`mlock`,
`VirtualLock`, or an RTOS equivalent), disabled or encrypted swap and
hibernation, crash-dump suppression, short key lifetimes, allocator isolation
for secret regions, and the deployment's approved zeroization primitive at the
ownership boundary.
For constant-time-oriented decode, use `ct::CtEngine::decode_slice_clear_tail`
or `ct::CtEngine::decode_buffer` when a caller-owned output buffer may be
reused after a rejected input. For shared-memory, HSM-adjacent, sandboxed, or
multi-principal environments where even transient writes to caller output are
unacceptable, use `ct::CtEngine::decode_slice_staged_clear_tail` with a private
staging buffer. The non-clear-tail CT slice API was removed before the `1.0`
stable boundary because it could leave decoded plaintext from valid leading
quanta in the buffer on error.
The internal constant-time-oriented decode loop writes decoded bytes to the
caller-owned output buffer before final malformed-input reporting, then
`decode_slice_clear_tail` wipes the buffer before returning an error. This
reduces post-return retention, but it is not an isolation boundary: code running
in the same process with concurrent or unsafe access to the output buffer during
the decode call could observe transient partial plaintext before the final wipe.
Before the opaque malformed-input result is reported, the accumulated ct error
mask passes through a non-inlined compiler barrier plus architecture-specific
hardware speculation barriers where available. Shared-memory deployments that
cannot tolerate transient writes to the caller output should use
`CtEngine::decode_slice_staged_clear_tail` with a private staging buffer.
On AArch64, the emitted CT gate uses `isb sy` plus the CSDB hint encoding.
The runtime report classifies this as
`CtGatePosture::HardwareSpeculationBarrierUnattested`, so
`runtime::BackendPolicy::HighAssuranceScalarOnly` does not pass on AArch64
solely because the instruction sequence was emitted. Deployments that rely on
CSDB must attest that the deployed core treats it as an effective speculation
barrier; older ARM cores may treat the hint as a no-op. Military and
high-assurance deployments should call
`runtime::require_backend_policy(runtime::BackendPolicy::HighAssuranceScalarOnly)`
during startup and treat failure on unattested AArch64 as expected. If platform
evidence from processor documentation, BSP notes, or certification material
exists, build with `--cfg base64_ng_aarch64_csdb_attested` to make the runtime
posture reflect the deployment attestation as
`CtGatePosture::HardwareSpeculationBarrierBuildAsserted`.
(`hardware-speculation-barrier-build-asserted`). This is intentionally not a
Cargo feature, and the distinct posture string keeps audit logs from confusing
operator attestation with a native target guarantee. On RISC-V, the crate
reports an ordering-fence CT gate because the base ISA does not provide a
canonical speculation barrier. RISC-V deployments in Spectre-v1 threat models
must rely on platform-level mitigations outside this crate.
For constant-time-oriented in-place decode, use
`ct::CtEngine::decode_in_place_clear_tail`. The non-clear-tail CT in-place API
was removed before the `1.0` stable boundary because it could partially destroy
the encoded input and retain decoded plaintext on error.
If a platform requires a formal zeroization policy, apply that policy to
caller-owned buffers in addition to the crate's dependency-free cleanup APIs.
For applications that already admit `zeroize`, decode into caller-owned buffers
and apply `Zeroize::zeroize()` after the Base64 step; `base64-ng` intentionally
does not add that dependency for every user.
Avoid cloning stack-backed decoded or encoded buffers that contain secret
material. `DecodedBuffer` and `EncodedBuffer` implement `Clone` for ergonomic
no-alloc interop, but cloning duplicates visible bytes and may create compiler
temporaries outside the crate's cleanup boundary. Use `SecretBuffer` for
heap-owned secret material when clone-free redacted handling is required.

### Side-Channel Posture

The default scalar encoder avoids input-derived alphabet table indexes for
built-in alphabets, and custom alphabet helper decoding now scans the full
64-byte alphabet before returning. The default strict decoder still preserves
localized errors and may return early for malformed input, padding, length, or
output-size failures, so it is not a constant-time decoder. Named engines and
profiles such as `STANDARD`, `URL_SAFE_NO_PAD`, `MIME`, `PEM`, `BCRYPT`, and
`CRYPT` are strict interoperability APIs, not token-comparison or key-material
decode APIs. The `ct` module narrows the timing target further for scalar
decode and uses a fixed scan over the selected alphabet for generic symbol
mapping. It still does not carry a formal cryptographic constant-time
guarantee.
For high-assurance MAC, bearer-token, password-hash, or equivalent protocol
comparisons, use a deployment-approved reviewed constant-time comparison
primitive at the protocol boundary. The crate's dependency-free comparison
helpers are intentionally documented as best-effort and public-length only.
Input length, padding length, decoded length, and final success/failure remain
public protocol facts; callers that must hide those facts need fixed-shape
protocol-level processing after decode failure.
Avoid the `stream` decoder adapters for secret-bearing Base64 when
constant-time behavior is required. Streaming decoders intentionally use the
normal strict decoder so they can preserve I/O-style behavior and avoid
buffering unbounded frames. For secrets, collect one complete protocol frame
under an application size limit and decode it with `ct::CtEngine`.
Normal SIMD decode admission is not a secret-decoding claim. The `1.3.0`
decode acceleration scope is limited to strict Standard and URL-safe
interoperability decode. The admitted decode backends are std x86/x86_64
AVX-512 VBMI for full 64-byte encoded blocks, AVX2 for full 32-byte encoded
blocks, SSSE3/SSE4.1 for full 16-byte encoded blocks, and little-endian std
aarch64 NEON for full 16-byte encoded blocks after scalar whole-input
validation; every other decode backend, including big-endian AArch64, and the
`base64_ng::ct` constant-time-oriented decode path remain scalar unless a
separate formal side-channel evidence package admits otherwise.
Deployments that require the most conservative side-channel posture should
combine `base64_ng::ct` with
`runtime::BackendPolicy::HighAssuranceScalarOnly` so sensitive decode paths
remain on the scalar backend and are not affected by future SIMD admission.

### Supply Chain

The published crate is dependency-free by default. Fuzzing dependencies are
isolated under `fuzz/` and reviewed separately. The release gate requires
formatting, clippy, tests, documentation, cargo-deny, RustSec audit, license
inventory, SBOM generation, and reproducibility checks.
