# base64-ng-tokio

Optional Tokio helpers for `base64-ng`.

The current `1.2.3` companion crate provides async convenience helpers and
manual `AsyncRead` streaming adapters. Use the `*_limited` helper variants when
input size is controlled by a peer or request boundary. Async writer adapters
remain a future admission item because `poll_write` accepted-byte and
backpressure semantics require a separate review.

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::{encode_reader_to_writer_limited, EncoderReader};
use tokio::io::AsyncReadExt;

let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 1024).await.unwrap();
assert_eq!(output, b"aGVsbG8=");

let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
let mut streamed = Vec::new();
reader.read_to_end(&mut streamed).await.unwrap();
assert_eq!(streamed, b"aGVsbG8=");
```
