<p align="center">
  <b>bounded RFC 2045 Base64 content-transfer body support.</b><br>
  Canonical generation, explicit interoperable decoding, finite limits, and chunk-resumable state.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-mime">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_MIME_BODY.md">MIME body contract</a>
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

# base64-ng-mime

This companion implements only the Base64 content-transfer body rules from
RFC 2045 Section 6.8. It does **not** parse MIME headers, complete messages,
multipart boundaries, media types, or body-part containers.

Canonical encoding uses Standard Base64, required padding, 76-column lines,
and `CRLF`. The terminal `CRLF` choice is explicit:

```rust
use base64_ng_mime::{
    MimeBodyLimits, MimeBodyTerminalLineEnding,
    encode_mime_content_transfer_body_to_string,
};

let body = encode_mime_content_transfer_body_to_string(
    b"hello",
    MimeBodyLimits::DEFAULT,
    MimeBodyTerminalLineEnding::IncludeCrLf,
)?;
assert_eq!(body, "aGVsbG8=\r\n");
# Ok::<(), base64_ng_mime::MimeBodyError>(())
```

Decoding makes strict canonical layout and RFC 2045-compatible transport
handling distinct choices. Compatible decoding ignores bytes outside RFC 2045
Table 1 only under finite input, output, physical-line, skip, and work limits.
The returned report lets applications warn about suspicious non-whitespace or
bare line endings.

```rust
use base64_ng_mime::{
    MimeBodyDecodePolicy, MimeBodyLimits,
    decode_mime_content_transfer_body_to_vec,
};

let (plain, report) = decode_mime_content_transfer_body_to_vec(
    b"aG\r\nVsbG8=",
    MimeBodyDecodePolicy::Rfc2045Compatible,
    MimeBodyLimits::DEFAULT,
)?;
assert_eq!(plain, b"hello");
assert!(!report.has_transport_warning());
# Ok::<(), base64_ng_mime::MimeBodyError>(())
```

`MimeBodyEncoder` and `MimeBodyDecoder` preserve state across arbitrary
transport chunk boundaries and short output buffers. Their output is
prefix-committing. Malformed input, limit failures, arithmetic overflow, and
internal core failures permanently close the affected state. The one-shot
decode helper performs a complete validation pass before writing and is
transactional for caller-owned output.

These are ordinary, non-secret APIs. They do not provide constant-time
processing, protected memory, or plaintext cleanup guarantees.
