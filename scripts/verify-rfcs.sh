#!/usr/bin/env sh
set -eu

rfc_dir="${BASE64_NG_RFC_DIR:-rfc}"
skip_package="${BASE64_NG_RFC_SKIP_PACKAGE:-0}"

expected_files="$(
    printf '%s\n' \
        README.md \
        SHA256SUMS \
        SOURCES \
        rfc2045-errata.tsv \
        rfc2045-requirements.json \
        rfc2045.txt \
        rfc4648-errata.tsv \
        rfc4648-requirements.json \
        rfc4648.txt \
        rfc7468-errata.tsv \
        rfc7468-requirements.json \
        rfc7468.txt
)"
actual_files="$(
    for path in "$rfc_dir"/*; do
        test -f "$path" || continue
        basename "$path"
    done | LC_ALL=C sort
)"

if [ "$actual_files" != "$expected_files" ]; then
    echo "RFC verify: missing, extra, or unlocked file in $rfc_dir" >&2
    printf 'expected:\n%s\nactual:\n%s\n' "$expected_files" "$actual_files" >&2
    exit 1
fi

(
    cd "$rfc_dir"
    sha256sum -c SHA256SUMS
)

python3 scripts/validate-rfc4648.py "$rfc_dir"
python3 scripts/validate-rfc2045.py "$rfc_dir"
python3 scripts/validate-rfc7468.py "$rfc_dir"

if [ "$skip_package" != "1" ]; then
    package_list="$(mktemp)"
    trap 'rm -f "$package_list"' EXIT HUP INT TERM
    for package in \
        base64-ng \
        base64-ng-derive \
        base64-ng-mime \
        base64-ng-pem \
        base64-ng-sanitization \
        base64-ng-serde \
        base64-ng-bytes \
        base64-ng-subtle \
        base64-ng-tokio
    do
        cargo package --locked --allow-dirty --list -p "$package" >"$package_list"
        if grep -E '(^|/)rfc/' "$package_list"; then
            echo "RFC verify: $package contains locked RFC material" >&2
            exit 1
        fi
    done

    if find . -path ./target -prune -o -path ./.git -prune -o \
        -name package.json -print | grep -q .; then
        python3 scripts/validate-rfc4648.py "$rfc_dir" --npm-packages
    fi
fi

if [ "${BASE64_NG_CHECK_LIVE_RFC_ERRATA:-0}" = "1" ]; then
    scripts/check-rfc-errata-live.py
fi

echo "RFC verify: offline source, errata, requirements, and packaging checks ok"
