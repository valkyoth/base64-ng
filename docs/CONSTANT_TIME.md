# Constant-Time Decode Design

`base64-ng` does not currently claim a formally verified cryptographic
constant-time API. The scalar encoder and decoder avoid obvious timing pitfalls,
and the `ct` module now provides an initial constant-time-oriented scalar decode
path. The stable API still prioritizes strict correctness, small size, and
ordinary performance.

This document defines the bar for strengthening the `ct` module into a
cryptographic constant-time API claim.

## Goal

Provide a clearly named API for callers that handle secret-bearing Base64
payloads:

```rust
use base64_ng::ct;

let mut output = [0u8; 32];
let written = ct::STANDARD.decode_slice_clear_tail(b"...", &mut output)?;
```

The API should be separate from the default strict decoder so users can choose
the tradeoff explicitly.

## Default Decoder Timing

The default `Engine` decode APIs are strict scalar decoders, not constant-time
decoders. They intentionally preserve exact error indexes and fast rejection for
malformed padding, invalid bytes, undersized outputs, and invalid lengths. That
means default methods such as `decode_slice`, `decode_in_place`,
`validate_result`, profile decoders, and stream adapters may branch or return
early based on malformed input content.

Treat the named default engines and profiles, including `STANDARD`,
`STANDARD_NO_PAD`, `URL_SAFE`, `URL_SAFE_NO_PAD`, `MIME`, `PEM`, `BCRYPT`, and
`CRYPT`, as strict interoperability APIs rather than token-comparison or
key-material decode APIs. For sensitive payloads, use the matching `ct`
constant such as `ct::STANDARD` or `ct::URL_SAFE_NO_PAD`, or promote an engine
with `Engine::ct_decoder()`.

Use the `base64_ng::ct` module for secret-bearing payloads where timing posture
matters more than localized malformed-input diagnostics. The `ct` module still
documents public length, output length, and final success/failure as public
values; callers with stricter protocol requirements must continue processing
with dummy data at the application layer.

High-assurance deployments that use the `ct` module should also consider
enforcing `runtime::BackendPolicy::HighAssuranceScalarOnly` at startup. That
keeps execution on the audited scalar backend and avoids future SIMD-induced
timing variation unless an accelerated backend has been admitted with its own
side-channel evidence.

## Non-Goals

- Do not describe Base64 itself as cryptography.
- Do not claim whole-program constant-time behavior.
- Do not make SIMD the first constant-time target.
- Do not hide the performance tradeoff behind the default APIs.
- Do not promise guarantees that are not backed by tests and generated-code
  review.

## Proposed Guarantee

The scalar constant-time decoder should aim to document this narrow guarantee
once the verification requirements below are complete:

> For a fixed input length and selected alphabet, the scalar constant-time
> decoder performs no secret-dependent branches and no secret-indexed table
> lookups while mapping Base64 bytes to decoded output.

The guarantee should explicitly exclude:

- public input length
- selected engine/alphabet
- final success or failure result
- total protocol work performed after the public `Result` is returned
- invalid length and output-buffer capacity errors
- output length
- allocator behavior
- memory cleanup and zeroization behavior
- OS scheduling, interrupts, and unrelated system noise

## API Shape

The initial API prefers caller-owned buffers:

```rust
pub mod ct {
    pub const STANDARD: CtEngine<Standard, true>;
    pub const STANDARD_NO_PAD: CtEngine<Standard, false>;
    pub const URL_SAFE: CtEngine<UrlSafe, true>;
    pub const URL_SAFE_NO_PAD: CtEngine<UrlSafe, false>;

    impl<A, const PAD: bool> CtEngine<A, PAD> {
        pub fn validate_result(&self, input: &[u8]) -> Result<(), DecodeError>;

        pub fn validate(&self, input: &[u8]) -> bool;

        pub fn decode_slice_clear_tail(
            &self,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, DecodeError>;

        pub fn decode_buffer<const CAP: usize>(
            &self,
            input: &[u8],
        ) -> Result<DecodedBuffer<CAP>, DecodeError>;

        pub fn decode_in_place_clear_tail<'a>(
            &self,
            buffer: &'a mut [u8],
        ) -> Result<&'a mut [u8], DecodeError>;
    }
}
```

