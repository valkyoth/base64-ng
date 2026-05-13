# Constant-Time Decode Design

`base64-ng` does not currently claim a formally verified cryptographic
constant-time API. The scalar encoder and decoder avoid obvious timing pitfalls,
but the stable API still prioritizes strict correctness, small size, and
ordinary performance.

This document defines the bar for a future constant-time-focused decode module.

## Goal

Add a clearly named API for callers that handle secret-bearing Base64 payloads:

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

The future scalar constant-time decoder should aim to document this narrow
guarantee:

> For a fixed input length and selected alphabet, the scalar constant-time
> decoder performs no secret-dependent branches and no secret-indexed table
> lookups while mapping Base64 bytes to decoded output.

The guarantee should explicitly exclude:

- public input length
- selected engine/alphabet
- final success or failure result
- output length
- allocator behavior
- memory cleanup and zeroization behavior
- OS scheduling, interrupts, and unrelated system noise

## API Shape

The initial API should prefer caller-owned buffers:

```rust
pub mod ct {
    pub const STANDARD: CtEngine<Standard, true>;
    pub const STANDARD_NO_PAD: CtEngine<Standard, false>;
    pub const URL_SAFE: CtEngine<UrlSafe, true>;
    pub const URL_SAFE_NO_PAD: CtEngine<UrlSafe, false>;

    impl<A, const PAD: bool> CtEngine<A, PAD> {
        pub fn decode_slice(
            &self,
            input: &[u8],
            output: &mut [u8],
        ) -> Result<usize, DecodeError>;
    }
}
```

Allocation helpers may come later, but the first version should avoid allocator
behavior entirely.

## Implementation Rules

- Accumulate validity into masks instead of returning early on input-dependent
  byte classes.
- Avoid lookup tables indexed by input bytes or decoded 6-bit values.
- Decode all complete quanta for the public input length before reporting
  malformed input.
- Keep padding validation explicit and documented; padding length and final
  output length are public.
- Keep the implementation scalar and `unsafe`-free.
- Keep the module independent from future SIMD dispatch.

## Verification Requirements

Before documenting the guarantee as supported:

- Unit tests for all RFC 4648 vectors.
- Exhaustive short-input tests for all byte combinations practical under the
  test budget.
- Differential tests against the strict scalar decoder for canonical inputs.
- Malformed-input tests covering invalid bytes, mixed alphabets, padding, and
  non-canonical trailing bits.
- Miri coverage for the constant-time module.
- Generated-code review for supported release targets.
- A release note that states the exact guarantee and exclusions.

Until this evidence exists, README and SECURITY must continue to say that
`base64-ng` does not claim a formally verified cryptographic constant-time API.

## Memory Cleanup

Clear-tail in-place encode and decode APIs are intentionally separate from the
future constant-time API. They reduce ordinary caller-buffer retention but do
not provide a verified zeroization guarantee. Any future cryptographic profile
must document memory cleanup separately from timing behavior.
