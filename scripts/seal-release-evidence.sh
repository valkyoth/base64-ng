#!/usr/bin/env sh
set -eu

tag="${1:-}"
case "$tag" in
    v[0-9]*.[0-9]*.[0-9]*) ;;
    *)
        echo "usage: BASE64_NG_EVIDENCE_SIGNING_KEY=/path/to/key scripts/seal-release-evidence.sh vX.Y.Z" >&2
        exit 2
        ;;
esac

if [ -z "${BASE64_NG_EVIDENCE_SIGNING_KEY:-}" ]; then
    echo "release evidence sealing: dedicated evidence-only key is required" >&2
    exit 1
fi
manifest="target/release-evidence/FINAL-MANIFEST.txt"
scripts/sign-release-evidence.sh "$manifest"
unset BASE64_NG_EVIDENCE_SIGNING_KEY
scripts/verify-release-evidence-signature.sh "$manifest" "$manifest.sig"
scripts/validate-release-readiness.sh "$tag"
echo "release evidence sealing: isolated signing and readiness checks passed"
