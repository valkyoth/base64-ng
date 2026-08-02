# base64-ng-wasm-loader

`base64-ng-wasm-loader` is the supported JavaScript companion for
[`base64-ng`](https://github.com/valkyoth/base64-ng). It selects a scalar or
`simd128` WebAssembly artifact before instantiation and exposes byte-only
Base64 APIs.

```js
import { Codecs, createBase64Ng } from "base64-ng-wasm-loader";

const base64 = await createBase64Ng();
const input = new TextEncoder().encode("hello");
const encoded = base64.encode(input, Codecs.URL_SAFE_NO_PAD);
const decoded = base64.decode(encoded, Codecs.URL_SAFE_NO_PAD);
base64.dispose();
```

The API accepts genuine, fixed, attached, non-shared `Uint8Array` values.
Cross-realm arrays and subclasses are supported without invoking caller
constructors, iterators, species, or helper overrides. Proxies, detached or
resizable buffers, `SharedArrayBuffer`, and overlapping `*Into` views are
rejected.

## Artifact policy

`createBase64Ng()` runs an embedded `WebAssembly.validate` SIMD probe and then
loads only the selected artifact. Use `{ artifact: "scalar" }` to require the
portable artifact or `{ artifact: "simd128" }` to require SIMD support. The
returned frozen `posture` records the capability evidence, selected artifact,
selection reason, limits, and ABI version.
The reviewed JavaScript embeds and verifies the SHA-256 digest of each shipped
artifact before instantiation; `posture.artifactSha256` reports the selected
digest. `artifacts/SHA256SUMS` records the same values for release tooling.

The package uses no `eval` or `new Function`. It works as an ES module in Node
and browsers where the `.wasm` artifacts can be fetched. A restrictive browser
CSP must allow WebAssembly compilation with `script-src 'wasm-unsafe-eval'`;
this does not enable JavaScript `eval`. Callers may instead provide artifact
bytes or URLs through `scalarArtifact` and `simdArtifact`. A custom source must
also provide the matching lowercase `scalarArtifactSha256` or
`simdArtifactSha256`; custom artifacts are never instantiated without an
explicit digest.

## Security boundary

This package provides ordinary Base64 operations, not a secret API. Inputs are
snapshotted before validation, outputs are owned copies, and `*Into`
destinations are changed only after successful validation and capacity checks.
Each operation performs best-effort clearing only over the bounded input and
output ranges it could have touched. `dispose()` clears both complete scratch
capacities. Neither path can clear engine, GC, JIT, register, or historical
linear-memory copies.

Default limits are 1 MiB input, the corresponding padded output size, and 128
wasm pages (8 MiB). Lower limits may be configured at construction. The shipped
artifacts cannot grow beyond 128 pages.

## Codecs

- `Codecs.STANDARD`
- `Codecs.STANDARD_NO_PAD`
- `Codecs.URL_SAFE`
- `Codecs.URL_SAFE_NO_PAD`

The byte-only surface is `encodedLength`, `decodedLength`, `encode`, `decode`,
`encodeInto`, `decodeInto`, and WHATWG-compatible `decodeForgiving`.
