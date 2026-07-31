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

## 2.0 Bounded Arrays

- `EncodedArray<CAP>` and `DecodedArray<CAP>` construct only when their private
  visible length is at most `CAP`; `remaining_capacity` therefore cannot
  underflow.
- Codec-produced ordinary arrays initialize the complete backing array to zero
  before writing the visible prefix. Public `from_array` preserves the
  caller-supplied tail but never exposes it through `as_bytes`.
- Ordinary arrays are `Copy` and have no cleanup destructor. They make no
  secrecy claim.
- `SecretArray<CAP>` is non-Clone and non-Copy, wipes caller-supplied tail
  capacity during construction, and wipes the complete backing array on
  invalid construction, explicit clear, and drop.
- Const transforms validate malformed input before exact output-length
  comparison and never partially return an output array.

Evidence:

- exhaustive const single-quantum differential tests
- external const compile and compile-fail fixtures
- bounded-array Kani constructor invariant harness
- secret redaction, tail cleanup, clear, and structural Drop policy tests

## 2.0 In-Place Transforms

- Reverse encode copies each source group before writing its expanded group;
  every destination cursor remains at or after the unread source cursor.
- Forward decode validates before mutation and maintains `write <= read` after
  every consumed quantum or tail.
- Ordinary in-place preflight and input errors leave the complete buffer
  unchanged; no recoverable error exists after mutation begins.
- Secret staged decode checks complete byte-range disjointness and capacity
  before scanning input. Preflight errors leave both ranges unchanged.
- Invalid secret input leaves caller-visible encoded bytes unchanged and wipes
  complete staging. Internal faults wipe both complete ranges.
- The fixed-work secret claim ends at the result gate; successful plaintext
  release is an explicitly public, success-only copy.

Evidence:

- exhaustive bounded ordinary in-place differential tests
- strict secret mutation differential tests and work counters
- checked overlap/address geometry tests
- injected backend-fault cleanup tests
- Kani cursor proofs, Miri tests, and AddressSanitizer tests
- production and Kani cursors share the same tail and quantum length helpers,
  guarded by source-coupling and exhaustive helper-domain tests

## 2.0 Formatting, Append, And Chunks

- Lazy display and formatter encoding synthesize at most one four-byte quantum
  at a time and allocate no heap storage.
- Formatter progress counts only bytes from fully successful `write_str`
  calls; a failing call may have performed unobservable partial mutation.
- A counted sink reports exact accepted bytes and accepts zero bytes on `Err`;
  zero progress and over-reported counts are contract failures.
- Append guards preserve the original prefix and restore entry length on every
  returned crate error and unwinding panic.
- Encoded chunk items own synthesized output. The iterator borrows plaintext
  input and copies validated codec settings.

Evidence:

- bounded chunk differential tests for built-in and runtime codecs
- adversarial formatter and counted-sink tests
- injected reserve, crate-error, and unwind rollback tests
- integration-test global allocation counter
- external API and compile-fail lifetime checks

## 2.0 WHATWG Forgiving Decode

- The WHATWG state retains at most four input symbols and three pending output
  bytes; input and pending output quantums are never both unresolved.
- Only WHATWG ASCII whitespace is ignored. Vertical tab is not whitespace.
- `=` can enter only the third or fourth quantum position. Once a padded
  quantum completes, only ignored whitespace may follow.
- Final unpadded tails contain exactly two or three Standard symbols. Unused
  trailing bits are deliberately discarded as required by WHATWG.
- One-shot decode validates and measures through an independent state instance
  before writing caller output.
- Failure is absorbing until reset. Web content errors are opaque and cannot
  expose source bytes or indexes.

Evidence:

- locked WHATWG/browser fixture corpus
- every-split incremental tests with one-byte output
- transactional sentinel tests and no-allocation builds
- Node/V8, Chromium, Firefox, and Safari browser scripts
- strict-separation and expert-policy differential tests

## 2.0 Legacy Compatibility Profiles

- `legacy::ASCII_WHITESPACE` ignores only space, tab, carriage return, and
  line feed. Every other byte is semantically significant.
- Legacy progress and diagnostics count original source bytes, never compacted
  positions. Position overflow is checked before compaction and is absorbing.
- Legacy and protocol-body compatibility is ordinary, detailed-error,
  non-wiping behavior and is unreachable from `secret::*`.
- Named `*_ALPHABET_*` values provide alphabet-level Base64 only. Named
  `*_BODY_*` values provide encoded-body layout only. Neither naming family
  claims complete container or record parsing.

## Review Rule

When adding new indexing in non-test code, prefer `get`, slice-pattern
matching, checked arithmetic, or a small helper that carries the local
invariant. If direct indexing remains clearer, update this document and add a
focused test, Kani harness, or policy check that justifies the bound.
