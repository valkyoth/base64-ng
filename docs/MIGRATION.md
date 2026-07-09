# Migrating from the `base64` Crate

This guide targets projects using `base64` `0.22.x`.

`base64-ng` is intentionally stricter and smaller. It does not try to mirror
every compatibility setting from `base64`; it provides a strict RFC 4648 scalar
core, caller-owned buffers, optional allocation helpers, and release evidence.

The migration examples are covered by a local smoke crate:

```sh
scripts/check_migration_smoke.sh
```

The standard release gate runs this script so strict standard, URL-safe no-pad,
MIME/PEM wrapping, legacy whitespace, custom alphabets, stack buffers, secret
buffers, and stream wrapper migration examples stay in sync with the crate.

## Dependency

Before:

```toml
[dependencies]
base64 = "0.22"
```

After:

```toml
[dependencies]
base64-ng = "1.3.6"
```

For embedded or freestanding use:

```toml
[dependencies]
base64-ng = { version = "1.3.6", default-features = false }
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
use base64_ng::{
    LineEnding, LineWrap, checked_encoded_len, checked_wrapped_encoded_len, decoded_capacity,
};

assert_eq!(checked_encoded_len(5, true), Some(8));
assert_eq!(
    checked_wrapped_encoded_len(5, true, LineWrap::new(4, LineEnding::Lf)),
    Some(9)
);
assert_eq!(decoded_capacity(8), 6);
```

Use `checked_encoded_len` for untrusted length metadata before allocating.
Use `decode_slice` or `decode_in_place` when a caller-owned memory limit is
required.

## Streaming

Enable the `stream` feature for `std::io` wrappers:

```toml
[dependencies]
base64-ng = { version = "1.3.6", features = ["stream"] }
```

```rust
use std::io::Write;
use base64_ng::{STANDARD, stream::Encoder};

let mut encoder = Encoder::new(Vec::new(), STANDARD);
assert_eq!(encoder.engine(), STANDARD);
assert!(encoder.is_padded());

encoder.write_all(b"he").unwrap();
assert!(encoder.has_pending_input());

encoder.write_all(b"llo").unwrap();
assert!(encoder.has_pending_input());

encoder.try_finish().unwrap();
assert!(encoder.is_finalized());

let encoded = encoder.finish().unwrap();

assert_eq!(encoded, b"aGVsbG8=");
```

Writer adapters expose `try_finish()` when a caller wants to finalize pending
Base64 input and flush the wrapped writer without immediately consuming the
adapter. After successful finalization, later non-empty writes return
`InvalidInput`. Writer adapters buffer encoded or decoded output internally
before draining it into the wrapped writer, so failed wrapped writes can be
retried by calling `flush()` or `try_finish()` again without re-encoding or
re-decoding accepted input. Direct `write()` calls may report partial progress;
use `write_all()` when the whole input slice must be consumed. Stream adapters
also expose non-sensitive state helpers such as
`engine()`, `is_padded()`, `pending_len()`, `has_pending_input()`,
`pending_input_needed_len()`, `buffered_output_len()`,
`buffered_output_capacity()`, `buffered_output_remaining_capacity()`, and
`has_finished_input()`, and decoder-side `has_terminal_padding()` for framed
protocols and audit logging. Use `can_into_inner()` and `try_into_inner()` when
recovering the wrapped reader or writer should be refused if it would discard
pending input or buffered output. Decoder writer and reader adapters also
expose `is_failed()` and fail closed after malformed Base64 input; unchecked
`into_inner()` remains available for explicit recovery of the wrapped object
after a decode error.

The core crate's `tokio` feature is reserved, inert, and dependency-free. For
Tokio applications, use the optional `base64-ng-tokio` companion crate instead:

```toml
[dependencies]
base64-ng = "1.3.6"
base64-ng-tokio = "1.3.6"
tokio = { version = "1.52.3", features = ["io-util"] }
```

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader, EncoderWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

# async fn example() -> std::io::Result<()> {
let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 1024).await?;
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await?;
assert_eq!(streamed, b"aGVsbG8=");

let mut writer = EncoderWriter::new(Vec::new(), STANDARD);
writer.write_all(b"hello").await?;
writer.shutdown().await?;
let encoded = writer.into_inner()?;
assert_eq!(encoded, b"aGVsbG8=");
# Ok(())
# }
```

## Security Notes

The scalar encode/decode core has no external crate dependencies and remains
safe Rust. The only scalar-side unsafe code is the audited volatile wipe helpers
used by clear-tail and secret-buffer cleanup APIs; architecture-specific unsafe
code remains limited to the dedicated SIMD boundary.
Release gates include tests, clippy, docs, dependency policy, RustSec audit,
license review, SBOM generation, reproducible package/build checks, and Miri
when installed.

`base64-ng` currently hardens obvious timing pitfalls in scalar encode/decode,
but it does not claim a formally verified cryptographic constant-time API.
