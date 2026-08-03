<p align="center">
  <b>bounded multibase support for the Base64 encoding family.</b><br>
  Strict prefixes, canonical payloads, finite limits, and explicit registry scope.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-multibase">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_MULTIBASE.md">Multibase contract</a>
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

# base64-ng-multibase

Bounded, strict support for the four Base64-family entries in the pinned
multiformats multibase registry:

| Prefix | Encoding | Padding | Registry status |
| --- | --- | --- | --- |
| `m` | `base64` | forbidden | final |
| `M` | `base64pad` | canonical | experimental |
| `u` | `base64url` | forbidden | final |
| `U` | `base64urlpad` | canonical | final |

This crate is not a complete multibase implementation. Every other prefix is
rejected explicitly. Prefixes are case-sensitive, and payloads use the strict
canonical RFC 4648 codec selected by the prefix.

```rust
use base64_ng_multibase::{
    Base64MultibaseEncoding, Base64MultibaseLimits,
    decode_base64_multibase_to_vec, encode_base64_multibase_to_string,
};

let limits = Base64MultibaseLimits::new(1024, 2048, 1024);
let encoded = encode_base64_multibase_to_string(
    Base64MultibaseEncoding::Base64Url,
    b"hello",
    limits,
)?;
assert_eq!(encoded, "uaGVsbG8");

let decoded = decode_base64_multibase_to_vec(encoded.as_bytes(), limits)?;
assert_eq!(decoded.encoding(), Base64MultibaseEncoding::Base64Url);
assert_eq!(decoded.as_bytes(), b"hello");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Resource and state policy

`Base64MultibaseLimits` bounds source bytes, destination bytes, and work before
complete output. Caller-owned one-shot output is transactional on every error.
`Base64MultibaseEncoder` and `Base64MultibaseDecoder` are heapless incremental
states with exact progress, absorbing errors, and explicit reset/clear calls.

The APIs are ordinary public-data transforms. Their errors may expose a
rejected prefix or payload position, and their state does not claim secret
zeroization or constant-time behavior. Use the core secret capability for
secret-bearing Base64 rather than this protocol companion.

The exact upstream commit, registry, and official vectors used for release
evidence are documented in the repository under `spec/multibase/`.
