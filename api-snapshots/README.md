# Public API Snapshots

This directory records machine-generated public API at migration boundaries.
The `v1.3.9` snapshots are immutable authoritative input inventories. The
`2.0-development` snapshots record reviewed intentional additions and
companion migrations while CI preserves every applicable frozen 1.3.9 input
inventory.

Commit 38 pins the Tokio writer migration to shared 2.0 state, including its
accepted/committed progress and checked-recovery API.

Commit 39 pins the Serde migration to validated 2.0 codec specifications and
records its fixed-capacity ordinary and fixed-work secret field modules.

Commit 40 pins the sanitization migration to separately protected staging and
destination mappings plus the 2.0 fixed-work result gate.

Commit 41 pins the subtle migration to the sealed final 2.0 secret-owner
comparison trait, explicit public-length naming, and `Choice`-only result.

Commit 42 pins the derive migration to private `SecretArray<N>` storage,
mandatory sealed-codec and exposure policy, staged secret decode, wiping
secret encode, and removal of ordinary conversion and equality expansion.

Snapshots are generated with:

```text
cargo-public-api 0.52.0
Rust 1.97.1
--all-features --omit blanket-impls --omit auto-trait-impls
```

Blanket implementations are omitted because they are properties of upstream
traits rather than package-owned API. Auto-trait implementations are checked
separately by `scripts/check-2.0-feature-contract.sh`, where layout and
feature-unification behavior can be compared directly.

Update the development snapshot only when a numbered 2.0 commit intentionally
changes public API. The command never rewrites the frozen 1.3.9 baseline:

```console
scripts/check-api-snapshots.sh --update
```

Normal CI uses the check mode and fails on drift:

```console
scripts/check-api-snapshots.sh
```
