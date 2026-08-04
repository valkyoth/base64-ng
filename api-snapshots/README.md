# Public API Snapshots

This directory records machine-generated public API at migration boundaries.
The `v1.3.9` snapshots are immutable authoritative input inventories. The
`v2.0.0` snapshots are the frozen release API for the complete synchronized
package family. CI preserves both the applicable 1.3.9 compatibility inventory
and the exact 2.0.0 public surface.

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

Commit 43 adds the new `base64-ng-mime` package snapshot for bounded RFC 2045
Section 6.8 content-transfer body operations. It has no frozen 1.3.9 baseline
because the package did not exist in that release.

Commit 44 adds the new `base64-ng-pem` package snapshot for bounded complete
RFC 7468 textual encoding operations. It has no frozen 1.3.9 baseline because
the package did not exist in that release.

Commit 45 adds the new `base64-ng-multibase` package snapshot for bounded,
strict Base64-family multibase operations. It has no frozen 1.3.9 baseline
because the package did not exist in that release.

Commit 46 adds the new `base64-ng-imap` package snapshot for bounded, strict
RFC 3501 Section 5.1.3 modified-Base64 payload operations. It has no frozen
1.3.9 baseline because the package did not exist in that release.

Commit 47 adds the new `base64-ng-password` package snapshot for bounded,
exact Passlib PBKDF2 and SHA-crypt field/record transforms. It has no frozen
1.3.9 baseline because the package did not exist in that release.

The pre-seal Commit 54 usability reopening adds the policy-carrying ordinary
`Base64String<S>` owner and focused ordinary prelude. The release owner
explicitly approved this API change before the final full-range pentest; the
updated `v2.0.0` core snapshot is the new reviewed release boundary.

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

The update mode existed for numbered development checkpoints. After Commit 54,
using it requires an explicit reviewed API change because `v2.0.0` is the
release snapshot. The command never rewrites the frozen 1.3.9 baseline:

```console
scripts/check-api-snapshots.sh --update
```

Normal CI uses the check mode and fails on drift:

```console
scripts/check-api-snapshots.sh
```
