#!/usr/bin/env sh
set -eu

expected="SHA256:EoLRQ5k4J5pYz3UMFmkrV798gYFNkToGS2xEPvebqB4"
policy_key="$(mktemp "${TMPDIR:-/tmp}/base64-ng-release-policy-test.XXXXXX")"
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
}' security/release-signers >"$policy_key"
actual="$(ssh-keygen -lf "$policy_key" -E sha256 | awk 'NR == 1 { print $2 }')"
if [ "$actual" != "$expected" ]; then
    echo "release tag policy tests: authorized key fingerprint drifted" >&2
    exit 1
fi

if scripts/verify-release-tag.sh HEAD >/dev/null 2>&1; then
    echo "release tag policy tests: verifier accepted an unsigned commit object" >&2
    exit 1
fi

if git rev-parse --verify refs/tags/v1.3.9 >/dev/null 2>&1; then
    scripts/verify-release-tag.sh v1.3.9 >/dev/null
fi

echo "release tag policy tests: ok"
