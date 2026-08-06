#!/usr/bin/env sh
set -eu

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

source_script="$(pwd)/scripts/validate-release-readiness.sh"
equivalence_script="$(pwd)/scripts/evidence-equivalence.py"
equivalence_allowlist="$(pwd)/security/evidence-reuse-allowlist.txt"
signature_verifier="$(pwd)/scripts/verify-release-evidence-signature.sh"
test_key="$tmp/evidence-key"
test_principal="release-readiness@example.invalid"
ssh-keygen -q -t ed25519 -N '' -f "$test_key"
test_fingerprint="$(ssh-keygen -lf "$test_key.pub" -E sha256 | awk 'NR == 1 { print $2 }')"
test_public_key="$(cat "$test_key.pub")"

make_fixture() {
    name="$1"
    repo="$tmp/$name"

    mkdir -p "$repo/scripts" "$repo/release-notes" "$repo/security/pentest" "$repo/target/release-evidence"
    cp "$source_script" "$repo/scripts/validate-release-readiness.sh"
    cp "$equivalence_script" "$repo/scripts/evidence-equivalence.py"
    cp "$signature_verifier" "$repo/scripts/verify-release-evidence-signature.sh"
    cp "$equivalence_allowlist" "$repo/security/evidence-reuse-allowlist.txt"
    sed -i \
        -e "s#1921261+eldryoth@users.noreply.github.com#$test_principal#" \
        -e "s#SHA256:EoLRQ5k4J5pYz3UMFmkrV798gYFNkToGS2xEPvebqB4#$test_fingerprint#" \
        "$repo/scripts/verify-release-evidence-signature.sh"
    printf '%s namespaces="base64-ng-evidence-v2" %s\n' \
        "$test_principal" "$test_public_key" >"$repo/security/release-signers"

    (
        cd "$repo"
        git init -q
        git config user.email "release-readiness@example.invalid"
        git config user.name "Release Readiness Test"
        printf 'fixture\n' >README.md
        printf '/target/\n' >.gitignore
        printf '# Release 2.0.0\n' >release-notes/RELEASE_NOTES_2.0.0.md
        git add README.md .gitignore release-notes/RELEASE_NOTES_2.0.0.md \
            scripts/validate-release-readiness.sh \
            scripts/evidence-equivalence.py \
            scripts/verify-release-evidence-signature.sh \
            security/evidence-reuse-allowlist.txt \
            security/release-signers
        git commit -q -m "fixture"
    )

    printf '%s\n' "$repo"
}

assert_fails_with() {
    expected="$1"
    shift

    if "$@" >"$tmp/stdout" 2>"$tmp/stderr"; then
        echo "expected command to fail: $*" >&2
        exit 1
    fi

    if ! grep -q "$expected" "$tmp/stderr"; then
        echo "expected stderr to contain: $expected" >&2
        echo "actual stderr:" >&2
        cat "$tmp/stderr" >&2
        exit 1
    fi
}

write_release_notes() {
    version="$1"
    printf '# Release %s\n' "$version" >"release-notes/RELEASE_NOTES_${version}.md"
}

write_sbom() {
    printf '{"spdxVersion":"SPDX-2.3"}\n' >target/release-evidence/base64-ng.spdx.json
    printf '{"bomFormat":"CycloneDX"}\n' >target/release-evidence/base64-ng.cyclonedx.json
}

write_evidence_index() {
    commit="$(git rev-parse HEAD)"
    mkdir -p target/release-evidence/reproducible
    cat >target/release-evidence/FINAL-MANIFEST.txt <<EOF
base64-ng final release evidence index

source:
commit=${commit}
tree_state=clean
evidence_mode=exact
campaign_commit=${commit}
release_commit=${commit}
EOF
    rm -f target/release-evidence/FINAL-MANIFEST.txt.sig
    ssh-keygen -Y sign -f "$test_key" -n base64-ng-evidence-v2 \
        target/release-evidence/FINAL-MANIFEST.txt >/dev/null
    cat >target/release-evidence/sbom-MANIFEST.txt <<EOF
source:
commit=${commit}
tree_state=clean
EOF
    cat >target/release-evidence/reproducible/MANIFEST.txt <<EOF
source:
commit=${commit}
tree_state=clean
EOF
}

write_equivalent_evidence_index() {
    campaign_commit="$1"
    release_commit="$(git rev-parse HEAD)"
    mkdir -p target/release-evidence/reproducible
    cat >target/release-evidence/FINAL-MANIFEST.txt <<EOF
base64-ng final release evidence index

source:
commit=${release_commit}
tree_state=clean
evidence_mode=metadata-equivalent
campaign_commit=${campaign_commit}
release_commit=${release_commit}
EOF
    rm -f target/release-evidence/FINAL-MANIFEST.txt.sig
    ssh-keygen -Y sign -f "$test_key" -n base64-ng-evidence-v2 \
        target/release-evidence/FINAL-MANIFEST.txt >/dev/null
    cat >target/release-evidence/sbom-MANIFEST.txt <<EOF
source:
commit=${release_commit}
tree_state=clean
EOF
    cat >target/release-evidence/reproducible/MANIFEST.txt <<EOF
source:
commit=${release_commit}
tree_state=clean
EOF
    python3 scripts/evidence-equivalence.py \
        --evidence-commit "$campaign_commit" \
        --output target/release-evidence/EQUIVALENCE-MANIFEST.txt >/dev/null
}

write_pentest() {
    tag="$1"
    reviewed_commit="$2"
    cat >"security/pentest/${tag}.md" <<EOF
Status: PASS
Reviewed-Commit: ${reviewed_commit}
Tester: Release Readiness Test
Scope: Fixture release metadata.
Date: 2026-07-03
EOF
}

