# Pinned Multibase Sources

This directory locks the multiformats multibase specification and registry at
commit `d7406cdea189b82a0b3937f5737b440f5fa92f92`.

Commit 45 implements only the four Base64-family registrations:

| Prefix | Registry name | Padding | Alphabet | Status |
|---|---|---|---|---|
| `m` | `base64` | forbidden | RFC 4648 Standard | final |
| `M` | `base64pad` | required when needed | RFC 4648 Standard | experimental |
| `u` | `base64url` | forbidden | RFC 4648 URL-safe | final |
| `U` | `base64urlpad` | required when needed | RFC 4648 URL-safe | final |

`upstream-README.md`, `multibase.csv`, and `tests/*.csv` are exact upstream
bytes. Upstream documentation is licensed CC-BY-SA 3.0; it is retained as
review evidence and excluded from every published Rust package.

Run `scripts/validate-multibase-spec.py` to verify hashes and the admitted
registry/vector semantics without network access.
