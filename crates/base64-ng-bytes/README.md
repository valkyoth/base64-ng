# base64-ng-bytes

Optional `bytes` integration for `base64-ng`.

This companion crate provides explicit helpers for services that already use
`bytes::Bytes`, `bytes::BytesMut`, `bytes::Buf`, and `bytes::BufMut`.

```rust
use base64_ng::STANDARD;
use base64_ng_bytes::EngineBytesExt;

let encoded = STANDARD.encode_bytes(b"hello").unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");

let decoded = STANDARD.decode_bytes(encoded).unwrap();
assert_eq!(&decoded[..], b"hello");
```
