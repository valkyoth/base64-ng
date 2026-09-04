//! Compile-time deployment-policy gates.

#[cfg(all(
    feature = "secrets",
    target_arch = "wasm32",
    not(feature = "allow-wasm32-best-effort-wipe")
))]
compile_error!(
    "base64-ng: wasm32 `secrets` builds use a compiler-fence-only wipe barrier that cannot \
     constrain downstream wasm runtime JITs. Enable \
     `allow-wasm32-best-effort-wipe` to accept this limitation and use \
     caller-owned, platform-approved zeroization for high-assurance wasm deployments."
);

#[cfg(all(base64_ng_require_high_assurance, not(feature = "secrets")))]
compile_error!(
    "base64-ng: base64_ng_require_high_assurance requires the `secrets` capability. \
     This build policy is not an assurance upgrade for ordinary public-data codecs."
);

#[cfg(all(
    base64_ng_require_high_assurance,
    feature = "secrets",
    not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        all(target_arch = "aarch64", base64_ng_aarch64_csdb_attested),
    ))
))]
compile_error!(
    "base64-ng: the high-assurance build policy requires an attested speculation \
     barrier. This target is unsupported or unattested; x86/x86_64 are eligible, \
     while AArch64 additionally requires --cfg base64_ng_aarch64_csdb_attested."
);

#[cfg(all(
    not(miri),
    feature = "secrets",
    not(feature = "allow-compiler-fence-only-wipe"),
    not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "wasm32",
        target_arch = "x86",
        target_arch = "x86_64",
    ))
))]
compile_error!(
    "base64-ng: this architecture has no native hardware wipe barrier in \
     base64-ng. Enable `allow-compiler-fence-only-wipe` only after reviewing \
     docs/UNSAFE.md and applying platform-approved memory hygiene controls."
);
