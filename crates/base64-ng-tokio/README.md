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

The current companion crate provides async convenience helpers plus manual
`AsyncRead` and `AsyncWrite` streaming adapters. Use the `*_limited` helper
variants when input size is controlled by a peer or request boundary. Writer
shutdown is the finalization boundary: call `AsyncWriteExt::shutdown` to encode
or validate a final partial quantum.

Read-all helper allocations are RAII-wiped on success, error, and cancellation.
Limited helpers consume no more than the configured limit plus one lookahead
byte used to detect overflow. Use a separately bounded reader or a streaming
adapter when an adjacent frame's first byte must remain unread.

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader, EncoderWriter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 1024).await.unwrap();
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await.unwrap();
assert_eq!(streamed, b"aGVsbG8=");

let mut writer = EncoderWriter::new(Vec::new(), STANDARD);
writer.write_all(b"hello").await.unwrap();
writer.shutdown().await.unwrap();
assert_eq!(writer.into_inner(), b"aGVsbG8=");
```
