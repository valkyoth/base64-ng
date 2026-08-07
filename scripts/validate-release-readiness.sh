#!/usr/bin/env sh
set -eu

tag="${1:-}"
case "$tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *)
        echo "usage: scripts/validate-release-readiness.sh vX.Y.Z" >&2
        exit 2
        ;;
esac

version="${tag#v}"
release_notes="release-notes/RELEASE_NOTES_${version}.md"
pentest_report="security/pentest/${tag}.md"
spdx="${BASE64_NG_SBOM_DIR:-target/release-evidence}/base64-ng.spdx.json"
cyclonedx="${BASE64_NG_SBOM_DIR:-target/release-evidence}/base64-ng.cyclonedx.json"
evidence_manifest="target/release-evidence/FINAL-MANIFEST.txt"
equivalence_manifest="target/release-evidence/EQUIVALENCE-MANIFEST.txt"

exact_key() {
    file="$1"
    key="$2"
    awk -v key="$key" '
        index($0, key "=") == 1 { count += 1; value = substr($0, length(key) + 2) }
        END { if (count == 1) print value; else exit 1 }
    ' "$file"
}

if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
    echo "tag already exists locally: ${tag}" >&2
    exit 1
fi

if [ -f PENTEST.md ]; then
    echo "root PENTEST.md is temporary scratch input and must be removed" >&2
    exit 1
fi

if [ ! -f "$release_notes" ]; then
    echo "missing release notes: ${release_notes}" >&2
    exit 1
fi

if [ ! -f "$pentest_report" ]; then
    echo "missing pentest report: ${pentest_report}" >&2
    exit 1
fi

if ! git cat-file -e "HEAD:${pentest_report}" 2>/dev/null; then
    echo "pentest report must be committed in tag candidate: ${pentest_report}" >&2
    exit 1
fi

grep -q '^Status: PASS$' "$pentest_report"
grep -Eq '^Reviewed-Commit: [0-9a-f]{40}$' "$pentest_report"
grep -Eq '^Tester: .+' "$pentest_report"
grep -Eq '^Scope: .+' "$pentest_report"
grep -Eq '^Date: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$pentest_report"

if [ ! -s "$spdx" ]; then
    echo "missing or empty SPDX SBOM evidence: ${spdx}" >&2
    exit 1
fi

if [ ! -s "$cyclonedx" ]; then
    echo "missing or empty CycloneDX SBOM evidence: ${cyclonedx}" >&2
    exit 1
fi

if [ ! -s "$evidence_manifest" ]; then
    echo "missing exact-candidate release evidence index: ${evidence_manifest}" >&2
    exit 1
fi
scripts/verify-release-evidence-signature.sh "$evidence_manifest" "${evidence_manifest}.sig"

head_commit="$(git rev-parse HEAD)"
if ! grep -F -q "commit=$head_commit" "$evidence_manifest" ||
    ! grep -F -q 'tree_state=clean' "$evidence_manifest"
then
    echo "release evidence index is not bound to clean HEAD ${head_commit}" >&2
    exit 1
fi

evidence_mode="$(exact_key "$evidence_manifest" evidence_mode)" || {
    echo "release evidence index has no singleton evidence_mode" >&2
    exit 1
}
campaign_commit="$(exact_key "$evidence_manifest" campaign_commit)" || {
    echo "release evidence index has no singleton campaign_commit" >&2
    exit 1
}
release_commit="$(exact_key "$evidence_manifest" release_commit)" || {
    echo "release evidence index has no singleton release_commit" >&2
    exit 1
}
if [ "$release_commit" != "$head_commit" ]; then
    echo "release evidence release_commit does not match HEAD ${head_commit}" >&2
    exit 1
fi

case "$evidence_mode" in
    exact)
        if [ "$campaign_commit" != "$head_commit" ]; then
            echo "exact evidence campaign_commit does not match HEAD ${head_commit}" >&2
            exit 1
        fi
        ;;
    metadata-equivalent)
        if [ ! -s "$equivalence_manifest" ]; then
            echo "metadata-equivalent release lacks ${equivalence_manifest}" >&2
            exit 1
        fi
        equivalence_tmp="$(mktemp target/release-evidence/.equivalence.XXXXXX)"
        trap 'rm -f "$equivalence_tmp"' EXIT INT TERM
        python3 scripts/evidence-equivalence.py \
            --evidence-commit "$campaign_commit" \
            --release-commit "$head_commit" \
            --output "$equivalence_tmp"
        if ! cmp -s "$equivalence_tmp" "$equivalence_manifest"; then
            echo "release evidence equivalence manifest is stale or altered" >&2
            exit 1
        fi
        rm -f "$equivalence_tmp"
        trap - EXIT INT TERM
        ;;
    *)
        echo "unsupported release evidence mode: ${evidence_mode}" >&2
        exit 1
        ;;
esac

scripts/verify-release-evidence-artifacts.py \
    "$evidence_manifest" target/release-evidence

for current_manifest in \
    target/release-evidence/sbom-MANIFEST.txt \
    target/release-evidence/reproducible/MANIFEST.txt
do
    if [ "$(exact_key "$current_manifest" commit 2>/dev/null || true)" != "$head_commit" ] ||
        [ "$(exact_key "$current_manifest" tree_state 2>/dev/null || true)" != clean ]
    then
        echo "candidate package evidence is not bound to clean HEAD: ${current_manifest}" >&2
        exit 1
    fi
done

reviewed_commit="$(sed -n 's/^Reviewed-Commit: //p' "$pentest_report")"
if ! git cat-file -e "${reviewed_commit}^{commit}" 2>/dev/null; then
    echo "reviewed commit ${reviewed_commit} was not found" >&2
    exit 1
fi

head_parent="$(git rev-parse HEAD^)"
if [ "$reviewed_commit" != "$head_parent" ]; then
    echo "reviewed commit ${reviewed_commit} does not match first parent ${head_parent}" >&2
    exit 1
fi

changed_paths="$(git diff --name-only "$reviewed_commit" HEAD)"
if [ "$changed_paths" != "$pentest_report" ]; then
    echo "release report commit may only change ${pentest_report}" >&2
    echo "$changed_paths" >&2
    exit 1
fi
