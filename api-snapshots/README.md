# Public API Snapshots

This directory records the machine-generated public API at migration
boundaries. The `v1.3.9` snapshots are the authoritative input inventory for
the 2.0 migration ledger.

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

Update snapshots only when intentionally changing the recorded baseline:

```console
scripts/check-api-snapshots.sh --update
```

Normal CI uses the check mode and fails on drift:

```console
scripts/check-api-snapshots.sh
```
