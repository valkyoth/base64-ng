#!/usr/bin/env sh
set -eu

manifest="target/release-evidence/dudect/MANIFEST.txt"
backup="$(mktemp "${TMPDIR:-/tmp}/base64-ng-dudect-manifest.XXXXXX")"
had_manifest=0
if [ -e "$manifest" ]; then
    cp "$manifest" "$backup"
    had_manifest=1
fi
cleanup() {
    if [ "$had_manifest" -eq 1 ]; then
        mkdir -p "$(dirname "$manifest")"
        cp "$backup" "$manifest"
    else
        rm -f "$manifest"
    fi
    rm -f "$backup"
}
trap cleanup EXIT INT TERM

if BASE64_NG_RUN_DUDECT=1 \
    BASE64_NG_DUDECT_RELEASE=1 \
    BASE64_NG_DUDECT_THRESHOLD=1000000 \
    scripts/check_dudect.sh >/dev/null 2>&1
then
    echo "dudect release policy tests: weakened threshold was accepted" >&2
    exit 1
fi

if [ -e "$manifest" ]; then
    echo "dudect release policy tests: rejected policy left a final manifest" >&2
    exit 1
fi

echo "dudect release policy tests: ok"
