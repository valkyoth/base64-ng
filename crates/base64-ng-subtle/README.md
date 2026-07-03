# base64-ng-subtle

Optional `subtle::ConstantTimeEq` integration for `base64-ng`.

The core `base64-ng` crate stays zero-runtime-dependency. This companion crate
is for applications that already admit the `subtle` crate and want explicit
comparison helpers for decoded or encoded Base64 material.

```toml
[dependencies]
base64-ng = "1.3.4"
base64-ng-subtle = "1.3.4"
```

```rust
use base64_ng::ct;
use base64_ng_subtle::SubtleEqExt;

let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
assert!(decoded.subtle_verify(b"hello"));
```

Length is public: mismatched lengths return `Choice::from(0)` immediately.
Use fixed-size protocol tokens when length must not vary.
