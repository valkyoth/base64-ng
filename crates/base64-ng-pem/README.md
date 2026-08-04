<p align="center">
  <b>bounded RFC 7468 textual encoding for the base64-ng crate family.</b><br>
  Strict labels, matching boundaries, canonical bodies, and finite document limits.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-pem">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_PEM.md">PEM contract</a>
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

# base64-ng-pem

Bounded parsing and generation of the complete textual encoding grammar in
RFC 7468, including labels, matching BEGIN/END boundaries, 64-column Base64
bodies, multiple blocks, adjacent text, and newline interoperability.

This crate does not parse ASN.1 payloads. Legacy OpenSSL encapsulated headers
such as `Proc-Type:` and `DEK-Info:` are deliberately outside its API.

```rust
use base64_ng_pem::{
    PemGenerationOptions, PemLabel, PemLimits, PemParsePolicy,
    encode_pem_block_to_string, parse_pem_document,
};

let label = PemLabel::new("PUBLIC KEY")?;
let encoded = encode_pem_block_to_string(
    &label,
    b"public bytes",
    PemLimits::default(),
    PemGenerationOptions::default(),
)?;
let parsed = parse_pem_document(
    encoded.as_bytes(),
    PemLimits::default(),
    PemParsePolicy::Strict,
)?;
assert_eq!(parsed.blocks()[0].contents(), b"public bytes");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Parsing policies

- `Strict` requires uppercase labels, matching boundaries, CRLF, exact
  64-character non-final lines, canonical padding, and no ignored body bytes.
- `Rfc7468Compatible` accepts RFC parser latitude for line endings, boundary
  blanks, noncanonical body wrapping, ignored body bytes, and mismatched END
  labels. Every deviation is counted in `PemParseReport` and every permissive
  path remains finitely bounded.

Both policies accept bounded text before, between, and after blocks, as RFC
7468 requires parsers not to malfunction on explanatory text.

`PemDocumentParser` and `PemBlockEncoder` accept arbitrarily partitioned input
chunks while retaining only their caller-bounded document or payload. Their
results match the transactional one-shot APIs.

## Secret payloads

Enable `secrets` for `parse_pem_secret_block`. It requires strict syntax,
exactly one caller-expected label, and bounded output. The compacted encoded
body is clear-on-drop and the core fixed-work secret decoder stages plaintext
until complete validation succeeds. This is best-effort software cleanup, not
memory locking or a formal constant-time claim.

## Resource policy

`PemLimits` always bounds input, generated output, decoded output, physical
line length, label length, block count, adjacent text, and work before output.
The work limit is a cumulative, conservative byte-pass budget across source
scanning, boundary and label classification, body compaction, Base64
validation, exact sizing, and decoding. It is therefore intentionally larger
than the default input limit and is not merely a second input-length ceiling.
Physical lines are consumed through a cursor without a document-wide line
index, and body-layout validation retains only constant-size metadata.
The parser never interprets payload bytes as certificates, keys, CMS, or any
other ASN.1 type.
