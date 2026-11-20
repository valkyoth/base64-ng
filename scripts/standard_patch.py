"""Standard maintenance closeout for the reviewed 2.1.1 compiler/dependency delta."""
import tomllib

import scoped_candidate as metadata
from scoped_campaign import git
from scoped_evidence import read_regular


RELEASE = {"version": "2.1.1", "policy": "selective-patch"}
REVIEWED = "9a045427e28ad431c4405dfc5bf3ae6f4c5e7950"
# Only these regular-file blobs may replace the original CI inputs. Keep
# workflow paths out of CLOSEOUT_PATHS (which permits further content edits).
CI_CORRECTION = {
    ".github/workflows/ci.yml": "bdf95251ce5c7213910b7570ca0d90efe482084b",
    "scripts/test-ci-release-mode.py": "a6abdfe3090e61023f2ac1506c1dfc3d5c66b93a",
}
CLOSEOUT_PATHS = {
    "security/pentest/v2.1.1.md", "docs/RELEASE_STANDARD.md",
    "scripts/scoped_candidate.py", "scripts/standard_patch.py", "scripts/standard_release.py",
    "scripts/test-standard-release.py", "scripts/validate-release-readiness.sh",
    "scripts/test-release-readiness.sh",
}
CHECKS = {"security-review", "artifact-inspection", "compiler-and-dependency-validation"}


def plan(root):
    return tomllib.loads(read_regular(root / "release-crates.toml").decode())


def impact(root, current):
    git(root, "merge-base", "--is-ancestor", REVIEWED, current["commit"])
    paths = sorted(filter(None, git(root, "diff", "--no-renames", "--name-only", "-z",
                                    REVIEWED, current["commit"]).decode().split("\0")))
    unexpected = set(paths) - CLOSEOUT_PATHS
    if unexpected == CI_CORRECTION.keys() and all(
        git(root, "ls-tree", current["commit"], "--", path).decode().rstrip("\n")
        == f"100644 blob {blob}\t{path}" for path, blob in CI_CORRECTION.items()
    ):
        unexpected.clear()
    if unexpected:
        raise ValueError("change outside the reviewed maintenance scope: " +
                         ", ".join(sorted(unexpected)))
    return {"baseline": {"commit": REVIEWED,
                         "tree": git(root, "rev-parse", REVIEWED + "^{tree}").decode().strip()},
            "paths": paths}


def ci_jobs():
    # The pinned CI correction changes provisioning, not the required job matrix.
    jobs = {"Format, lint, test, and audit", "2.1 feature gates", "Miri",
            "Address, Leak, and Thread Sanitizers", "Big-endian QEMU functional evidence",
            "RISC-V QEMU and RVV candidate evidence", "AArch64 SVE QEMU candidate evidence"}
    for version in ("1.90.0", "1.91.0", "1.92.0", "1.93.0", "1.94.0", "1.95.0",
                    "1.96.0", "1.96.1", "1.97.0", "1.97.1", "1.98.0", "1.98.1",
                    "1.99.0", "1.99.1", "1.100.0"):
        jobs.add(f"Rust compatibility ({version})")
    for target in ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu",
                   "x86_64-unknown-freebsd", "wasm32-unknown-unknown", "thumbv7em-none-eabihf"):
        jobs.update((f"Rust 1.90 target contract ({target})", f"no_std target build ({target})"))
    for os in ("ubuntu-latest", "macos-latest", "macos-15", "macos-26", "macos-15-intel",
               "windows-latest"):
        jobs.add(f"Platform tests ({os})")
    return jobs


def verify(root, evidence, current, inventory):
    value = plan(root)
    if (value["release"] != RELEASE or value["npm"]["publish"] is not True
            or value["npm"]["version"] != RELEASE["version"]):
        raise ValueError("not the reviewed maintenance release plan")
    selected = {name for name, entry in value["crates"].items() if entry["publish"] is True}
    metadata.verify_ci_jobs(evidence, current, inventory, ci_jobs())
    metadata.verify_build(evidence, current, inventory)
    metadata.verify_package_set(root, evidence, current, inventory, value, selected)

    expected = {"RELEASE-CHECKLIST.json", "candidate/security-review.md", metadata.CI,
                metadata.RUST, metadata.NPM, *metadata.SBOMS, "sbom-MANIFEST.txt",
                "reproducible/MANIFEST.txt"}
    expected.update("reproducible/" + name for pair in metadata.REPRO_PAIRS for name in pair)
    expected.update("candidate/npm/" + name for name in (
        "package.tgz", "files.txt", "PROVENANCE.json", "base64-ng-scalar.wasm", "base64-ng-simd128.wasm"))
    for name in selected:
        expected.update((f"candidate/rust/{name}-{RELEASE['version']}.crate",
                         f"candidate/rust/{name}-files.txt"))
    if (set(inventory["files"]) != expected or set(inventory["directories"]) !=
            {"candidate", "candidate/rust", "candidate/npm", "reproducible"}):
        raise ValueError("unexpected files in maintenance evidence collection")
