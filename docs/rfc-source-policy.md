# RFC Source Policy

Status: Commit 4 source-lock contract for the 2.0 development line.

RFC 4648 is the normative primary source for the ordinary strict Base64
contract. The repository stores the exact RFC Editor plain-text publication at
`rfc/rfc4648.txt`. That file retains its original notices and is not covered by
the project's MIT/Apache-2.0 license grant.

Published RFC bytes are immutable. Errata are recorded in
`rfc/rfc4648-errata.tsv`; accepted errata affect requirements, implementation,
or tests without modifying the publication. Reported errata are reviewed but
do not silently acquire Verified status.

`scripts/verify-rfcs.sh` is offline and blocks changed bytes, newline
normalization, source-manifest drift, missing or extra files, stale errata
decisions, unmapped requirements, and accidental Cargo/npm publication.
Network access occurs only through explicit maintainer commands:

- `scripts/fetch-rfcs.sh` downloads HTTPS sources into `target/`.
- `scripts/lock-rfcs.sh` copies a checksum-matching download into the lock.
- `scripts/check-rfc-errata-live.py` performs an opt-in RFC Editor drift check.

The `.gitattributes` rule marks RFC text as binary for Git normalization
purposes. Builds, tests, and package creation never fetch specifications.
