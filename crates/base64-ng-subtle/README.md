<p align="center">
  <b>Reviewed subtle::ConstantTimeEq boundary for base64-ng 2.0 secrets.</b><br>
  Sealed secret integrations, explicit public-length behavior, and release-gated evidence.
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

Reviewed `subtle::ConstantTimeEq` integration for `base64-ng` 2.0 secret
storage.

The core `base64-ng` crate stays zero-runtime-dependency. This companion crate
is for applications that admit `subtle` and want one explicit comparison
boundary for decoded tokens, MACs, and fixed-width keys. The sealed
`SubtleSecretEq` trait is implemented only for the final 2.0 secret owners and
views. This package requires exactly `subtle` 2.6.1 so downstream resolution
cannot silently move beyond the implementation used for the reviewed assembly
and timing evidence.

```toml
[dependencies]
base64-ng = { version = "2.0.1", features = ["secrets"] }
base64-ng-subtle = "2.0.1"
```

```rust
use base64_ng::{
    STRICT_STANDARD_PADDED,
    secret::{SecretArrayFrame, SecretInput},
};
use base64_ng_subtle::SubtleSecretEq;

let mut frame = SecretArrayFrame::<5>::new(&STRICT_STANDARD_PADDED).unwrap();
frame.update(&SecretInput::new(b"aGVsbG8=")).unwrap();
let decoded = frame.finish().unwrap();
let choice = decoded.subtle_ct_eq_public_len(b"hello");
assert!(bool::from(choice));
```

Length is public: mismatched lengths return `Choice::from(0)` immediately. The
crate deliberately provides no boolean convenience method; convert or compose
the returned `Choice` at the protocol decision point.

For fixed-width keys, enforce the expected initialized width during decode and
compare against an array-backed expected value:

```rust
use base64_ng::secret::SecretArray;
use base64_ng_subtle::SubtleSecretEq;

let key = SecretArray::<32>::from_array([0x42; 32], 32).unwrap();
let expected = [0x42; 32];
assert!(bool::from(key.subtle_ct_eq_public_len(&expected)));
```

This integration does not make compiler-level constant-time behavior a formal
guarantee. Use the exact release toolchain/target evidence required by your
deployment and keep length, comparison result, and any following branch within
the documented public protocol contract.
