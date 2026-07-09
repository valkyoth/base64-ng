<p align="center">
  <b>explicit serde wrappers for visible Base64 fields.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-serde">Docs.rs</a>
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

# base64-ng-serde

Optional `serde` integration for `base64-ng`.

The core `base64-ng` crate intentionally does not depend on `serde`. This
companion crate provides explicit wrappers for applications that already admit
`serde` in their dependency policy.

The wrapper types clear their initialized bytes on drop as a best-effort
retention-reduction measure, but they are still interoperability types. They
serialize bytes into visible Base64 text, clones are independent copies, and
deserialization uses the normal strict decoder rather than the `ct` module. Do
not use this crate as the primary path for private keys, bearer tokens, or
other fields whose malformed-input timing matters.

```rust
use base64_ng_serde::Base64Standard;

let wrapped = Base64Standard::new(b"hello".to_vec());
let json = serde_json::to_string(&wrapped).unwrap();
assert_eq!(json, "\"aGVsbG8=\"");
```

For field-level use, prefer the explicit modules:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct Message {
    #[serde(with = "base64_ng_serde::standard")]
    payload: Vec<u8>,
}
```

Available field modules are `standard`, `standard_no_pad`, `url_safe`,
`url_safe_no_pad`, `mime`, and `pem`. MIME and PEM use the strict wrapping
profiles from `base64-ng`; they are interoperability helpers, not
constant-time-oriented secret decoders.
