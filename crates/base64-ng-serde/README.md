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

The compatibility wrapper types clear their initialized bytes on drop as a
retention-reduction measure, but they remain ordinary interoperability types.
Human-readable formats receive a string; binary formats receive a byte string
containing the same Base64 text. Deserialization prefers borrowed encoded
input and allocates only the exact decoded vector.

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
profiles from `base64-ng` and stream body bytes without a compacted encoded
copy.

Fixed-capacity ordinary fields use matching modules below `bounded`:

```rust
use base64_ng::DecodedArray;

#[derive(serde::Serialize, serde::Deserialize)]
struct BoundedMessage {
    #[serde(with = "base64_ng_serde::bounded::standard")]
    payload: DecodedArray<32>,
}
```

Enable `secrets` for fixed-work Standard and URL-safe deserialization into
wiping `base64_ng::secret::SecretArray<CAP>`. Serde format parsing before the
adapter remains timing-variable and may allocate. See
[`docs/2.0_SERDE_INTEGRATION.md`](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_SERDE_INTEGRATION.md)
for the exact boundary.
