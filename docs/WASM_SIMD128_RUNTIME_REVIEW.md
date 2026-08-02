# Wasm `simd128` Runtime Review

This file tracks the direct wasm `simd128` implementation and supported
JavaScript loader introduced by 2.0 Commit 30.

## Decision

The Rust crate admits direct fixed-block `simd128` encode and strict decode for
Standard and URL-safe alphabet families in artifacts compiled with
`target-feature=+simd128` and the `simd` feature. Encode processes exact
12-byte input blocks into 16-byte output blocks. Strict decode performs one
whole-input scalar validation, then directly classifies and decodes exact
16-byte blocks into 12-byte output blocks. Padded final quanta, canonical tail
bits, and short tails remain scalar.

The `base64-ng-wasm-loader` npm package ships two immutable artifacts:

- `base64-ng-scalar.wasm`
- `base64-ng-simd128.wasm`

An embedded `WebAssembly.validate` SIMD probe selects the artifact before
instantiation. A rejected probe never loads or instantiates the SIMD artifact.
Deployments may explicitly require scalar or SIMD posture. The loader reports
capability evidence, selected artifact, selection reason, compile-time artifact
posture, ABI version, and configured ceilings separately.
The loader verifies an embedded SHA-256 digest before instantiating either
shipped artifact. Custom artifact bytes or URLs require an explicit expected
digest rather than inheriting trust from the selected posture label.

## JavaScript Boundary

The supported API is byte-only and accepts genuine `Uint8Array` values. It
provides `encodedLength`, `decodedLength`, `encode`, `decode`, `encodeInto`,
`decodeInto`, and `decodeForgiving` for Standard, Standard-no-pad, URL-safe,
and URL-safe-no-pad codecs.

The loader:

- captures a closed intrinsic allowlist at module evaluation;
- uses the captured WebCrypto SHA-256 primitive to verify artifact bytes before
  `WebAssembly.instantiate`;
- rejects proxies, spoofed views, shared backing, detached backing, resizable
  backing, overlapping input/output views, and runtimes that cannot prove
  fixed `ArrayBuffer` storage;
- accepts genuine cross-realm arrays and subclasses without invoking caller
  constructors, iterators, species, or helper overrides;
- snapshots input before validation and stages output before changing an
  `*Into` destination;
- exposes owned output copies and never exposes wasm memory or borrowed views;
- checks exact safe-integer, WASM32, input, output, artifact, and memory limits;
- exposes redacted `Base64NgError` diagnostics without rejected input bytes;
- uses no `eval` or `new Function`; restrictive browser CSP deployments must
  allow WebAssembly compilation with `script-src 'wasm-unsafe-eval'`; and
- clears only the bounded scratch ranges each operation could have touched,
  performs a complete scratch clear on `dispose()`, and makes no engine, GC,
  JIT, register, or historical-memory cleanup claim; and
- converts a Rust panic into an immediate wasm trap instead of spinning on the
  host execution thread.

The package has no secret API. The Rust `secrets` capability remains scalar and
is governed by the separate wasm wipe policy.

## Evidence

`scripts/check-2.0-wasm-loader.sh` is the primary package gate. It:

- builds scalar and SIMD artifacts twice and compares exact checksums;
- proves embedded loader digests match rebuilt artifacts and rejects missing or
  mismatched digests for custom artifact sources;
- denies unreviewed Rust unsafe sites in the private artifact ABI;
- runs Node/V8 scalar/SIMD differential, malformed-input, transactionality,
  limit, proportional/full cleanup, disposal, cross-realm, hostile-object,
  shared-worker, and intrinsic instrumentation tests;
- executes each artifact self-test under Wasmtime when installed;
- records Node/V8 encode/decode benchmark evidence;
- packs the exact npm tarball, verifies its file allowlist, extracts it, and
  runs install-from-package smoke tests; and
- prepares that exact extracted package for browser tests.

`scripts/check_wasm_loader_browser_dispatch.sh` and
`scripts/check_wasm_loader_browser_firefox_dispatch.sh` serve the extracted npm
package over HTTP and run complete codec sweeps in Chromium/V8 and
Firefox/SpiderMonkey. `scripts/check_wasm_loader_browser_safari_dispatch.sh`
runs the same package and page through Safari/WebKit on an operator-provided
macOS host with remote automation enabled.

`scripts/generate_wasm_simd_evidence.sh` records release LLVM IR and requires
the `simd128` target feature, byte-vector shuffles, validity-mask reduction,
and wasm bit-select operations. The existing Rust runtime smoke remains an
independent check of core backend reporting and public Rust surfaces through
`scripts/check_wasm_runtime_dispatch.sh`.

## Limits

The evidence proves functional correctness, selection, packaging, and measured
benefit only for the named runtime executions. It does not prove universal JIT
timing, native register cleanup, every browser version, every edge runtime, or
constant-time behavior. Browser benchmark values are observations, not release
thresholds. Node/V8 performance admission requires the exact local benchmark
run to show both encode and decode benefit; other runtimes require their own
record before making numerical throughput claims.

Non-Standard-family custom alphabets, custom `Alphabet::decode` contracts that
diverge from their encode table, bcrypt/crypt alphabets, line-ending insertion,
and secret constant-time-oriented operations remain scalar. Unsupported
runtimes use the separate scalar artifact rather than internal runtime fallback
inside a SIMD artifact.
