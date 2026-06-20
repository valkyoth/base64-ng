# base64-ng-tokio

Optional Tokio helpers for `base64-ng`.

The current `1.0.9` companion crate provides bounded async convenience helpers:
it reads the input into memory, validates/encodes or validates/decodes, then
writes the output. Full cancellation-audited streaming adapters remain a future
admission item.

```rust
use base64_ng::STANDARD;
use base64_ng_tokio::encode_reader_to_writer;

let mut input = &b"hello"[..];
let mut output = Vec::new();
encode_reader_to_writer(&STANDARD, &mut input, &mut output).await.unwrap();
assert_eq!(output, b"aGVsbG8=");
```
