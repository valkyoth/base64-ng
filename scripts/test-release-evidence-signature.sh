#!/usr/bin/env sh
set -eu

tmp="$(mktemp -d)"
cleanup() {
    rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

key="$tmp/key"
principal="evidence-test@example.invalid"
namespace="base64-ng-evidence-v2"
manifest="$tmp/FINAL-MANIFEST.txt"
verifier="$tmp/verify.sh"

ssh-keygen -q -t ed25519 -N '' -f "$key"
fingerprint="$(ssh-keygen -lf "$key.pub" -E sha256 | awk 'NR == 1 { print $2 }')"
printf '%s namespaces="%s" %s\n' \
    "$principal" "$namespace" "$(cat "$key.pub")" >"$tmp/release-signers"
printf 'signed evidence\n' >"$manifest"
ssh-keygen -Y sign -f "$key" -n "$namespace" "$manifest" >/dev/null

cp scripts/verify-release-evidence-signature.sh "$verifier"
sed -i \
    -e "s#security/release-signers#../release-signers#" \
    -e "s#1921261+eldryoth@users.noreply.github.com#$principal#" \
    -e "s#SHA256:EoLRQ5k4J5pYz3UMFmkrV798gYFNkToGS2xEPvebqB4#$fingerprint#" \
    "$verifier"

repo="$tmp/repo"
mkdir -p "$repo/scripts"
cp "$verifier" "$repo/scripts/verify.sh"
cp "$tmp/release-signers" "$repo/release-signers"
(
    cd "$repo"
    git init -q
    scripts/verify.sh "$manifest" "$manifest.sig" >/dev/null
)

printf 'tampered evidence\n' >"$manifest"
if (
    cd "$repo"
    scripts/verify.sh "$manifest" "$manifest.sig" >/dev/null 2>&1
); then
    echo "release evidence signature tests: accepted a tampered manifest" >&2
    exit 1
fi

echo "release evidence signature tests: ok"