repo="$(make_fixture bad-tag)"
(
    cd "$repo"
    assert_fails_with "usage: scripts/validate-release-readiness.sh vX.Y.Z" \
        scripts/validate-release-readiness.sh "0.2.0"
)

repo="$(make_fixture existing-tag)"
(
    cd "$repo"
    git tag v9.9.9
    assert_fails_with "tag already exists locally: v9.9.9" \
        scripts/validate-release-readiness.sh "v9.9.9"
)

repo="$(make_fixture scratch-pentest)"
(
    cd "$repo"
    printf 'scratch\n' >PENTEST.md
    assert_fails_with "root PENTEST.md is temporary scratch input" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture missing-release-notes)"
(
    cd "$repo"
    assert_fails_with "missing release notes: release-notes/RELEASE_NOTES_0.2.0.md" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture missing-report)"
(
    cd "$repo"
    write_release_notes "0.2.0"
    assert_fails_with "missing pentest report: security/pentest/v0.2.0.md" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture uncommitted-report)"
(
    cd "$repo"
    reviewed_commit="$(git rev-parse HEAD)"
    write_release_notes "0.2.0"
    write_pentest "v0.2.0" "$reviewed_commit"
    assert_fails_with "pentest report must be committed in tag candidate" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture missing-sbom)"
(
    cd "$repo"
    reviewed_commit="$(git rev-parse HEAD)"
    write_release_notes "0.2.0"
    write_pentest "v0.2.0" "$reviewed_commit"
    git add "security/pentest/v0.2.0.md"
    git commit -q -m "report"

    assert_fails_with "missing or empty SPDX SBOM evidence" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture wrong-reviewed-commit)"
(
    cd "$repo"
    base_branch="$(git symbolic-ref --short HEAD)"
    git checkout -q -b side
    printf 'side\n' >side.txt
    git add side.txt
    git commit -q -m "side"
    side_commit="$(git rev-parse HEAD)"
    git checkout -q "$base_branch"

    write_release_notes "0.2.0"
    write_sbom
    write_pentest "v0.2.0" "$side_commit"
    git add "security/pentest/v0.2.0.md"
    git commit -q -m "report"
    write_evidence_index

    assert_fails_with "does not match first parent" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture mixed-report-commit)"
(
    cd "$repo"
    reviewed_commit="$(git rev-parse HEAD)"
    write_release_notes "0.2.0"
    write_sbom
    write_pentest "v0.2.0" "$reviewed_commit"
    printf 'changed\n' >>README.md
    git add README.md "security/pentest/v0.2.0.md"
    git commit -q -m "report plus code"
    write_evidence_index

    assert_fails_with "release report commit may only change security/pentest/v0.2.0.md" \
        scripts/validate-release-readiness.sh "v0.2.0"
)

repo="$(make_fixture ready)"
(
    cd "$repo"
    reviewed_commit="$(git rev-parse HEAD)"
    write_release_notes "2.0.0"
    write_sbom
    write_pentest "v2.0.0" "$reviewed_commit"
    git add "security/pentest/v2.0.0.md"
    git commit -q -m "report"
    write_evidence_index

    scripts/validate-release-readiness.sh "v2.0.0"
)

repo="$(make_fixture metadata-equivalent)"
(
    cd "$repo"
    campaign_commit="$(git rev-parse HEAD)"
    mkdir -p docs
    printf 'reviewed release metadata\n' >docs/RELEASE.md
    git add docs/RELEASE.md
    git commit -q -m "release metadata"
    reviewed_commit="$(git rev-parse HEAD)"
    write_sbom
    write_pentest "v2.0.0" "$reviewed_commit"
    git add "security/pentest/v2.0.0.md"
    git commit -q -m "report"
    write_equivalent_evidence_index "$campaign_commit"

    scripts/validate-release-readiness.sh "v2.0.0"
)

repo="$(make_fixture metadata-equivalent-runtime-change)"
(
    cd "$repo"
    campaign_commit="$(git rev-parse HEAD)"
    mkdir -p src
    printf 'runtime change\n' >src/lib.rs
    git add src/lib.rs
    git commit -q -m "runtime"
    reviewed_commit="$(git rev-parse HEAD)"
    write_sbom
    write_pentest "v2.0.0" "$reviewed_commit"
    git add "security/pentest/v2.0.0.md"
    git commit -q -m "report"
    # Write the claimed index manually because the equivalence generator must
    # reject this range.
    release_commit="$(git rev-parse HEAD)"
    mkdir -p target/release-evidence/reproducible
    cat >target/release-evidence/FINAL-MANIFEST.txt <<EOF
source:
commit=${release_commit}
tree_state=clean
evidence_mode=metadata-equivalent
campaign_commit=${campaign_commit}
release_commit=${release_commit}
EOF
    cat >target/release-evidence/sbom-MANIFEST.txt <<EOF
source:
commit=${release_commit}
tree_state=clean
EOF
    cp target/release-evidence/sbom-MANIFEST.txt \
        target/release-evidence/reproducible/MANIFEST.txt
    printf 'forged\n' >target/release-evidence/EQUIVALENCE-MANIFEST.txt
    rm -f target/release-evidence/FINAL-MANIFEST.txt.sig
    ssh-keygen -Y sign -f "$test_key" -n base64-ng-evidence-v2 \
        target/release-evidence/FINAL-MANIFEST.txt >/dev/null

    assert_fails_with "non-metadata paths changed" \
        scripts/validate-release-readiness.sh "v2.0.0"
)
