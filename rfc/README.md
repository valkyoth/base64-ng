# RFC 4648, RFC 2045, RFC 3501, RFC 7468, And RFC 9580 Reference Material

This directory contains an immutable copy of the RFC Editor plain-text
publication used to trace `base64-ng` requirements. The RFC remains subject to
its original notices and copying conditions. It is reference material, not
project-licensed Rust source.

`rfc4648.txt`, `rfc2045.txt`, `rfc3501.txt`, `rfc7468.txt`, and `rfc9580.txt`
are byte-for-byte identical to the HTTPS sources in `SOURCES`. Verified errata
are not patched into those files. Project dispositions are recorded separately
in the matching errata TSV.

RFC 2045 evidence is limited to Section 6.8 Base64 content-transfer body
bytes. It does not claim complete MIME message, header, media-type, or
multipart parsing.

RFC 7468 evidence covers the textual encoding grammar, labels, boundaries,
body layout, adjacent text, and multiple instances. It does not claim to parse
or validate decoded ASN.1 structures.

RFC 9580 evidence covers complete ordinary OpenPGP ASCII armor. Its errata
snapshot includes every current RFC Editor record; none changes the Section 6
armor grammar. Packet semantics and cleartext signatures remain outside that
companion's scope.

RFC 3501 evidence is limited to the legacy modified Base64 payload transform
in Section 5.1.3. RFC 3501 is obsolete as an IMAP protocol specification; this
lock does not claim a complete modified UTF-7 mailbox codec or current IMAP4rev2
conformance.

Normal builds and checks are offline:

```sh
scripts/verify-rfcs.sh
```

Maintainers may explicitly compare the locked source with the RFC Editor:

```sh
scripts/fetch-rfcs.sh
scripts/lock-rfcs.sh
BASE64_NG_CHECK_LIVE_RFC_ERRATA=1 scripts/check-rfc-errata-live.py
```

The complete `rfc/` tree is excluded from crates.io and future npm packages.
