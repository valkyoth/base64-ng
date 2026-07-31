# Invalidated Commit 3 Campaign

The original `commit-3-amd-9950x3d-linux` campaign is intentionally removed
from retained performance evidence.

An external pentest over `v1.3.9..221c753` found that its
`environment.json` identified Commit 2
(`fa7ac3f18bf189cf6b452775b4b48765351e3bdb`) while also recording the entire
Commit 3 implementation as dirty and untracked source. The measurements
therefore were not cryptographically bound to committed source and cannot
satisfy the repository's evidence-integrity contract.

The invalid campaign had marked every measured x86 SIMD tier
non-admissible, so its removal does not revoke a backend admission. No timing,
ratio, or resource number from it may be cited in release notes or used for
future admission decisions.

The corrective checkpoint makes generation reject dirty trees, validates full
commit identifiers, requires complete measurement matrices, and rejects unsafe
CSV labels. A replacement campaign is accepted only when generated from the
clean signed corrective commit and retained in a separate follow-up commit.
