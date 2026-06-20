# base64-ng-serde

Optional `serde` integration for `base64-ng`.

The core `base64-ng` crate intentionally does not depend on `serde`. This
companion crate provides explicit wrappers for applications that already admit
`serde` in their dependency policy.

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
