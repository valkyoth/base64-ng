# base64-ng 2.1.1

This maintenance release updates the build compiler to Rust 1.100.0. The
minimum supported compiler is Rust 1.90. The core, bytes, Tokio, serde and
sanitization packages and the npm WASM loader are selected for 2.1.1.

The updated integrations use bytes 1.12.2, Tokio 1.54.1, sanitization 2.2.0
and serde_json 1.0.154 for tests. The loader ships scalar and SIMD artifacts
built with the release compiler. The performance and protocol-registry
reference implementations retain the versions used for their comparisons.

The API snapshot nightly and Kani 0.67.0 remain pinned for comparison with
the existing inventories. No 2.1.0 campaign is claimed as a run against
the new compiler or the rebuilt WASM binaries.
