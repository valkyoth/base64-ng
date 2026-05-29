# Internal Bounds Invariants

This document records the local invariants that justify bounded indexing in
non-test scalar code. It is part of the `v0.11` panic-policy hardening work.

`base64-ng` does not use unchecked indexing in public APIs. Remaining safe
indexing is accepted only when one of the invariants below applies and is
covered by tests, Kani harnesses, or a local preflight check.

## Chunk Reads

- Four-byte Base64 quanta are read through `read_quad`, which uses checked
  offset arithmetic and `slice::get`.
- Unpadded tail decode uses slice-pattern matching for `[]`, `[b0, b1]`, and
  `[b0, b1, b2]`.
- Wrapped and legacy decoders compact or validate input before forwarding to
  the same strict chunk and tail routines.

Evidence:

- `decode_chunk_bit_packing_matches_exhaustive_small_inputs`
- `decode_chunk_bit_packing_matches_representative_full_quanta`
- Kani harnesses for `decode_chunk` bounds and unpadded tail decode bounds

## Output Writes

- Slice encode/decode functions compute or validate the required output length
  before writing.
- Full decode quanta write three bytes only after the destination is known to
  have enough capacity for the decoded length.
- Tail decode writes one or two bytes only after `first_mut` or checked
  two-byte mutable slice access succeeds.
- Clear-tail variants use the same checked decode path and then wipe the
  caller-provided unused output region or the entire buffer on failure.

Evidence:

- output-too-small tests for slice APIs
- clear-tail cleanup tests
- Kani harnesses for slice encode/decode output-prefix bounds

## In-Place Decode

- Strict in-place decode writes toward the front of the buffer.
- For each full Base64 quantum, three decoded bytes replace four input bytes,
  so `write <= read` after the first quantum and never overtakes unread input.
- Legacy and wrapped in-place decode compact accepted input first, then call
  the same strict decode-to-front path.
- Constant-time-oriented non-clear-tail in-place decode is destructive on
  error and is deprecated for sensitive payloads. It may leave decoded
  plaintext at the front of the buffer and unrecoverably overwrite part of the
  encoded input. Use `ct::CtEngine::decode_in_place_clear_tail` when an error
  should leave a known-zero buffer.

Evidence:

- in-place strict, legacy, and wrapped round-trip tests
- clear-tail in-place failure tests
- Kani in-place decode prefix-bound harnesses

## In-Place Encode

- In-place encode validates `input_len <= buffer.len()` and checks the required
  encoded length before writing.
- Encoding writes from the back of the output region toward the front, so
  unread input bytes are not overwritten.

Evidence:

- in-place encode equivalence tests
- encode error non-panic tests
- Kani encode in-place prefix-bound harness

## Alphabet Tables

- Alphabet encode tables contain exactly 64 bytes.
- Runtime and macro-defined custom alphabets are validated for uniqueness,
  visible ASCII, and absence of padding.
- Built-in encoders use arithmetic mapping. Custom alphabet default encoding
  uses a fixed 64-entry scan instead of secret-indexed table lookup.
- Constant-time-oriented generic decode scans all 64 alphabet entries for every
  input byte.

Evidence:

- custom alphabet validation tests
- generic ct alphabet tests
- release-gated constant-time policy checks

## Constant-Time-Oriented Decode

- The `ct` module treats input length, padding length, decoded length, and
  success/failure as public facts.
- Full unpadded quanta use `read_quad`; remaining tail bytes use checked tail
  access and public length dispatch.
- Malformed-content errors are accumulated and reported as an opaque error to
  avoid localizing the first malformed byte in the ct path.
- Internal CT loop guard failures use debug assertions during development and
  fail closed to `DecodeError::InvalidInput` in release builds by setting the
  accumulated invalid-input masks. This creates a deliberate debug/release
  diagnostic difference: debug builds catch invariant violations loudly, while
  release builds avoid panicking on sensitive decode paths.

Evidence:

- ct validate/decode agreement tests
- dudect harness compilation and opt-in timing run
- generated assembly evidence script
- Kani ct validate/decode one-quantum agreement harness

## Stream Buffers

- Streaming adapters keep pending input and queued output in explicit staging
  buffers.
- Decode readers do not read past terminal padding before exposing the terminal
  state to the caller.
- Drop implementations wipe initialized internal staging buffers on a
  best-effort basis.

Evidence:

- stream chunk-boundary tests
- stream trailing-input-after-padding tests
- stream retry/fail-closed tests

## Review Rule

When adding new indexing in non-test code, prefer `get`, slice-pattern
matching, checked arithmetic, or a small helper that carries the local
invariant. If direct indexing remains clearer, update this document and add a
focused test, Kani harness, or policy check that justifies the bound.
