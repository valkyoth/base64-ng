# Commit 3 Baseline Review

This campaign was captured on an AMD Ryzen 9 9950X3D using Rust 1.97.1. The
machine-readable environment and raw samples are authoritative; this note
records the admission interpretation.

Both complete runs passed the schema, correctness, allocation, and
reproducibility checks. The host could execute scalar, SSSE3/SSE4.1, AVX2, and
AVX-512 VBMI paths.

For 1 KiB and 64 KiB ordinary encode and strict-decode cases, every forced x86
SIMD tier was below the matching scalar median. Production auto dispatch
selected AVX-512 VBMI and was also below scalar. The generated
`admission.csv` therefore marks these rows
`non-admissible-below-scalar`.

This result does not change 1.3.9 runtime behavior in Commit 3. It prevents 2.0
from inheriting an unsupported acceleration claim. The later backend rebuild
and final dispatch commits must either provide improved evidence that meets the
retained threshold or keep the affected backend out of 2.0 dispatch.

NEON and wasm `simd128` were unavailable on this host. Their evidence must come
from the documented target-specific hardware/runtime campaigns.
