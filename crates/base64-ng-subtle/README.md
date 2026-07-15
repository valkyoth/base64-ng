<p align="center">
  <b>subtle::ConstantTimeEq bridge for base64-ng buffers.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-subtle">Docs.rs</a>
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

# base64-ng-subtle

Optional `subtle::ConstantTimeEq` integration for `base64-ng`.

The core `base64-ng` crate stays zero-runtime-dependency. This companion crate
is for applications that already admit the `subtle` crate and want explicit
comparison helpers for decoded or encoded Base64 material.

```toml
[dependencies]
base64-ng = "1.3.8"
base64-ng-subtle = "1.3.8"
```

```rust
use base64_ng::ct;
use base64_ng_subtle::SubtleEqExt;

let decoded = ct::STANDARD.decode_secret(b"aGVsbG8=").unwrap();
assert!(decoded.subtle_verify(b"hello"));
```

Length is public: mismatched lengths return `Choice::from(0)` immediately.
Use fixed-size protocol tokens when length must not vary.
