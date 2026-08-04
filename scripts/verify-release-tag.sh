#!/usr/bin/env sh
set -eu

tag="${1:-}"
if [ -z "$tag" ]; then
    echo "usage: scripts/verify-release-tag.sh <tag>" >&2
    exit 2
fi

root="$(git rev-parse --show-toplevel)"
policy="$root/security/release-signers"
expected_principal="1921261+eldryoth@users.noreply.github.com"
expected_fingerprint="SHA256:EoLRQ5k4J5pYz3UMFmkrV798gYFNkToGS2xEPvebqB4"

if [ ! -s "$policy" ]; then
    echo "release tag verification: authorized signer policy is missing" >&2
    exit 1
fi

policy_key="$(mktemp "${TMPDIR:-/tmp}/base64-ng-release-key.XXXXXX")"
cleanup() {
    rm -f "$policy_key"
}
trap cleanup EXIT INT TERM
awk '{
    for (field = 1; field <= NF; field += 1) {
        if ($field ~ /^(ssh-|ecdsa-)/) {
            print $field " " $(field + 1)
            exit
        }
    }
}' "$policy" >"$policy_key"
policy_fingerprint="$(ssh-keygen -lf "$policy_key" -E sha256 | awk 'NR == 1 { print $2 }')"
if [ "$policy_fingerprint" != "$expected_fingerprint" ]; then
    echo "release tag verification: signer policy fingerprint is not authorized" >&2
    exit 1
fi

verification="$(
    git \
        -c gpg.format=ssh \
        -c gpg.ssh.allowedSignersFile="$policy" \
        verify-tag --raw "$tag" 2>&1
)" || {
    echo "release tag verification: $tag has no valid authorized signature" >&2
    printf '%s\n' "$verification" >&2
    exit 1
}

actual_principal="$(
    printf '%s\n' "$verification" |
        sed -n 's/^Good "git" signature for \([^ ]*\) with .*$/\1/p' |
        sed -n '1p'
)"
actual_fingerprint="$(
    printf '%s\n' "$verification" |
        sed -n 's/^Good "git" signature for .* key \(SHA256:[A-Za-z0-9+/]*\)$/\1/p' |
        sed -n '1p'
)"

if [ "$actual_principal" != "$expected_principal" ] || \
    [ "$actual_fingerprint" != "$expected_fingerprint" ]; then
    echo "release tag verification: $tag was not signed by an authorized release signer" >&2
    exit 1
fi

echo "release tag verification: $tag is signed by $actual_principal ($actual_fingerprint)"
