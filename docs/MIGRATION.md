# Migrating from the `base64` Crate

This guide targets projects using `base64` `0.22.x`.

`base64-ng` is intentionally stricter and smaller. It does not try to mirror
every compatibility setting from `base64`; it provides a strict RFC 4648 scalar
core, caller-owned buffers, optional allocation helpers, and release evidence.

## Dependency

Before:

```toml
[dependencies]
base64 = "0.22"
```

After:

```toml
[dependencies]
base64-ng = "0.6.0"
```

For embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "0.6.0", default-features = false }
```

## Engine Mapping

| `base64` engine | `base64-ng` engine |
| --- | --- |
| `base64::engine::general_purpose::STANDARD` | `base64_ng::STANDARD` |
| `base64::engine::general_purpose::STANDARD_NO_PAD` | `base64_ng::STANDARD_NO_PAD` |
| `base64::engine::general_purpose::URL_SAFE` | `base64_ng::URL_SAFE` |
| `base64::engine::general_purpose::URL_SAFE_NO_PAD` | `base64_ng::URL_SAFE_NO_PAD` |

## Encoding

`base64`:

```rust
use base64::{Engine as _, engine::general_purpose::STANDARD};

let encoded = STANDARD.encode(b"hello");
assert_eq!(encoded, "aGVsbG8=");
```

`base64-ng` with allocation:

```rust
use base64_ng::STANDARD;

let encoded = STANDARD.encode_string(b"hello").unwrap();
assert_eq!(encoded, "aGVsbG8=");
```

`base64-ng` with caller-owned output:

```rust
use base64_ng::{STANDARD, checked_encoded_len};

let input = b"hello";
let mut output = vec![0u8; checked_encoded_len(input.len(), true).unwrap()];
let written = STANDARD.encode_slice(input, &mut output).unwrap();
output.truncate(written);

assert_eq!(output, b"aGVsbG8=");
```

## Decoding

`base64`:

```rust
use base64::{Engine as _, engine::general_purpose::STANDARD};

let decoded = STANDARD.decode("aGVsbG8=").unwrap();
assert_eq!(decoded, b"hello");
```

`base64-ng` with allocation:

```rust
use base64_ng::STANDARD;

let decoded = STANDARD.decode_vec(b"aGVsbG8=").unwrap();
assert_eq!(decoded, b"hello");
```

`base64-ng` with caller-owned output:

```rust
use base64_ng::{STANDARD, decoded_capacity};

let input = b"aGVsbG8=";
let mut output = vec![0u8; decoded_capacity(input.len())];
let written = STANDARD.decode_slice(input, &mut output).unwrap();
output.truncate(written);

assert_eq!(output, b"hello");
```

## Strictness Differences

`base64-ng` rejects ambiguous input by default:

- whitespace is not ignored
- mixed standard and URL-safe alphabets are rejected
- padding in the payload body is rejected
- trailing bytes after terminal padding are rejected
- non-canonical trailing bits are rejected
- padded engines require canonical padding

If the old project depends on line-wrapped or spaced Base64, use the explicit
legacy whitespace APIs:

```rust
use base64_ng::STANDARD;

let decoded = STANDARD.decode_vec_legacy(b" aG\r\nVs\tbG8= ").unwrap();
assert_eq!(decoded, b"hello");
```

The legacy profile only ignores ASCII space, tab, carriage return, and line
feed. It still rejects mixed alphabets, malformed padding, trailing payload
after padding, and non-canonical trailing bits. If the old project accepts
broader non-canonical input, normalize or reject that input before calling
`base64-ng`.

## Length And Memory Handling

`base64-ng` exposes recoverable length helpers:

```rust
use base64_ng::{checked_encoded_len, decoded_capacity};

assert_eq!(checked_encoded_len(5, true), Some(8));
assert_eq!(decoded_capacity(8), 6);
```

Use `checked_encoded_len` for untrusted length metadata before allocating.
Use `decode_slice` or `decode_in_place` when a caller-owned memory limit is
required.

## Streaming

Enable the `stream` feature for `std::io` wrappers:

```toml
[dependencies]
base64-ng = { version = "0.6.0", features = ["stream"] }
```

```rust
use std::io::Write;
use base64_ng::{STANDARD, stream::Encoder};

let mut encoder = Encoder::new(Vec::new(), STANDARD);
encoder.write_all(b"hello").unwrap();
let encoded = encoder.finish().unwrap();

assert_eq!(encoded, b"aGVsbG8=");
```

The `tokio` feature is reserved for future async wrappers. It is currently
inert and dependency-free; use the explicit `stream` feature for `std::io`
wrappers.

## Security Notes

The scalar encode/decode core has no external crate dependencies and remains
safe Rust. The only scalar-side unsafe code is the audited volatile wipe helper
used by clear-tail and secret-buffer cleanup APIs; architecture-specific unsafe
code remains limited to the dedicated SIMD boundary.
Release gates include tests, clippy, docs, dependency policy, RustSec audit,
license review, SBOM generation, reproducible package/build checks, and Miri
when installed.

`base64-ng` currently hardens obvious timing pitfalls in scalar encode/decode,
but it does not claim a formally verified cryptographic constant-time API.