The stack-backed `decode_buffer` helper avoids allocator behavior while keeping
the same cleanup and redacted formatting posture as `DecodedBuffer`.

The normal `Engine::decode_slice`, `Profile::decode_slice`,
`decode_slice_legacy`, and `decode_slice_wrapped` methods are documented with a
`# Security` section and a `#[must_use]` attribute. Those methods remain the
right APIs for strict diagnostics and ordinary throughput, but they may branch
or return early on malformed input. Secret-bearing payloads should use the
`ct` module, preferably `decode_slice_clear_tail` or `decode_buffer`.

## Implementation Rules

- Accumulate validity into masks instead of returning early on input-dependent
  byte classes.
- Avoid lookup tables indexed by input bytes or decoded 6-bit values.
- Decode symbols with a fixed scan over the selected alphabet so standard,
  URL-safe, bcrypt-style, crypt-style, and custom alphabets share the same
  generic mapping rule.
- Decode all complete quanta for the public input length before reporting
  malformed input.
- Keep padding validation explicit and documented; padding length and final
  output length are public.
- Return one opaque, non-localized malformed-content error from the
  constant-time-oriented path. Use the normal strict decoder when exact error
  indexes or malformed-input categories are required.
- Generate byte masks with integer arithmetic helpers instead of a generic
  `bool`-to-mask conversion. Generated-code review is still required before a
  formal constant-time claim.
- Keep the Base64 symbol-mapping and decode logic scalar and `unsafe`-free.
- Clear-tail cleanup uses the audited volatile wipe helpers documented in
  `docs/UNSAFE.md`.
- Keep the module independent from future SIMD dispatch.

## Verification Requirements

Before documenting the guarantee as formally supported:

- Unit tests for all RFC 4648 vectors.
- Exhaustive short-input tests for all byte combinations practical under the
  test budget.
- Differential tests against the strict scalar decoder for canonical inputs.
- Malformed-input tests covering invalid bytes, mixed alphabets, padding, and
  non-canonical trailing bits.
- Miri coverage for the constant-time module.
- dudect-style fixed-vs-random timing evidence for the supported release
  binaries covered by the claim.
- Generated-code review for supported release targets.
- A release note that states the exact guarantee and exclusions.

Until this evidence exists, README and SECURITY must continue to say that the
`ct` module is constant-time-oriented and does not claim a formally verified
cryptographic constant-time API.

## Generated-Code Review

Before changing the documentation from "constant-time-oriented" to a formal
cryptographic constant-time claim, maintainers must inspect generated code for
every supported release target and feature mode covered by the claim.

Minimum local commands:

```sh
scripts/generate_ct_asm_evidence.sh
```

Normal CI and the release gate run this script. It writes release assembly
artifacts and a checksum manifest under `target/release-evidence/asm/` for
no-default-features, all-features, and all-features LTO builds. It wraps these
raw compiler invocations:

```sh
cargo rustc --release --lib --no-default-features -- --emit=asm
cargo rustc --release --lib --all-features -- --emit=asm
RUSTFLAGS="-C lto=fat -C embed-bitcode=yes" cargo rustc --release --lib --all-features -- --emit=asm
```

Target-specific reviews must also include the targets named in the release
claim. For example:

```sh
cargo rustc --release --lib --no-default-features --target x86_64-unknown-linux-gnu -- --emit=asm
cargo rustc --release --lib --no-default-features --target aarch64-unknown-linux-gnu -- --emit=asm
```

The review must check the scalar `ct` decode mapping and padding/error
tracking code for:

- no secret-indexed loads from alphabet or decode tables
- no branches whose condition is derived from secret input byte classes
- no early returns after malformed content is discovered inside fixed-length
  decode loops
- no optimizer-introduced control flow that invalidates the documented mask
  arithmetic assumptions
- no accidental dispatch into future SIMD code
- high-assurance deployments that require scalar-only timing posture also
  enforce `runtime::BackendPolicy::HighAssuranceScalarOnly`

Generated assembly and reviewer notes should be archived with release evidence
if a formal claim is made. Without that evidence, public documentation must keep
the current non-claim wording.
The reviewer checklist and current release position live in
[CT_ASM_REVIEW.md](CT_ASM_REVIEW.md).

