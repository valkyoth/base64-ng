# Migrating To base64-ng 2.0

This guide covers migration from `base64-ng` 1.3.9 and from the `base64`
crate's ordinary Standard and URL-safe engines. The 2.0 API makes strictness,
destination mutation, compatibility parsing, secret handling, and protocol
scope visible at the call site.

The examples are compiled by:

```sh
scripts/check-2.0-migration-smoke.sh
scripts/check_migration_smoke.sh
```

The first command exercises the canonical 2.0 surface. The second proves that
the reviewed 1.x compatibility inventory still compiles; compatibility names
are not the recommended API for new 2.0 code.

## Dependency

```toml
[dependencies]
base64-ng = "2.0.0"
```

For `no_std` without allocation:

```toml
[dependencies]
base64-ng = { version = "2.0.0", default-features = false }
```

Enable ordinary runtime SIMD with `features = ["simd"]`, synchronous
`std::io` adapters with `features = ["stream"]`, and secret owners with
`features = ["secrets"]`. Secret operations remain scalar even when ordinary
SIMD is enabled.

## Strict Presets

Use the explicit RFC 4648 presets:

| Policy | 2.0 value |
| --- | --- |
| Standard, canonical padding required | `STRICT_STANDARD_PADDED` |
| Standard, padding forbidden | `STRICT_STANDARD_UNPADDED` |
| URL-safe, canonical padding required | `STRICT_URL_SAFE_PADDED` |
| URL-safe, padding forbidden | `STRICT_URL_SAFE_UNPADDED` |

The historical `STANDARD`, `STANDARD_NO_PAD`, `URL_SAFE`, and
`URL_SAFE_NO_PAD` names remain in the reviewed compatibility inventory, but
new code should use the explicit strict names.

Strict decode rejects whitespace, mixed alphabets, impossible lengths,
misplaced or forbidden padding, trailing data after terminal padding, and
noncanonical trailing bits.

## One-Shot Operations

The canonical caller-owned methods are transactional. A returned error leaves
the complete destination unchanged.

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let mut encoded = [0xa5; 12];
let written = STRICT_STANDARD_PADDED
    .encode_into(b"hello", &mut encoded)
    .unwrap();
assert_eq!(&encoded[..written], b"aGVsbG8=");
assert_eq!(&encoded[written..], &[0xa5; 4]);

let before = encoded;
assert!(STRICT_STANDARD_PADDED
    .decode_into(b"!!!!", &mut encoded)
    .is_err());
assert_eq!(encoded, before);
```

With `alloc`, use `encode_to_string`, `decode_to_vec`, `encode_append`, and
`decode_append`. Append operations roll back the appended suffix on returned
errors and supported unwinding; they cannot undo side effects in arbitrary
formatters or I/O sinks.

| 1.x name | Canonical 2.0 name |
| --- | --- |
| `encode_slice` | `encode_into` |
| `decode_slice` / `decode_slice_clear_tail` | `decode_into` |
| `encode_string` | `encode_to_string` |
| `decode_vec` | `decode_to_vec` |
| `encode_buffer` | `encode_bounded` or `encode_array` |
| `decode_buffer` | `decode_bounded` or `decode_array` |

## Incremental And In-Place Operations

`codec.encoder()` and `codec.decoder()` are allocation-free state machines.
Every update returns explicit consumed/produced progress and a `Status`.
Committed output from an incremental operation is not transactional; after an
error, only the previously reported prefix is observable.

```rust
use base64_ng::{Status, STRICT_STANDARD_PADDED};

let mut state = STRICT_STANDARD_PADDED.encoder();
let mut output = [0u8; 8];
let step = state.update(b"hello", &mut output).unwrap();
let mut written = step.progress().output_produced();
let final_step = state.finish(&mut output[written..]).unwrap();
written += final_step.progress().output_produced();
assert_eq!(final_step.status(), Status::Complete);
assert_eq!(&output[..written], b"aGVsbG8=");
```

The 2.0 in-place methods return initialized lengths. Ordinary in-place decode
may expose a committed prefix before a later error. Use `decode_into` when the
destination must remain unchanged, or `decode_in_place_staged` for the
validated-before-mutation secret-adjacent contract.

```rust
use base64_ng::STRICT_STANDARD_PADDED;

let mut buffer = [0u8; 16];
buffer[..6].copy_from_slice(b"secret");
let encoded = STRICT_STANDARD_PADDED.encode_in_place(&mut buffer, 6).unwrap();
let decoded = STRICT_STANDARD_PADDED
    .decode_in_place(&mut buffer, encoded)
    .unwrap();
