<p align="center">
  <b>bounded OpenPGP ASCII armor for the base64-ng crate family.</b><br>
  RFC 9580 framing, explicit checksum policy, finite limits, and strict boundaries.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-openpgp">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_OPENPGP.md">OpenPGP contract</a>
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

# base64-ng-openpgp

Bounded RFC 9580 OpenPGP ASCII armor parsing and generation for the
`base64-ng` crate family.

The crate validates complete armor blocks: one of the four ordinary armor
types, headers, the required blank separator, Base64 body, optional CRC-24,
and matching closing boundary. It does not parse OpenPGP packets and does not
implement the separate cleartext signature framework.

RFC 9580 requires readers not to reject an object solely because its legacy
CRC-24 is absent, malformed, or mismatched. `ChecksumPolicy::Rfc9580` follows
that rule and reports `ChecksumStatus`; `ChecksumPolicy::RequireValidCrc24`
is a separately named stricter application policy. Generation omits CRC-24
by default and emits it only when `ChecksumGeneration::LegacyCrc24` is chosen.

```rust
use base64_ng_openpgp::{
    ArmorType, ChecksumGeneration, ChecksumPolicy, GenerationOptions,
    OpenPgpLimits, encode_armor_to_string, parse_armor_document,
};

let limits = OpenPgpLimits::default();
let text = encode_armor_to_string(
    ArmorType::Message,
    &[],
    b"packet bytes",
    limits,
    GenerationOptions::new(ChecksumGeneration::Omit),
)?;
let document = parse_armor_document(text.as_bytes(), limits, ChecksumPolicy::Rfc9580)?;
assert_eq!(document.blocks()[0].contents(), b"packet bytes");
# Ok::<(), base64_ng_openpgp::OpenPgpError>(())
```

All ordinary payload-returning APIs use normal memory. Enable `secrets` and
use `parse_secret_armor_block` for bounded, clear-on-drop secret payload
release.
