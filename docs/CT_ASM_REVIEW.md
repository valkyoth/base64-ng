# Constant-Time Assembly Review

This document records the manual generated-code review expected for
constant-time-oriented scalar decode work. It does not create a formal
cryptographic constant-time guarantee.

## Scope

Review these generated artifacts after running:

```sh
scripts/generate_ct_asm_evidence.sh
```

Expected artifacts:

- `target/release-evidence/asm/base64_ng-no-default-features.s`
- `target/release-evidence/asm/base64_ng-all-features.s`
- `target/release-evidence/asm/base64_ng-all-features-lto.s`
- `target/release-evidence/asm/MANIFEST.txt`

Review focus:

- `ct::CtEngine` decode entry points
- `ct_decode_padded`
- `ct_decode_unpadded`
- `ct_decode_padded_in_place`
- `ct_decode_unpadded_in_place`
- `ct_decode_alphabet_byte`
- `ct_mask_*` helpers
- `ct_error_gate_barrier`
- `constant_time_eq_public_len`
- `wipe_bytes` and `wipe_barrier` cleanup call boundaries

## Review Questions

- Are selected alphabet bytes scanned with fixed 64-entry symbol mapping rather
  than secret-indexed decode tables?
- Are malformed-content flags accumulated through masks instead of early
  returns inside fixed-length decode loops?
- Are branches in the reviewed ct path based only on public facts such as
  input length, padding mode, selected alphabet, and caller output capacity?
- Does generated code keep the scalar ct path independent from SIMD dispatch?
- Does `ct_error_gate_barrier` remain a separate non-inlined symbol in release
  and LTO artifacts before opaque malformed-input reporting?
- Does `constant_time_eq_public_len` remain a separate non-inlined symbol in
  release and LTO artifacts, and does the equal-length loop scan all bytes
  rather than lowering into an early-exit compare?
- Do `wipe_bytes` and `wipe_barrier` remain non-inlined cleanup call
  boundaries in release and LTO artifacts?
- Are padding length, decoded length, and final success/failure still treated
  as public by documentation and API shape?

## Current Release Position

For the current `1.0.0-alpha.0` development branch:

- Assembly evidence generation is release-gated.
- Manual review is required before a stable release that strengthens ct
  wording.
- The public documentation continues to use "constant-time-oriented" wording.
- No formally verified cryptographic constant-time guarantee is claimed.

## Reviewer Notes

Record release-candidate notes here before tagging a stable release:

```text
version:
rustc -Vv:
targets reviewed:
features reviewed:
assembly manifest checksum:
reviewer:
date:
notes:
```
