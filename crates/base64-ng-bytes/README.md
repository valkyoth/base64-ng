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

For peer-controlled `Buf` values, prefer the bounded helpers so a custom
`Buf::remaining()` value cannot drive an unbounded temporary allocation:

```rust
use base64_ng::STANDARD;
use base64_ng_bytes::EngineBytesExt;
use bytes::Bytes;

let encoded = STANDARD
    .encode_buf_limited(Bytes::from_static(b"hello"), 5)
    .unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");
```
