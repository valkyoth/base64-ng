# Panic Policy

`base64-ng` treats runtime panics in public scalar APIs as denial-of-service
risk. Public encode, decode, validation, length, in-place, streaming, and
profile APIs should report malformed input and size failures with `Result` or
`Option` rather than unwinding.

This policy is enforced by:

```sh
scripts/validate-panic-policy.sh
```

The validator scans non-test source before Kani/test-only modules and fails on
new `panic!`, `unreachable!`, `.unwrap()`, or `.expect()` sites unless they
match an allowlisted pattern documented here.
Remaining bounded indexing invariants are documented in
[INVARIANTS.md](INVARIANTS.md).

## Allowed Non-Test Sites

The current reviewed exceptions are:

- `Engine::encode_array` may panic during const evaluation when the caller
  supplies an output array length that does not match the compile-time encoded
  length, or when that const length calculation overflows. This is documented
  as a const-array API contract and is not used for runtime untrusted length
  metadata. Calling it at runtime with a mismatched const output length can
  also unwind; do not route attacker-controlled sizing decisions through this
  API.
- Internal remainder matches use `_ => unreachable!()` after matching
  `len % 3` or equivalent remainder values. The preceding arithmetic bounds
  make those arms unreachable.
- `String::from_utf8` conversions after Base64 encoding use `unreachable!`
  because crate encoders produce ASCII bytes by construction.
- Stream `get_ref`, `get_mut`, and `into_inner` internal helpers use
  `unreachable!` if a wrapper has already consumed its inner value. The public
  API consumes `self` for `into_inner`/`finish`, so this state is not reachable
  through safe public calls.

Test code, doctest examples, and Kani proof harnesses may use `unwrap`,
`expect`, or panic-like macros when they are asserting expected outcomes.

## Caller Guidance

For untrusted input and untrusted length metadata, prefer:

- `checked_encoded_len`
- `encoded_len`
- `checked_wrapped_encoded_len`
- `wrapped_encoded_len`
- `decoded_capacity`
- `decoded_len`
- caller-owned `encode_slice` and `decode_slice`
- in-place APIs that return `Result`

Compile-time array encoding is intentionally stricter: an incorrect destination
array length fails const evaluation so the mistake cannot silently produce a
truncated or oversized static value.

Use `Engine::encode_array` for fixed-size static data and compile-time checked
arrays. For runtime data, especially data sized from packet headers, file
metadata, network frames, or other untrusted sources, use checked length helpers
and caller-owned slice APIs instead.
