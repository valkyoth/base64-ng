# base64-ng 1.0.0 Release Notes

Status: released

## Summary

- Started the stable API and security-contract freeze candidate after the
  `0.12.0` stabilization release.
- Accepted the documented Kani verifier exception for the initial `1.0.0`
  contract: Kani harnesses remain in-tree and release-gated, but incompatible
  Kani compiler runs are policy skips backed by replacement evidence, not
  proofs.
- Hardened `stream::Encoder::write` so accepted input after a completed
  pending quantum continues through the current slice when buffer capacity
  allows, including preserving final 1-2 byte tails instead of forcing an
  early short write.
- Hardened `stream::Decoder::write` so direct writes process multiple complete
  Base64 quads per call, continue after completing pending input, and preserve
  final partial quads as pending input when those bytes are accepted.
- Hardened dependency-free cleanup by adding an architecture-gated inline
  assembly barrier after volatile wipe loops, while keeping crate-level docs
  explicit that cleanup remains best-effort and not formal zeroization.
- Strengthened default-engine and validation documentation so
  `STANDARD`/`URL_SAFE_NO_PAD`/profile users are pointed at `ct` constants or
  `Engine::ct_decoder()` for token validation and key-material decoding.
- Removed the non-clear-tail `ct::CtEngine::decode_slice` and
  `ct::CtEngine::decode_in_place` APIs before the `1.0` stable boundary because
  failed CT decodes could leave decoded plaintext in caller-owned buffers.
  Use `decode_slice_clear_tail`, `decode_buffer`, or
  `decode_in_place_clear_tail`.
- Hardened the equal-length comparison helper by making the OR accumulator
  opaque with `core::hint::black_box`, while preserving the documented
  best-effort constant-time-oriented posture.
- Renamed the internal padding-index helper to make its padding-present
  precondition explicit and added a debug assertion plus non-index sentinel for
  future misuse.
- Changed SIMD prototype equivalence tests to gate on per-feature availability
  and print explicit skip reasons instead of silently skipping lower-tier
  prototypes on higher-tier hardware.
- Added an explicit `Cargo.toml` comment documenting that the `tokio` feature
  is a reserved, dependency-free no-op until async admission is complete.
- Added public `wasm32` cleanup caveats for `EncodedBuffer`, `DecodedBuffer`,
  `SecretBuffer`, and memory-retention docs because wasm targets currently use
  the compiler-fence-only wipe barrier.
- Removed `PartialEq`/`Eq` implementations from `EncodedBuffer`,
  `DecodedBuffer`, and `SecretBuffer` so `==` cannot imply a formal
  constant-time token/MAC comparison guarantee; callers must use the explicit
  best-effort `constant_time_eq` helper or an application-admitted audited
  comparison crate.
- Tightened the SIMD admission policy so any future vector backend that loads
  caller data into SIMD registers must implement, document, and provide
  generated-assembly evidence for explicit register cleanup before it can be
  dispatched.
- Changed `wasm32` builds to fail closed by default unless callers explicitly
  enable the dependency-free `allow-wasm32-best-effort-wipe` feature to accept
  compiler-fence-only cleanup.
- Marked `wipe_bytes` as `#[inline(never)]` and extended generated assembly
  evidence to include an all-features LTO artifact for cleanup-boundary review.
- Added `#[must_use]` and stronger `# Security` rustdoc guidance to standard
  decode-slice APIs so secret-bearing callers are directed to the `ct` module.
- Added debug bounds assertions around wrapped-output writes and made the
  wrapped-encode scratch-buffer fallback use explicit checked arithmetic.
- Changed `SecretBuffer::into_exposed_vec` to return an `ExposedSecretVec`
  wrapper that remains redacted and wiped on drop; raw `Vec<u8>` extraction now
  requires the explicit
  `into_exposed_unprotected_vec_caller_must_zeroize` escape hatch.
- Documented the custom `Alphabet` timing contract: manual `encode`/`decode`
  overrides affect the normal `Engine` path, while the `ct` module scans
  `Alphabet::ENCODE` directly.
- Added RFC 4648 ground-truth vectors to the differential fuzz target and
  release-gated the fuzz workspace `cargo audit`/`cargo deny` checks.
- Documented that `ct::CtEngine::decode_slice_clear_tail` wipes caller output
  before returning errors, but same-process concurrent or unsafe access during
  decode could observe transient partial plaintext before that wipe.

## Commit Range

- Previous tag: `v0.12.0`
- Release tag: `v1.0.0`
- Release date: `2026-05-19`

## Commits

### Added

- `9f67f75` Add SIMD activation checklist
- `4ae0c6e` Clarify padding index helper precondition
- `a8f68e0` Add post-1.0 SIMD roadmap

### Security / Hardening

- `67614dc` Harden cleanup wipe barrier
- `b4e04a8` Make backend unsafe posture conservative
- `56ea612` Wipe stream reader buffers on read errors
- `97ce2eb` Harden equality accumulator opacity
- `6588915` Add fail-closed wasm wipe policy
- `332f388` Fail closed on wasm wipe limitations
- `f2b8965` Keep wipe primitive out of LTO inlining
- `fbfd691` Remove unsafe ct residual decode APIs
- `7648174` Harden wrapped output write invariants
- `15b4d3a` Harden fuzz oracle and dependency gates
- `c7bb0b9` Harden comparison helper evidence
- `74e10a2` Strengthen wipe barrier policy
- `b278725` Install wasm target for wipe policy check
- `4b44ef2` Address final pentest hardening notes
- `7e70e13` Address final pentest hardening findings
- `2b9433b` Harden CT barrier and secret string handling
- `cce5260` Harden exposed arrays and CT posture reporting
- `83d01da` Harden CT and secret buffer boundaries
- `ceec487` Address final pentest hardening pass
- `4b73e99` Harden scoped pentest findings
- `4285328` Harden secret wrappers and CT anchors

### Documentation

- `164152b` Document scalar-only ct deployment guidance
- `2709ac8` Document sensitive decode entry points
- `b9fefa6` Document ct decode output cleanup requirement
- `22a19e7` Document wasm cleanup caveat
- `ddceee9` Document strict decode timing boundary
- `87b3b1e` Document custom alphabet timing contract
- `2de6f85` Document ct transient output window
- `1f7abab` Prepare 1.0.0 release

### Verification

- `46f91d0` Clarify SIMD prototype evidence limits
- `3b356f0` Normalize SIMD prototype test gating
- `ca5e799` Exercise SIMD prototype tests by feature

### Other Changes

- `ad1d23a` Prepare 1.0.0 alpha candidate
- `d7a5976` Fix stream encoder tail buffering
- `40d1066` Process multiple stream decoder quads
- `77314d4` Clarify public-length equality semantics
- `de239dc` Reject zero line wrap construction
- `c8afe83` Surface SIMD candidate detection mode
- `e548b6a` Avoid shadow quad copy in decoder write
- `042f083` Clarify reserved tokio feature
- `04903e5` Deprecate retaining ct slice decode
- `8cbe3f2` Deprecate destructive ct in-place decode
- `a5001c5` Remove redacted buffer equality sugar
- `34c45cb` Require SIMD register cleanup before dispatch
- `2142ebf` Protect exposed secret vector ownership

## Verification

This file is generated from repository history. See the matching
`security/pentest/v1.0.0.md` report and the tagged CI/release-gate
artifacts for the permanent security-review context.
