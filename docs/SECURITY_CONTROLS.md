# Security Controls And CWE Mapping

This document maps `base64-ng` security controls to common weakness classes.
The mapping is practical adoption guidance, not a certification claim.

## Mitigated Or Reduced By The Crate

| Weakness Class | Relevant Risk | `base64-ng` Control |
| --- | --- | --- |
| CWE-20 Improper Input Validation | Accepting malformed Base64, mixed alphabets, invalid padding, or non-canonical trailing bits | Strict decoding is default; validation-only APIs use the same checks as decode APIs. |
| CWE-125 Out-of-bounds Read | Decoder reads past malformed or short input | Scalar decode validates lengths and chunk shapes before reading chunk contents. |
| CWE-787 Out-of-bounds Write | Encoder/decoder writes beyond caller output | Slice APIs check required output length before writing; in-place APIs validate buffer capacity and decode-to-front invariants. |
| CWE-190 Integer Overflow | Encoded or wrapped length calculation wraps `usize` | Public length helpers return `Result` or `Option`; wrapped lengths use checked arithmetic. |
| CWE-400 Uncontrolled Resource Consumption | Allocating based on attacker-controlled encoded input | `decode_vec` validates input before allocating; caller-owned APIs and checked length helpers are available for hard limits. |
| CWE-209 Information Exposure Through Error Messages | Constant-time-oriented malformed decode reveals exact failure location or category | `ct` malformed-content errors are intentionally opaque and non-localized. |
| CWE-226 Sensitive Information in Resource Not Removed Before Reuse | Caller-owned output buffers or crate-owned staging buffers retain partial sensitive data | Clear-tail APIs, stream cleanup, `EncodedBuffer`, `DecodedBuffer`, and `SecretBuffer` provide best-effort initialized-byte and spare-capacity cleanup. |
| CWE-327 Use of Broken Or Risky Cryptographic Algorithm | Treating Base64 as encryption | Documentation describes Base64 as encoding, not encryption; secret wrappers are retention/logging helpers only. |
| CWE-532 Insertion of Sensitive Information Into Log File | Accidentally logging sensitive encoded or decoded material | `SecretBuffer` redacts `Debug` and `Display` output and requires explicit reveal calls. |
| CWE-208 Observable Timing Discrepancy | Sensitive comparison exits early on the first different byte | `SecretBuffer`, `EncodedBuffer`, `DecodedBuffer`, and their direct byte/text comparison impls use dependency-free constant-time-oriented comparison for equal-length inputs. |
| CWE-829 Inclusion of Functionality From Untrusted Control Sphere | Runtime dependency compromise | Published crate has zero external runtime and default dev dependencies; dependency admission is documented and checked. |

## Caller Responsibilities

The caller still owns:

- authentication, encryption, signature verification, and key management
- protocol-level framing and maximum accepted payload size
- choosing strict, legacy, wrapped, MIME, PEM, bcrypt-style, or custom profiles
  intentionally
- avoiding accidental copies of secrets before wrapping them in `SecretBuffer`
- treating stack-buffer `into_exposed_array` calls as boundaries where redacted
  formatting and drop-time cleanup intentionally stop for the returned arrays
- treating `SecretBuffer::into_exposed_vec` as a boundary where redacted
  formatting and drop-time cleanup intentionally stop
- process-wide memory hygiene such as core-dump policy, swap policy, crash
  handling, allocator behavior, and log retention
- deciding whether the constant-time-oriented API is sufficient for the local
  threat model
- running the release gate and reviewing release evidence for the exact
  version being adopted

## Control Boundaries

### Input Validation

Strict APIs reject:

- non-alphabet bytes
- mixed standard and URL-safe alphabets
- invalid lengths
- malformed padding
- trailing data after padding
- non-canonical trailing bits

Legacy whitespace handling is opt-in through explicitly named APIs. Wrapped
profiles are strict about the configured line ending and non-final line width.

### Memory Retention Reduction

Cleanup APIs are best-effort initialized-byte cleanup. They are implemented
without runtime dependencies and without unsafe code in scalar paths. They do
not claim formal zeroization against compiler behavior, allocator spare
capacity, copies made outside the wrapper, core dumps, swap, or arbitrary
process memory disclosure vulnerabilities.

### Side-Channel Posture

The default scalar encoder avoids input-derived alphabet table indexes for
built-in alphabets, and the decoder uses branch-minimized ASCII arithmetic.
The `ct` module narrows the timing target further for scalar decode and uses a
fixed scan over the selected alphabet for generic symbol mapping. It still does
not carry a formal cryptographic constant-time guarantee.

### Supply Chain

The published crate is dependency-free by default. Fuzzing dependencies are
isolated under `fuzz/` and reviewed separately. The release gate requires
formatting, clippy, tests, documentation, cargo-deny, RustSec audit, license
inventory, SBOM generation, and reproducibility checks.
