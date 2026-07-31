# Commit 5 Correction Baseline Review

This replacement campaign was captured from clean, signed source commit
`9665094362c535550e3a7cb5d812bf3bccccb0b7` on an AMD Ryzen 9 9950X3D using
Rust 1.97.1. The machine-readable environment, raw samples, and checksum
manifest are authoritative; this note records the admission interpretation.

Both complete runs passed schema, correctness, allocation, exact-matrix, and
reproducibility validation. Each run contains 4,760 samples covering all 952
expected backend/profile/operation/length groups with five samples per group.
The host could execute scalar, SSSE3/SSE4.1, AVX2, and AVX-512 VBMI paths.

Every forced x86 SIMD tier has at least one measured row below the required
`0.95` ratio to the matching scalar median. SSSE3/SSE4.1 has no admissible
rows; AVX2 and AVX-512 VBMI have isolated admissible rows but also many
non-admissible rows. Production auto dispatch likewise has mixed results.
This campaign therefore does not support a new backend-wide performance
admission or a claim that an x86 SIMD tier is uniformly faster than scalar.

The result does not change 1.3.9 runtime behavior. It gives later 2.0 backend
work a clean, source-bound baseline and requires any future admission claim to
replace these results with complete evidence for the exact implementation.

NEON and wasm `simd128` were unavailable on this host. Their evidence must come
from the documented target-specific hardware or runtime campaigns.
