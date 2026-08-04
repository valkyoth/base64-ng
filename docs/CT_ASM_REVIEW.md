# Constant-Time Assembly Review

The evidence generator recognizes Rust function-definition labels in both
GNU/ELF assembly and Apple/Mach-O assembly. Its regression fixture rejects
call and global-reference lines so a referenced but inlined-away cleanup or
constant-time helper cannot satisfy the non-inlining gate.

This document records the manual generated-code review expected for
constant-time-oriented scalar decode work. It does not create a formal
cryptographic constant-time guarantee.

## Scope

Review these generated artifacts after running:

```sh
scripts/generate_ct_asm_evidence.sh
```

An installed target can be reviewed explicitly without changing the active
host toolchain:

```sh
BASE64_NG_CT_ASM_TARGET=aarch64-unknown-linux-gnu \
  scripts/generate_ct_asm_evidence.sh
```

The generator compiles each configuration in a fresh temporary Cargo target
directory with incremental compilation disabled and `--locked`, and accepts
exactly one newly produced crate assembly artifact. Its manifest records the
source commit, a clean worktree state, and the `Cargo.lock` checksum. The
default path rejects dirty or unavailable Git state and verifies that the
captured commit, worktree status, and lockfile remain unchanged after every
build. `BASE64_NG_ALLOW_DIRTY_EVIDENCE=1` exists only for development checks;
its manifest is marked `dirty-development-only` and is not release evidence.

Expected artifacts:

- `target/release-evidence/asm/base64_ng-no-default-features.s`
- `target/release-evidence/asm/base64_ng-all-features.s`
- `target/release-evidence/asm/base64_ng-all-features-lto.s`
- `target/release-evidence/asm/MANIFEST.txt`

Explicit-target artifacts use
`target/release-evidence/asm/<target>/`. Cross-compiled output is code-generation
evidence, not native timing or hardware evidence.

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
- `ct_accumulate_u8`
- `secret_encode_ascii`
- `secret_encode_scan`
- bounded 2.0 secret decoder `decode_symbol`
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
- Do the automated symbol checks find concrete text-section definitions for
  the required boundaries under both legacy Rust mangling and Rust `1.97.1`'s
  default v0 mangling?
- Does `constant_time_eq_public_len` remain a separate non-inlined symbol in
  release and LTO artifacts, and does the equal-length loop scan all bytes
  rather than lowering into an early-exit compare?
- Does `secret_encode_ascii` map all five RFC 4648 ranges without branching on
  the secret six-bit value or loading from a secret-indexed table?
- Does `secret_encode_scan` retain exactly one fixed 64-iteration loop whose
  alphabet load is indexed only by the public candidate counter?
- Do `wipe_bytes` and `wipe_barrier` remain non-inlined cleanup call
  boundaries in release and LTO artifacts?
- Does the manifest bind the overwrite boundary to its source commit,
  lockfile, compiler, target, flags, target barrier, and
  `WIPE_PRIMITIVE_REVISION`?
- Is the operation-specific runtime wipe generation taken from the assurance
  report rather than inferred from static assembly?
- Are padding length, decoded length, and final success/failure still treated
  as public by documentation and API shape?

## Current Release Position

For the `2.0.0` release candidate:

- Assembly evidence generation is release-gated.
- Manual review is required before any future release that strengthens ct
  wording beyond "constant-time-oriented".
- The public documentation continues to use "constant-time-oriented" wording.
- No formally verified cryptographic constant-time guarantee is claimed.
- The exact pre-gate, post-gate, cleanup, target, and residual-risk wording is
  defined in [`2.0_TIMING_AND_CODEGEN.md`](2.0_TIMING_AND_CODEGEN.md).

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
