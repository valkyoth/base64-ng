# Commit 11 WASM Measurements

These are bounded development measurements, not a release admission or a
portable performance guarantee. Browser/JIT versions, tiering, garbage
collection and host scheduling can affect results. CT/secret APIs are outside
this ordinary-data change.

## Method

`packages/base64-ng-wasm-loader/test/benchmark-validation.mjs` compares trusted
local baseline and current packages. The local run used the artifacts committed
at `781c2d8` and the Commit 11 working tree based on that commit. It checks
independent Base64 fixtures and decoded outputs outside the timer, warms both
implementations, alternates order for seven paired samples, and verifies output
lengths. All samples, Node/V8 identity and artifact digests are retained.
Both artifacts and all four strict profiles are measured at eight payload
sizes. Input is deterministic, buffers are warm, and checked-backend is not
enabled in the shipped SIMD artifact.

- `guest-decode`: direct Rust artifact ABI call with input already in guest
  memory, including strict validation, size checks and decoding. This is not
  an isolated intrinsic benchmark. Loader copies and per-call cleanup are excluded.
- `loader-decode`: supported allocating JavaScript decode, including copying,
  validation/decoding, output allocation and cleanup.
- `loader-decodeInto`: supported JavaScript decode into a reused caller buffer,
  including copying, validation/decoding and cleanup.

The ABI keeps preliminary validation and output high-water tracking. This
checkpoint does not remove redundant ABI passes or change JavaScript contracts.
Run after rebuilding and verifying current artifacts:

```sh
node packages/base64-ng-wasm-loader/test/benchmark-validation.mjs \
    /path/to/trusted/baseline/package > paired-wasm.json
```

The baseline executes local JavaScript and WASM; do not use untrusted source.
Build it independently and retain matching embedded integrity pins. This is
not a sandboxed competitor runner.

## Node Results

Runtime: Node v24.18.1, V8 13.6.233.17-node.50, Linux x64. The file contains
all 2688 unique artifact/profile/size/surface/sample/variant records. Ranges
below are across four profiles of median paired old/new elapsed-time ratios
for the SIMD artifact, not SIMD divided by scalar. Sizes are decoded bytes;
ratios above one mean faster.

| Payload bytes | Guest decode | Loader decode | Loader decodeInto |
| --- | ---: | ---: | ---: |
| 0 | 0.921-0.951x | 0.999-1.137x | 0.999-1.125x |
| 3 | 0.843-0.878x | 0.993-1.135x | 0.999-1.128x |
| 32 | 0.790-0.968x | 1.001-1.141x | 0.996-1.126x |
| 384 | 1.214-1.784x | 1.031-1.161x | 1.010-1.155x |
| 1024 | 1.433-2.182x | 1.048-1.225x | 1.037-1.189x |
| 4096 | 1.502-2.255x | 1.137-1.368x | 1.111-1.313x |
| 65536 | 1.519-2.289x | 1.318-1.693x | 1.322-1.698x |
| 786432 | 1.532-2.155x | 1.332-1.776x | 1.336-1.770x |

Every pair improves on all three surfaces at 4096 bytes and above. Tiny raw
guest calls regress: differences of per-case median times are about 2-3 ns for
empty input, 6-9 ns at three bytes, and 6-53 ns at 32 bytes. These are not
universal no-regression results. Loader overhead and JIT behavior mask or change
that pattern in the supported JavaScript surface. The unchanged scalar
artifact also varies (guest decode at 4096 bytes ranges 0.959-1.003x), so small
differences alone do not establish an algorithmic cause. No cross-runtime
performance claim is inferred from Node.

| Item | SHA-256 |
| --- | --- |
| Scalar, both builds | `246daf099d4b4df44ad5be246f01645f0dda1d70cb07cf3641f85d4a90bba232` |
| Baseline SIMD | `67f3f5c3ce82fd0fef2517853e34b82585b82e0e129307ec8ec4054889104406` |
| Commit 11 SIMD | `fae642dd36e6e1e2e42fa3aec074fbeec7133af8a634a20de9ec3db6f93ed0ea` |
| Paired JSON | `ae13fc2a700c423a94d0972f70b9cadd360a27414b4478fcef1a0b933064b137` |

Raw records: `target/release-evidence/2.1-commit11-wasm/paired-node.json`.
These are local development artifacts, not a signed exact-source release bundle.

## Verification Scope

The Rust gate covers active/MSRV scalar/SIMD no_std, alloc and std builds;
Wasmtime runs plain/checked byte/lane/unaligned tests, fault injection and
production-linked public tests. The npm gate covers deterministic and
path-independent builds, embedded pins, 26 Node tests, Wasmtime ABI self-tests,
exact tarball contents and installation. The scalar binary is unchanged.

Chromium and Firefox run the extracted package through scalar/SIMD differential
tests, vector-boundary lengths, malformed lanes and output sentinels. Their
smoke timings are observational, not paired baseline performance evidence.
Safari operator execution passed in the supplied logs described below. To
repeat on the Mac, enable Safari remote automation, run
`sh scripts/check-2.0-wasm-loader.sh`, then
`sh scripts/check_wasm_loader_browser_safari_dispatch.sh`. A missing-tool skip
does not satisfy the Safari requirement.

No new JavaScript options, shared-memory support, constant-time claim or native
register-clearing guarantee is introduced. Host JIT behavior remains outside
the Rust compiler assurance boundary.

## Mac Operator Follow-up

The maintainer supplied `base64-wasm-package.log` and `base64-wasm-safari.log`
after the instructions for `a74fed6`. The package log reports Node v24.21.0,
successful format/Clippy, deterministic and path-independent scalar/SIMD
rebuilds, matching integrity pins, all 26 Node tests (zero failures/skips),
and exact npm package/install smoke. It ends with `2.0 wasm loader: ok`.
Wasmtime is explicitly skipped because it is not installed on that Mac;
the separate local Linux Wasmtime results above are not relabeled as Mac runs.

The Safari log contains `BASE64_NG_WASM_LOADER_BROWSER_PASS` and `safari ok`.
Its eight-call smoke timings are encode 8 ms scalar / 3 ms SIMD and decode
13 ms scalar / 5 ms SIMD. These are observational scalar/SIMD timings, not a
paired pre/post Commit 11 comparison. The Node package benchmark similarly
compares scalar against SIMD, reporting 4.187x encode and 3.610x decode.

These are operator-reported execution results. Neither log prints Git HEAD,
the exact artifact digests, Rust version, macOS version or Safari version, so
they are not an independently bound exact-source/browser-version attestation.
They satisfy the requested operator smoke check without changing runtime code.

Copies are retained under `target/release-evidence/2.1-commit11-wasm/`:

| Log | SHA-256 |
| --- | --- |
| `base64-wasm-package.log` | `268992f3a7af7e20b0aba45e37d4427ec9abd59eed84b155ab6b834f7e9f7262` |
| `base64-wasm-safari.log` | `33e14037b98af5747d1c04e77b11568a7504f1329b31af40c601cce8e4401019` |
