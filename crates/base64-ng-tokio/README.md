# base64-ng-tokio

Optional Tokio helpers for `base64-ng`.

The current companion crate provides async convenience helpers plus manual
`AsyncRead` and `AsyncWrite` streaming adapters. Use the `*_limited` helper
variants when input size is controlled by a peer or request boundary. Writer
shutdown is the finalization boundary: call `AsyncWriteExt::shutdown` to encode
or validate a final partial quantum.

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
