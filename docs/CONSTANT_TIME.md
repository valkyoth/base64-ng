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
let written = ct::STANDARD.decode_slice(b"...", &mut output)?;
```

The API should be separate from the default strict decoder so users can choose
the tradeoff explicitly.

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

        pub fn decode_slice(
            &self,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, DecodeError>;

        pub fn decode_slice_clear_tail(
            &self,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, DecodeError>;

        pub fn decode_buffer<const CAP: usize>(
            &self,
            input: &[u8],
        ) -> Result<DecodedBuffer<CAP>, DecodeError>;

        pub fn decode_in_place<'a>(
            &self,
            buffer: &'a mut [u8],
        ) -> Result<&'a mut [u8], DecodeError>;

        pub fn decode_in_place_clear_tail<'a>(
            &self,
            buffer: &'a mut [u8],
        ) -> Result<&'a mut [u8], DecodeError>;
    }
}
```

The stack-backed `decode_buffer` helper avoids allocator behavior while keeping
the same cleanup and redacted formatting posture as `DecodedBuffer`.

## Implementation Rules

- Accumulate validity into masks instead of returning early on input-dependent
  byte classes.
- Avoid lookup tables indexed by input bytes or decoded 6-bit values.
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

The script writes release assembly artifacts and a checksum manifest under
`target/release-evidence/asm/` for no-default-features and all-features builds.
It wraps these raw compiler invocations:

```sh
cargo rustc --release --lib --no-default-features -- --emit=asm
cargo rustc --release --lib --all-features -- --emit=asm
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

Generated assembly and reviewer notes should be archived with release evidence
if a formal claim is made. Without that evidence, public documentation must keep
the current non-claim wording.

This policy is release-gated by:

```sh
scripts/validate-constant-time-policy.sh
```

## dudect-Style Timing Evidence

`dudect/` contains an isolated, dependency-free timing harness for the scalar
constant-time-oriented decoder. The normal gate compiles the harness and checks
its dependency policy. Local timing runs are opt-in because virtualized CI
runners and busy developer machines can produce noisy measurements:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

See [DUDECT.md](DUDECT.md) for the exact command contract and evidence rules.

## Memory Cleanup

The `ct` module provides clear-tail decode variants for caller-owned buffers.
They clear unused bytes after the decoded prefix on success and clear the whole
caller-owned buffer on error. This reduces ordinary caller-buffer retention but
does not provide a verified zeroization guarantee.

The clear-tail APIs do not try to hide success, failure, or output length:
those values are visible through the returned `Result` and decoded length. Any
future cryptographic profile must document memory cleanup separately from timing
behavior.

## Buffer Comparisons

`SecretBuffer::constant_time_eq`, `EncodedBuffer::constant_time_eq`, and
`DecodedBuffer::constant_time_eq` provide dependency-free,
constant-time-oriented comparison for equal-length buffers. Their `PartialEq`
implementations use the same helper.

Length mismatch returns immediately. Treat buffer length, the selected buffer
type, and the final equality result as public. The helper scans every byte for
equal-length inputs before returning, but this remains a best-effort API and
does not upgrade `base64-ng` to a formally verified cryptographic
constant-time comparison crate.
