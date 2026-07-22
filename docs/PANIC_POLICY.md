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
new `panic!`, `unreachable!`, `assert!`, `.unwrap()`, or `.expect()` sites
unless they match an allowlisted pattern documented here. `debug_assert!` is
allowed for internal invariant documentation because it is not a release-mode
panic surface.
Root `src/*_tests.rs` files are exempt only when the validator can also prove
that each file is declared behind `#[cfg(test)]` in `src/lib.rs`; the filename
convention alone is not enough.
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
- `LineWrap::new` may panic when `line_len == 0`. It is intended for fixed,
  trusted values and profile constants. This remains true when the `const fn`
  is called at runtime: passing attacker-controlled or externally supplied
  zero values can unwind. Use `LineWrap::checked_new` when a line length comes
  from runtime configuration, file metadata, network input, a database row, or
  another untrusted source.
- `ExposedEncodedArray::from_array` and `ExposedDecodedArray::from_array` may
  panic when a caller supplies `len > CAP`. These constructors are explicit
  escape-hatch wrappers around caller-provided arrays; callers must pass the
  actual visible initialized length.
- Internal remainder matches use `_ => unreachable!()` after matching
  `len % 3` or equivalent remainder values. The preceding arithmetic bounds
  make those arms unreachable.
- `String::from_utf8` conversions after Base64 encoding use a reviewed
  `unreachable!` because crate encoders produce ASCII bytes by construction.
  Secret string conversions do not use this exception; if an already validated
  secret byte vector somehow fails UTF-8 conversion, the helper wipes the bytes
  and returns an empty string rather than panicking or using unchecked UTF-8.
- `encode_vec_infallible`, `encode_string_infallible`, profile variants, and
  the top-level `encode_infallible` may panic if the underlying fallible encode
  API returns an error, including encoded length overflow. On 32-bit targets,
  inputs larger than roughly 1.5 GiB can overflow the encoded length. These
  helpers are explicit convenience APIs for ordinary byte-to-Base64 paths where
  every byte sequence is encodable and a failure indicates an internal
  length/allocation invariant break. Do not use them for untrusted length
  metadata, constrained allocation environments, or code paths that require
  recoverable errors; use the fallible encode helpers instead.
- `base64-ng-sanitization`'s compatibility `SanitizationCtEqExt`
  implementations for locked containers panic when checked exposure reports
  canary corruption. This is an explicit fail-stop compatibility boundary:
  `Choice` cannot carry the integrity error, and returning `Choice::FALSE`
  would hide a security signal as an ordinary mismatch. High-assurance callers
  should use `LockedSanitizationCtEqExt` and choose their own alert, cleanup,
  and abort policy from the returned `CanaryCorruptedError`. A panic may unwind
  or be caught under some build and application policies, so this compatibility
  behavior is not itself a guaranteed process termination primitive.
- Core stream and Tokio writer `get_ref`, `get_mut`, and `into_inner` internal
  helpers use `unreachable!` if a wrapper has already consumed its inner value.
  The public API consumes `self` for `into_inner`/`finish`, so this state is not
  reachable through safe public calls.

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

Compile-time array decoding is intentionally `Result`-based:
`Engine::decode_array` reports malformed input, padding errors, and undersized
output arrays with `DecodeError` rather than adding another reviewed panic
exception. It is still a strict decoder, not a constant-time-oriented secret
decoder.
