<p align="center">
  <b>locked and clear-on-drop secret decode bridge for base64-ng.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-sanitization">Docs.rs</a>
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

# base64-ng-sanitization

Optional `sanitization` integration helpers for `base64-ng`.

This companion crate keeps `base64-ng` itself dependency-free while giving
applications that already use `sanitization` a direct path from
constant-time-oriented Base64 decode into clear-on-drop secret containers.

```toml
[dependencies]
base64-ng = { version = "1.3.9", default-features = false }
base64-ng-sanitization = { version = "1.3.9", default-features = false }
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
base64-ng-sanitization = { version = "1.3.9", features = ["alloc"] }
```

For high-assurance x86_64 or AArch64 native deployments, enable locked storage
helpers. This
uses `sanitization` 2.0.3's hardened native controls, including memory locking,
strict random canaries, and strict assembly comparison, and decodes directly
into locked memory:

```toml
base64-ng-sanitization = { version = "1.3.9", features = ["high-assurance"] }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, LockedSanitizationCtEqExt};

let key = ct::STANDARD
    .decode_locked_secret_bytes_checked::<5>(b"aGVsbG8=")
    .unwrap();

key.try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))?;
assert!(key.try_sanitization_verify(
    b"hello",
    "example authentication decision is public"
)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

For dynamic output on supported native targets:

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationExt;

let key = ct::STANDARD
    .decode_locked_secret_vec_checked(b"aGVsbG8=")
    .unwrap();

key.try_with_secret(|bytes| assert_eq!(bytes, b"hello"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The built-in fixed-size and dynamic `_checked` methods establish required
memory-lock, dump, and fork controls before decoding plaintext into the
mapping. Dynamic decode uses sanitization 2.0.3's protected-capacity fill
constructor, whose closure is not invoked when a required control fails.
External implementations of `CtDecodeSanitizationExt` must override the
compatibility default to obtain that same pre-decode guarantee. Non-checked
methods remain available when callers apply a deployment-specific policy to
the complete report. The additive
`decode_locked_secret_bytes_fill` method exposes sanitization 2.0's integrity
aware fill error while the original method retains its generation-error return
type for source compatibility.

The integration intentionally targets `base64_ng::ct::CtEngine`. Strict
non-CT decoders remain available in `base64-ng`, but this crate keeps the
secret-container API pointed at the constant-time-oriented decode path.

`base64-ng-sanitization` also re-exports `sanitization::ct` and adds
`SanitizationCtEqExt` for comparing decoded `SecretBytes` and `SecretVec`
values through `sanitization` 2.0.3's native `Choice` API. This gives projects
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
base64-ng-sanitization = { version = "1.3.9", features = ["strict-compare"] }
```

The previous companion feature name `strict-ct` remains as an alias for
`strict-compare` during migration.
