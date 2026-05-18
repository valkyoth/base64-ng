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
| CWE-208 Observable Timing Discrepancy | Sensitive comparison exits early on the first different byte | `SecretBuffer`, `EncodedBuffer`, `DecodedBuffer`, and their direct byte/text comparison impls use dependency-free constant-time-oriented comparison for equal-length inputs. Length mismatch returns immediately and is treated as public protocol information. |
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
- understanding that stack-backed buffers can clear their own backing arrays
  but cannot clear historical stack-frame copies made by compiler spills,
  caller code, panic machinery, crash handlers, or operating system capture
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
without runtime dependencies using audited volatile wipe helpers, an
architecture-gated inline assembly barrier where stable Rust supports it, and
compiler fences as inventoried in [UNSAFE.md](UNSAFE.md). They do not claim
formal zeroization against compiler behavior, historical stack-frame copies,
allocator internals, copies made outside the wrapper, core dumps, swap, CPU
registers, cache lines, or arbitrary process memory disclosure vulnerabilities.
On `wasm32`, the wipe barrier uses only a compiler fence; downstream wasm
runtime JIT behavior is outside this crate's control. High-assurance wasm
deployments should apply their own memory strategy around `EncodedBuffer`,
`DecodedBuffer`, `SecretBuffer`, and caller-owned output buffers.
For high-assurance secret handling, use the clear-tail APIs promptly and pair
them with operating system and deployment controls that reduce crash dumps,
swap, and broad memory disclosure exposure.
For constant-time-oriented decode, prefer
`ct::CtEngine::decode_slice_clear_tail` when a caller-owned output buffer may be
reused after a rejected input: `ct::CtEngine::decode_slice` reports malformed
input after its fixed-shape pass and can leave partially decoded bytes in the
buffer on error.
If a platform requires a formal zeroization policy, apply that policy to
caller-owned buffers in addition to the crate's dependency-free cleanup APIs.
For applications that already admit `zeroize`, decode into caller-owned buffers
and apply `Zeroize::zeroize()` after the Base64 step; `base64-ng` intentionally
does not add that dependency for every user.

### Side-Channel Posture

The default scalar encoder avoids input-derived alphabet table indexes for
built-in alphabets, and custom alphabet helper decoding now scans the full
64-byte alphabet before returning. The default strict decoder still preserves
localized errors and may return early for malformed input, padding, length, or
output-size failures, so it is not a constant-time decoder. Named engines and
profiles such as `STANDARD`, `URL_SAFE_NO_PAD`, `MIME`, `PEM`, `BCRYPT`, and
`CRYPT` are strict interoperability APIs, not token-comparison or key-material
decode APIs. The `ct` module narrows the timing target further for scalar
decode and uses a fixed scan over the selected alphabet for generic symbol
mapping. It still does not carry a formal cryptographic constant-time
guarantee.
Input length, padding length, decoded length, and final success/failure remain
public protocol facts; callers that must hide those facts need fixed-shape
protocol-level processing after decode failure.
Deployments that require the most conservative side-channel posture should
combine `base64_ng::ct` with
`runtime::BackendPolicy::HighAssuranceScalarOnly` so sensitive decode paths
remain on the scalar backend and are not affected by future SIMD admission.

### Supply Chain

The published crate is dependency-free by default. Fuzzing dependencies are
isolated under `fuzz/` and reviewed separately. The release gate requires
formatting, clippy, tests, documentation, cargo-deny, RustSec audit, license
inventory, SBOM generation, and reproducibility checks.
