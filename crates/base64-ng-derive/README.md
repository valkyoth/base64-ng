# base64-ng-derive

Dependency-free derive helpers for fixed-size `base64-ng` byte newtypes.

This companion crate keeps the core `base64-ng` package free of proc-macro
dependencies. It supports one deliberately narrow shape:

```rust
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
struct ApiKey([u8; 32]);
```

The generated impls provide:

- `from_base64(&[u8])` and `from_base64_str(&str)` using
  `base64_ng::ct::STANDARD.decode_slice_staged_clear_tail`.
- `encode_base64::<CAP>()` using strict standard padded Base64.
- `as_bytes()`, `as_mut_bytes()`, and `constant_time_eq(&Self)`.
- redacted `Debug`.
- drop-time cleanup through `base64_ng::clear_bytes`.

The macro intentionally does not support named structs, multiple fields,
generic structs, or non-array storage. Use `base64-ng-sanitization` when you
want integration with a dedicated clear-on-drop secret container.
