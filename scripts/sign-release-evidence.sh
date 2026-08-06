#!/usr/bin/env sh
set -eu

manifest="${1:-target/release-evidence/FINAL-MANIFEST.txt}"
signature="${manifest}.sig"
namespace="base64-ng-evidence-v2"
signing_key="${BASE64_NG_EVIDENCE_SIGNING_KEY:-$(git config --get user.signingkey || true)}"

case "$signing_key" in
    *.pub) signing_key="${signing_key%.pub}" ;;
esac
if [ -z "$signing_key" ] || [ ! -f "$signing_key" ]; then
    echo "release evidence signing: set BASE64_NG_EVIDENCE_SIGNING_KEY to the authorized private SSH key" >&2
    exit 1
fi
if [ ! -s "$manifest" ]; then
    echo "release evidence signing: manifest is missing or empty: $manifest" >&2
    exit 1
fi

rm -f "$signature"
ssh-keygen -Y sign -f "$signing_key" -n "$namespace" "$manifest" >/dev/null
scripts/verify-release-evidence-signature.sh "$manifest" "$signature" >/dev/null
echo "release evidence signing: wrote $signature"
