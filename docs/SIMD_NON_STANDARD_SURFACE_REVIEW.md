# SIMD Non-Standard Surface Review

This file tracks the `1.3.2` review scope. It is an admission ledger, not an
acceleration announcement. A surface listed here remains scalar unless this
file, [SIMD_ADMISSION.md](SIMD_ADMISSION.md), generated evidence, benchmarks,
unsafe inventory, and release notes all move together.

## Current Decision

No new non-standard SIMD acceleration is admitted yet.

The current `1.3.2` checkpoint adds regression evidence that these surfaces
preserve scalar-visible behavior while staying outside the active accelerated
scope:

- custom alphabet encode and decode
- bcrypt-style and `crypt(3)`-style alphabet encode and decode
- strict in-place decode
- legacy-whitespace decode
- strict wrapped decode
- wrapped encode staging

The checked test evidence currently includes:

- `non_standard_simd_candidate_surfaces_preserve_scalar_behavior`, covering
  successful custom, bcrypt-style, `crypt(3)`, in-place, legacy-whitespace,
  wrapped decode, and wrapped encode behavior against scalar-visible output.
- `non_standard_simd_candidate_error_surfaces_preserve_scalar_behavior`,
  covering malformed custom, bcrypt-style, `crypt(3)`, in-place,
  legacy-whitespace, wrapped decode, and wrapped encode error behavior.
- A naive wrapped-output oracle that inserts line endings by line length rather
  than calling the production `write_wrapped_byte` helper. This keeps the
  wrapped encode regression test from depending only on the same primitive used
  by the implementation.

## Surface Ledger

| Surface | Current status | Required before admission |
| --- | --- | --- |
| Custom alphabet encode | scalar fallback | fixed-block scalar equivalence for arbitrary alphabets, malformed alphabet rejection, output-size parity, fuzz evidence, benchmark evidence, and proof that any table lookup or SIMD shuffle does not introduce unsupported timing claims |
| Custom alphabet decode | scalar fallback | full error-shape parity, canonicality parity, invalid-byte offset parity, fuzz evidence, benchmark evidence, and a decision on whether a vector lookup can support arbitrary alphabets without secret-indexed timing claims |
| Bcrypt and `crypt(3)` profiles | scalar fallback | separate profile evidence for alphabet order, no-padding behavior, malformed input, canonicality, and benchmark value |
| MIME/PEM wrapped encode | partially staged through admitted unwrapped encode | line-ending insertion remains scalar; admission requires wrapped output parity, staging-retention review, clear-tail parity, and benchmark evidence showing wrapping overhead does not hide the SIMD benefit |
| MIME/PEM wrapped decode | scalar fallback | line-profile validation parity, compacted-input parity, absolute error-index parity, clear-tail behavior, fuzz evidence, and benchmark evidence |
| Legacy-whitespace decode | scalar fallback | whitespace compaction parity, original-index error reporting, post-padding rejection, fuzz evidence, and benchmark evidence |
| In-place encode | scalar fallback | overlap proof, backwards-write proof, clear-tail parity, malformed length parity, Miri/Kani evidence where applicable, and benchmark evidence |
| In-place decode | scalar fallback | prevalidation proof, overlap proof, failed-buffer-state policy, clear-tail parity, fuzz evidence, and benchmark evidence |
| Constant-time-oriented secret decode | scalar only | separate high-assurance side-channel project; do not admit through ordinary performance SIMD review |

## Source Routing Invariants

- `Engine::decode_slice_legacy` validates with `validate_legacy_decode` and
  decodes through `decode_legacy_to_slice`.
- `Engine::decode_slice_wrapped` validates with `validate_wrapped_decode` and
  decodes through `decode_wrapped_to_slice`.
- `Engine::encode_slice_wrapped` may use admitted unwrapped encode for its
  temporary Base64 staging step, but line-ending insertion remains scalar via
  `write_wrapped_byte` or `write_wrapped_bytes`.
- `Engine::encode_in_place` and `Engine::decode_in_place` remain scalar unless
  this ledger and the main admission manifest are updated in the same release.

## Review Rule

Do not broaden public SIMD claims because a lower-level fixed-block prototype
exists. Public claims are allowed only for surfaces whose row says admitted in
[SIMD_ADMISSION.md](SIMD_ADMISSION.md) and whose ledger entry here names the
same admitted scope.
