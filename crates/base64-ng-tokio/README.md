<p align="center">
  <b>async Tokio readers, writers, and bounded helpers for base64-ng.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-tokio">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/PLAN.md">Roadmap</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/TRUST.md">Trust Dashboard</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/SECURITY.md">Security</a>
</div>

<br>

<p align="center">
  <a href="https://github.com/valkyoth/base64-ng">
    <img src="https://raw.githubusercontent.com/valkyoth/base64-ng/main/.github/images/base64-ng.webp" alt="base64-ng Rust crate overview">
  </a>
</p>

# base64-ng-tokio

Optional Tokio helpers for `base64-ng`.

The read-all helpers and manual `AsyncRead`/`AsyncWrite` adapters use the shared
2.0 incremental core. Reader `new` consumes through EOF; `new_exact` transforms
one declared frame and leaves adjacent bytes unread. Writers preserve bounded
queued output across short writes, `Pending`, cancellation, and retryable I/O
errors. Call `shutdown` to finalize a trailing quantum.

On unwind-capable builds, a panic from a wrapped reader or writer is resumed
only after the adapter latches failure and clears retained state. Downstream
bytes consumed before a panic remain irrevocable; retry through that adapter is
therefore prohibited. `panic=abort` is outside this cleanup boundary.

Read-all helper allocations are RAII-wiped on success, error, and cancellation.
Limited helpers consume no more than the configured limit plus one lookahead
byte used to detect overflow. Use `new_exact` when an adjacent frame's first
byte must remain unread. Their eager
allocation is capped at 8 KiB so cleanup work stays proportional to accepted
input rather than the caller's maximum alone. Guarded vector growth wipes each
replaced allocation before it is returned to the allocator. Collection,
incremental transformation, and output delivery consume Tokio cooperative
budget between bounded chunks. This keeps always-ready custom readers and
short-writing writers from monopolizing a runtime worker, but unlimited helpers
still require a trusted finite source. Output delivery checks budget before the
next irreversible write; after the final write commits the complete frame, the
helper returns success without another internal cancellation point.

```rust
use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader, EncoderWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(
    &STRICT_STANDARD_PADDED,
    &mut input,
    &mut output,
    1024,
).await.unwrap();
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new_exact(
    &b"helloNEXT"[..],
    &STRICT_STANDARD_PADDED,
    5,
);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await.unwrap();
assert_eq!(streamed, b"aGVsbG8=");

let mut writer = EncoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
writer.write_all(b"hello").await.unwrap();
writer.shutdown().await.unwrap();
assert_eq!(writer.into_inner(), b"aGVsbG8=");
```
