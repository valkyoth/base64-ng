<p align="center">
  <b>bounded bytes and Buf helpers for base64-ng.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-bytes">Docs.rs</a>
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
