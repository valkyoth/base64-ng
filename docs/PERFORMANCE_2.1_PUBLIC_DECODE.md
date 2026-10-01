# Commit 9 Public Decode Measurements

Development comparison, not hardware or release admission. Captured locally on
2026-10-01 with Rust 1.98.1, Linux x86_64, AMD Ryzen 9 9950X3D. Parent:
`0ab8189327f1a0750d4f8a9bfbd745ebae39a85f` (accepted Commit 8 follow-up).
Candidate: Commit 9 working tree, with the 512-encoded-byte integration floor.
No Intel, ARM, wasm or cross-host conclusion follows from this run.

The unchanged `perf/public-api` executable was built in release mode against
the parent and candidate with `simd` and `RUSTFLAGS=--cfg base64_ng_perf_evidence`.
Seven samples alternate execution order, use deterministic random input, and
measure complete warmed public calls. Every operation checks independent RFC
oracle results before/after timing and counts successful calls during timing.
Measured operations reported zero allocations. Input sizes below are decoded
payload bytes, not encoded input bytes. Ratios are parent time / candidate time.

| Public operation | 512-byte payload, four profiles | 64 KiB payload, four profiles |
| --- | --- | --- |
| Canonical `decode_into` | 1.50-1.82x | 4.04-4.19x |
| Historical `decode_slice` | 2.39-3.67x | 11.82-15.86x |
| Canonical `validate` | 1.45-1.49x | 6.17-6.42x |

The initial unrestricted vector route regressed 32-byte canonical payloads;
the integration floor keeps those on portable validation/writing. After that
change their observed ratios were 1.05-1.12x; 383/384-byte payloads also cover
the padded/unpadded crossover. Tiny historical calls remain the original
decoder with an additional eligibility branch: empty calls measured about
4.2-4.4 ns before and 4.9-5.0 ns after. Do not interpret nanosecond differences
on one host as a universal improvement or a no-regression guarantee.

Example commands (run separately in an accepted-parent checkout and candidate):

```sh
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo build --locked --release \
  --manifest-path perf/public-api/Cargo.toml --features simd
perf/public-api/target/release/base64-ng-public-api-perf \
  canonical decode sp 65536 random 4096 128 warm
```

Substitute `historical` or `validate`, and `sp`, `su`, `up`, `uu` for all four
strict profiles. Local uncommitted raw samples are in
`target/release-evidence/2.1-commit9-public-decode/local-comparison.json`.
The short paired runner from Commit 2 remains the governed revision-comparison
tool; this trusted-parent local experiment is not a signed evidence bundle.
Final thresholds, cold-call behavior, checked-backend costs and the broader
regression matrix still need the planned release campaign. No performance claim
is made here for the independent checks intentionally retained by
`checked-backend` or for ScalarReference validation.
