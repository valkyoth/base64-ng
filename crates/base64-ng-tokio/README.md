# base64-ng-tokio

Optional Tokio helpers for `base64-ng`.

The current `1.2.3` companion crate provides async convenience helpers: it
reads the input into memory, validates/encodes or validates/decodes, then
writes the output. Use the `*_limited` variants when input size is controlled
by a peer or request boundary. Full cancellation-audited streaming adapters
remain a future admission item.

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::encode_reader_to_writer_limited;

let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 1024).await.unwrap();
assert_eq!(output, b"aGVsbG8=");
```