assert_eq!(&buffer[..decoded], b"secret");
```

## Compatibility Policies

Compatibility behavior is never enabled by a general permissive switch:

- `web::FORGIVING` implements the exact ordinary WHATWG forgiving policy.
- `legacy::ASCII_WHITESPACE` ignores only space, tab, CR, and LF around the
  otherwise strict codec supplied by the caller.
- `compat::*` contains explicitly named padding-indifferent and
  noncanonical-trailing-bit expert policies.
- `MIME_BODY_STRICT`, `PEM_BODY_LF`, and `PEM_BODY_CRLF` are body-formatting
  conveniences, not complete protocol parsers.

Compatibility codecs are ordinary APIs. They are intentionally rejected by
the secret API.

## Custom Alphabets And Specifications

Replace hand-written `Alphabet` implementations with `ValidatedAlphabet` or a
const validated alphabet. `CodecBuilder` validates padding and trailing-bit
policies before constructing an immutable `Base64<RuntimeSpec>` value. Invalid
specifications cannot be constructed through safe APIs.

Named alphabet-level values such as `BCRYPT_ALPHABET_NO_PAD`,
`PBKDF2_ALPHABET_NO_PAD`, `IMAP_MUTF7_ALPHABET_NO_PAD`, and `BINHEX_ALPHABET`
do not imply complete surrounding protocols.

## Secret Operations

The 1.x `ct::*`, `SecretBuffer`, and `encode_secret`/`decode_secret` names are
compatibility surfaces. New code enables `secrets` and uses `secret::*` with a
strict eligible codec:

```rust
use base64_ng::{
    secret::{SecretArrayFrame, SecretInput},
    STRICT_STANDARD_PADDED,
};

let input = SecretInput::new(b"aGVsbG8=");
let mut frame = SecretArrayFrame::<5>::new(&STRICT_STANDARD_PADDED).unwrap();
frame.update(&input).unwrap();
let secret = frame.finish().unwrap();
assert_eq!(secret.expose_secret().as_bytes(), b"hello");
```

Secret errors are opaque, frames are bounded, output is withheld until the
result gate, and crate-owned initialized storage is wiped on ordinary drop and
supported unwind paths. This is constant-time-oriented engineering, not a
formal cryptographic constant-time guarantee. Abort, process termination,
deliberate destructor suppression, caller-owned input, swap, hibernation,
crash dumps, and historical register/cache copies remain outside the cleanup
claim.

`BestEffortProvider` is finite, volatile, generation-scoped, and in-process.
Its journal orders bounded teardown retries only. Base 2.0 ships no persistent
provider and makes no restart- or crash-recovery claim.

## Streaming And Ecosystem Companions

The core `stream` feature keeps the synchronous `std::io` adapters. Async and
ecosystem integrations are separate synchronized packages:

```toml
[dependencies]
base64-ng = "2.0.0"
base64-ng-tokio = "2.0.0"
base64-ng-bytes = "2.0.0"
base64-ng-serde = "2.0.0"
```

The Tokio and bytes state machines expose accepted/committed progress,
bounded pending buffers, fail-closed post-error states, and checked recovery.
Panics from caller-provided I/O or trait implementations may propagate after
crate-owned state is latched and cleaned; side effects already performed by
the caller implementation cannot be rolled back.

## Protocol Companions

Use the scoped package instead of combining ordinary Base64 helpers into a
partial protocol parser:

| Package | Boundary |
| --- | --- |
| `base64-ng-mime` | RFC 2045 Section 6.8 transfer bodies |
| `base64-ng-pem` | RFC 7468 textual encoding |
| `base64-ng-openpgp` | RFC 9580 ASCII armor |
| `base64-ng-multibase` | Four pinned Base64-family multibase entries |
| `base64-ng-imap` | RFC 3501 modified-Base64 payload bytes only |
| `base64-ng-password` | Passlib PBKDF2 and SHA-crypt fields/records only |

Each parser is bounded and has an explicitly named strict or compatible
policy. Locked RFC/specification sources are retained in the repository but
excluded from published packages.

## Sanitization 2.0

`base64-ng-sanitization` 2.0 uses exact-pinned `sanitization` 2.0.3. Prefer its
2.0 `SanitizationProtectedDecodeExt` methods for protected fixed or bounded
dynamic decode. Required memory-lock, dump, and fork controls are established
before classified input reaches staging. The core `decode_assured` provider
path is the only single-allocation route carrying core generation, quarantine,
and fallible-teardown claims.

## Final Checklist

1. Replace ordinary engine calls with an explicit strict `Base64<Spec>` value.
2. Choose transactional, incremental, in-place, or sink semantics explicitly.
3. Move permissive parsing to a named `web`, `legacy`, `compat`, or protocol
   policy.
4. Move secrets to `secret::*` and keep declassification explicit.
5. Enforce input and output ceilings before allocation or protected mapping.
6. Treat caller callback/I/O panics and already committed external side effects
   as application boundaries.
