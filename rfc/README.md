# RFC 4648 And RFC 2045 Reference Material

This directory contains an immutable copy of the RFC Editor plain-text
publication used to trace `base64-ng` requirements. The RFC remains subject to
its original notices and copying conditions. It is reference material, not
project-licensed Rust source.

`rfc4648.txt` and `rfc2045.txt` are byte-for-byte identical to the HTTPS
sources in `SOURCES`. Verified errata are not patched into those files.
Project dispositions are recorded separately in the matching errata TSV.

RFC 2045 evidence is limited to Section 6.8 Base64 content-transfer body
bytes. It does not claim complete MIME message, header, media-type, or
multipart parsing.

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
