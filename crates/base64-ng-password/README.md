<p align="center">
  <b>bounded Base64 fields for exact password-record formats.</b><br>
  Strict record grammar, canonical fields, finite limits, and no password hashing claims.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-password">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_PASSWORD_RECORDS.md">Password-record contract</a>
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

# base64-ng-password

Bounded Base64 field and record transforms for exact Passlib PBKDF2 and
SHA-crypt formats.

This crate implements:

- Passlib 1.7.4 `$pbkdf2$`, `$pbkdf2-sha256$`, and `$pbkdf2-sha512$`
  adapted-Base64 fields and record grammar;
- Drepper SHA-256-crypt `$5$` and SHA-512-crypt `$6$` record grammar and
  digest-byte permutations;
- canonical positive decimal rounds, exact checksum lengths, strict alphabets,
  caller-owned output, optional allocation, and finite resource limits.

It deliberately does **not** accept passwords, derive PBKDF2 or SHA digests,
hash passwords, compare records, verify passwords, or choose a password
storage policy. Callers provide already-derived checksum/digest bytes.

```rust
use base64_ng_password::{
    PasslibPbkdf2Algorithm, PasswordRecordLimits, generate_pbkdf2_record,
    parse_pbkdf2_record,
};

let checksum = [0x42_u8; 32];
let text = generate_pbkdf2_record(
    PasslibPbkdf2Algorithm::Sha256,
    29_000,
    b"public-salt",
    &checksum,
    PasswordRecordLimits::default(),
)?;
let parsed = parse_pbkdf2_record(text.as_bytes(), PasswordRecordLimits::default())?;
assert_eq!(parsed.rounds(), 29_000);
# Ok::<(), base64_ng_password::PasswordRecordError>(())
```

Parsed records use field-selective redacted `Debug`; salts and checksums are
available only through explicitly named exposure/decode methods. Errors never
format malformed raw record fragments. This is an ordinary interoperability
API, not a constant-time secret processing or password verification API.

`PasswordRecordLimits::max_work_before_output` is a cumulative per-operation
budget. Every record scan, field validation, and encode/decode transform
charges the same budget before that pass begins; a validation pass followed by
a decode pass therefore charges the encoded field length twice. The
no-default-features test matrix executes the caller-owned PBKDF2 and SHA-crypt
APIs without enabling `alloc`. Allocating generators admit every validation,
size, and cumulative-work limit before reserving or initializing their output
allocation.
