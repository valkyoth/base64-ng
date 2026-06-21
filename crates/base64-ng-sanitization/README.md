# base64-ng-sanitization

Optional `sanitization` integration helpers for `base64-ng`.

This companion crate keeps `base64-ng` itself dependency-free while giving
applications that already use `sanitization` a direct path from
constant-time-oriented Base64 decode into clear-on-drop secret containers.

```toml
[dependencies]
base64-ng = { version = "1.2.1", default-features = false }
base64-ng-sanitization = { version = "1.2.1", default-features = false }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

assert!(secret.sanitization_verify(
    b"hello",
    "example compares public expected bytes"
));
```

Enable `alloc` for heap-backed `sanitization::SecretVec` helpers:

```toml
base64-ng-sanitization = { version = "1.2.1", features = ["alloc"] }
```

For high-assurance native deployments, enable locked storage helpers. This
uses `sanitization` 1.2.1's `memory-lock`, `canary-check`, and
`random-canary` features and decodes directly into locked memory:

```toml
base64-ng-sanitization = { version = "1.2.1", features = ["high-assurance"] }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationExt;

let key = ct::STANDARD
    .decode_locked_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

key.with_secret(|bytes| assert_eq!(bytes, b"hello"));
```

For dynamic output on supported native targets:

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationExt;

let key = ct::STANDARD
    .decode_locked_secret_vec(b"aGVsbG8=")
    .unwrap();

key.with_secret(|bytes| assert_eq!(bytes, b"hello"));
```

The integration intentionally targets `base64_ng::ct::CtEngine`. Strict
non-CT decoders remain available in `base64-ng`, but this crate keeps the
secret-container API pointed at the constant-time-oriented decode path.

`base64-ng-sanitization` also re-exports `sanitization::ct` and adds
`SanitizationCtEqExt` for comparing decoded `SecretBytes` and `SecretVec`
values through `sanitization` 1.2.1's native `Choice` API. This gives projects
that already admit `sanitization` a dependency-free alternative to external
`subtle` integration:

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

let equal = secret.sanitization_ct_eq(b"hello");
assert!(equal.declassify("example authentication decision is public"));
```

For deployments that want `sanitization`'s assembly-backed comparison checks,
enable the passthrough features:

```toml
base64-ng-sanitization = { version = "1.2.1", features = ["strict-ct"] }
```