This policy is release-gated by:

```sh
scripts/validate-constant-time-policy.sh
```

## dudect-Style Timing Evidence

`dudect/` contains an isolated, dependency-free timing harness for the scalar
constant-time-oriented decoder. Normal CI and the release gate compile the
harness and check its dependency policy. Local timing runs are opt-in because
virtualized CI runners and busy developer machines can produce noisy
measurements:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

See [DUDECT.md](DUDECT.md) for the exact command contract and evidence rules.

## Memory Cleanup

The `ct` module provides clear-tail decode variants for caller-owned buffers.
They clear unused bytes after the decoded prefix on success and clear the whole
caller-owned buffer on error. This reduces ordinary caller-buffer retention but
does not provide a verified zeroization guarantee.

The non-clear-tail `ct::CtEngine::decode_slice` and
`ct::CtEngine::decode_in_place` APIs were removed before the `1.0` stable
boundary. They could leave decoded plaintext in caller-owned buffers after a
malformed input error. Use `decode_slice_clear_tail`, `decode_buffer`, or
`decode_in_place_clear_tail` for constant-time-oriented decoding.

The clear-tail slice decoder still writes decoded bytes to caller-owned output
during the fixed-shape decode loop before it reports a malformed-input error.
On error it wipes the output before returning, but this is not a synchronization
or process-isolation boundary. A same-process observer with concurrent or
unsafe access to the output buffer during the call could observe transient
partial plaintext before the final wipe.

Before reporting the opaque malformed-input result, the ct decoder passes the
accumulated error mask through a non-inlined `ct_error_gate_barrier` that uses
`core::hint::black_box`, a compiler fence, and architecture-specific hardware
speculation barriers where available (`lfence` on x86/x86_64, `isb sy` on ARM,
and `isb sy; hint #20` on AArch64). This is defense in depth around the final
public success/failure gate; it does not make the ct decoder a formally
verified hardware side-channel resistant primitive and does not change the
transient-output window described above.

For shared-memory or in-process sandbox threat models where even that transient
output window is unacceptable, use
`CtEngine::decode_slice_staged_clear_tail` with a private staging buffer. That
API writes speculative decoded bytes into staging and copies into the caller's
output only after validation succeeds.

The clear-tail APIs do not try to hide success, failure, or output length:
those values are visible through the returned `Result` and decoded length. Any
future cryptographic profile must document memory cleanup separately from timing
behavior.

Applications that must hide success/failure timing at the protocol level should
continue with fixed-shape downstream work after decode failure. A common pattern
is to decode into caller-owned storage, substitute a same-length dummy buffer on
failure, and perform the same comparison, authentication, accounting, and
cleanup steps before returning a protocol decision.

## Buffer Comparisons

`SecretBuffer::constant_time_eq_public_len`,
`EncodedBuffer::constant_time_eq_public_len`, and
`DecodedBuffer::constant_time_eq_public_len` provide dependency-free,
constant-time-oriented comparison for equal-length buffers. These redacted
buffer types intentionally do not implement `PartialEq`/`==`: the explicit
method name is part of the security contract because this helper is best-effort
and not a formal cryptographic comparison primitive.

The old `constant_time_eq` method name remains only as a deprecated migration
alias during the `1.0.0-alpha` window. New code should use the
`constant_time_eq_public_len` name so the public-length contract is visible at
the call site.

Length mismatch returns immediately. Treat buffer length, the selected buffer
type, and the final equality result as public. The helper scans every byte for
equal-length inputs before returning. The per-byte difference is passed through
`core::hint::black_box`; the accumulator is also passed through `black_box`
after each OR reduction to reduce the risk of release-mode optimizer rewrites
into early-exit equality checks. The helper is marked `#[inline(never)]` and
the release evidence script checks that `constant_time_eq_public_len` remains
visible as a separate text symbol in the LTO artifact.

This remains a best-effort API and does not upgrade `base64-ng` to a formally
verified cryptographic constant-time comparison crate. Do not use this helper
as the sole MAC, bearer-token, password-hash, or authentication-secret
comparison primitive in high-assurance systems. Applications that require a
formally audited token, MAC, or password-hash comparison should admit that
dependency at the application boundary, for example by comparing exposed bytes
with `subtle`.
