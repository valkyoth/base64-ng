<p align="center">
  <b>derive fixed-size redacted Base64 secret owners.</b><br>
  Explicit codec policy, staged decoding, wiping storage, and opt-in exposure.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-derive">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_DERIVE_HARDENING.md">Security contract</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/SECURITY.md">Security</a>
</div>

<br>

<p align="center">
  <a href="https://github.com/valkyoth/base64-ng">
    <img src="https://raw.githubusercontent.com/valkyoth/base64-ng/main/.github/images/base64-ng.webp" alt="base64-ng Rust crate overview">
  </a>
</p>

# base64-ng-derive

Dependency-free derive support for fixed-size `base64-ng` 2.0 secret owners.
The core crate remains free of proc-macro dependencies.

```rust
use base64_ng::secret::SecretInput;
use base64_ng_derive::Base64Secret;

#[derive(Base64Secret)]
#[base64_ng(
    alphabet = "url_safe",
    padding = "unpadded",
    exact_length = 32,
    exposure = "read"
)]
struct ApiKey(base64_ng::secret::SecretArray<32>);

# fn main() -> Result<(), base64_ng::secret::SecretDecodeError> {
let encoded = SecretInput::new(
    b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
);
let key = ApiKey::decode_base64(&encoded)?;
assert_eq!(key.expose_secret().len(), 32);
# Ok(())
# }
```

The four policy keys are mandatory:

| Key | Values | Meaning |
|---|---|---|
| `alphabet` | `"standard"`, `"url_safe"` | Selects an admitted strict alphabet. |
| `padding` | `"padded"`, `"unpadded"` | Selects exact padding acceptance and output. |
| `exact_length` | `1..=1024` | Must equal the private `SecretArray<N>` field capacity. |
| `exposure` | `"none"`, `"read"`, `"read_write"` | Controls which explicitly named exposure methods exist. |

Generated decoding uses `SecretArrayFrame`, so rejected input never commits
plaintext into the returned owner. Generated encoding returns another wiping
`SecretArray`; callers must explicitly expose or declassify it.

The macro generates no ordinary string parsing, slice conversion, equality,
cloning, or dereference traits. Use `base64-ng-subtle` for reviewed comparison
and choose an exposure policy only when interoperability requires it.
