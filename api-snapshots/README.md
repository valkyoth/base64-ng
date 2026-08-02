# Public API Snapshots

This directory records machine-generated public API at migration boundaries.
The `v1.3.9` snapshots are immutable authoritative input inventories. The
The `2.0-development` snapshots record reviewed intentional additions and
companion migrations while CI preserves every applicable frozen 1.3.9 input
inventory.

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
