# Commit 34 X86 Dispatch Freeze Evidence

This focused campaign was captured from clean, signed source commit
`32cd285be9ab90a80db59b68ae49f1077b79a6a3` on an AMD Ryzen 9 9950X3D
with microcode `0xb404035`, Linux `x86_64`, and Rust 1.97.1. Commit 34 changes
selection policy and documentation, not the measured kernels.

Both files contain 15 samples for every exact backend, Standard/URL-safe
alphabet, padded/unpadded profile, and configured input length. Each sample
processes 16 MiB. The validator requires the frozen median advantage and a
one-sided paired sign-test probability at or below 0.05; environment variables
may raise but cannot lower the release thresholds.

Encode passed the automatic matrix. The weakest observed median ratios were:

- SSSE3/SSE4.1 to scalar: 1.8435.
- AVX2 to scalar: 1.0570.
- AVX-512 VBMI to AVX2 at automatic sizes: 1.1521.

Strict decode admitted SSSE3/SSE4.1 and AVX2. AVX-512 VBMI strict decode
reached only 1.0166 relative to AVX2 at the weakest automatic-size cell, below
the frozen 1.02 requirement. A smaller seven-sample campaign independently
missed the same requirement at another 64 KiB cell. Commit 34 therefore removes
automatic AVX-512 strict decode while retaining its exact/static backend,
correctness evidence, and observational measurements.

These results bind only this exact host and source. They do not substitute for
the separately required AArch64, wasm-runtime, or candidate-backend evidence.
