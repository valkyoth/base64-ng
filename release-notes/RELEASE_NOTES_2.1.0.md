# base64-ng 2.1.0 (Unreleased)

API and implementation freeze candidate. Publication, final native campaigns,
full-release CI and exact-source external acceptance are not complete.

## Ordinary Throughput And Explicit Validation

- Portable table validation and health-gated SSSE3/AVX2, little-endian NEON,
  WASM simd128 and exact Linux/SpacemiT X60 RVV validation remove the redundant
  reference pass from eligible ordinary Auto decoding. Padding, canonical
  tail bits, alphabet separation and exact diagnostics remain enforced.
- `DecodeValidation::ScalarReference` retains the reference validation pass.
  It does not force scalar writing. `checked-backend` independently checks
  accelerated validation and output, retaining quarantine and scalar recovery.
- Canonical encoding now uses admitted kernels. Allocating, bounded, append
  and formatting surfaces share the optimized routes without changing exact
  lengths, reservation ordering or rollback contracts.
- Private decode proofs survive allocating/append forwarding. Incremental
  states, sync/Tokio adapters, Bytes updates and in-place decoding process
  eligible bulk interiors while preserving progress and final-quantum rules.
- Synchronous decoder finalization now retains a valid tail for retry when
  draining the full queue fails; invalid tails do not trigger a drain.

## Additive APIs

- `Base64Ref` binds immutable borrowed input to validated codec settings and
  exact decoded length. Auto proofs may be reused; changed validator health
  forces revalidation. Reference-policy views validate on every decode.
- Per-call `DecodeReport` separates requested validation, actual validator,
  writer, checked work and recovery. Static-token policy/report methods retain
  canonical transactionality without replacing historical token methods.
- Explicit validation choices are available on incremental and adapter decode
  surfaces. Bytes, Tokio, Serde, Multibase, PEM and OpenPGP expose opt-in
  `simd`/`checked-backend` forwarding with unchanged defaults.
- No progressive decoder API ships: the prototype was measured, rejected and
  removed. Existing incremental progress contracts remain the streaming API.

## Compatibility And Limits

MSRV remains Rust **1.90.0**; the active release compiler is **1.99.0**.
The 13 Rust packages and supported npm loader target **2.1.0** together.
The core remains dependency-free. The 2.0.4 API/behavior baseline is retained.
Default features remain scalar; SIMD is opt-in. No SVE, broader RVV profile,
or new automatic AVX-512 decode route is admitted.

Secret/CT algorithms and protected-memory boundaries are unchanged. Ordinary
APIs are not constant-time or automatically wiping. Base64 is not encryption,
authentication or integrity protection. Enforce application input/output limits.
Canonical one-shot errors leave destinations unchanged; historical APIs keep
their documented mutation rules, and streams are not whole-message transactions.

The WASM loader rebuilds scalar and SIMD artifacts, with matching embedded
SHA-256 pins and build-generated source provenance. JavaScript signatures,
ownership, memory ceilings and hostile-input rejection contracts are unchanged.

## Evidence And Upgrade Guide

Development paired measurements show material bulk improvements on named x86,
AWS ARM and Apple Silicon hosts, alongside small-input, checked-backend,
cold-start and malformed-input costs. They are not portable speed guarantees
or final release authorization. Read [the policy decision](../docs/POLICY_2.1.md)
before interpreting the measurements.

See [migration](../docs/MIGRATION.md), the compiled
[policy example](../examples/decode_policy.rs),
[freeze checklist](../docs/RELEASE_FREEZE_2.1.md) and
[release plan](../docs/2.1.0-release-plan.md) for final acceptance status.
