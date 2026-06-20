# base64-ng-sanitization

Optional `sanitization` integration helpers for `base64-ng`.

This companion crate keeps `base64-ng` itself dependency-free while giving
applications that already use `sanitization` a direct path from
constant-time-oriented Base64 decode into clear-on-drop secret containers.

```toml
[dependencies]
base64-ng = { version = "1.0.9", default-features = false }
base64-ng-sanitization = { version = "1.0.9", default-features = false }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationExt;

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

secret.expose_secret(|bytes| assert_eq!(bytes, b"hello"));
```

Enable `alloc` for heap-backed `sanitization::SecretVec` helpers:

```toml
base64-ng-sanitization = { version = "1.0.9", features = ["alloc"] }
```

The integration intentionally targets `base64_ng::ct::CtEngine`. Strict
non-CT decoders remain available in `base64-ng`, but this crate keeps the
secret-container API pointed at the constant-time-oriented decode path.
