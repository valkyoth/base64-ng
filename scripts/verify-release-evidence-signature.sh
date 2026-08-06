#!/usr/bin/env sh
set -eu

manifest="${1:-target/release-evidence/FINAL-MANIFEST.txt}"
signature="${2:-${manifest}.sig}"
root="$(git rev-parse --show-toplevel)"
policy="$root/security/release-signers"
principal="1921261+eldryoth@users.noreply.github.com"
namespace="base64-ng-evidence-v2"
expected_fingerprint="SHA256:EoLRQ5k4J5pYz3UMFmkrV798gYFNkToGS2xEPvebqB4"

if [ ! -s "$manifest" ] || [ ! -s "$signature" ] || [ ! -s "$policy" ]; then
    echo "release evidence signature: manifest, signature, or signer policy is missing" >&2
    exit 1
fi

policy_key="$(mktemp "${TMPDIR:-/tmp}/base64-ng-evidence-key.XXXXXX")"
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
actual_fingerprint="$(ssh-keygen -lf "$policy_key" -E sha256 | awk 'NR == 1 { print $2 }')"
if [ "$actual_fingerprint" != "$expected_fingerprint" ]; then
    echo "release evidence signature: signer policy fingerprint is not authorized" >&2
    exit 1
fi

verification="$({
    ssh-keygen -Y verify \
        -f "$policy" \
        -I "$principal" \
        -n "$namespace" \
        -s "$signature" <"$manifest"
} 2>&1)" || {
    echo "release evidence signature: detached signature is not authorized" >&2
    printf '%s\n' "$verification" >&2
    exit 1
}

case "$verification" in
    *"Good \"$namespace\" signature for $principal with"*"$expected_fingerprint"*) ;;
    *)
        echo "release evidence signature: signer identity or fingerprint mismatch" >&2
        printf '%s\n' "$verification" >&2
        exit 1
        ;;
esac

echo "release evidence signature: authorized"
