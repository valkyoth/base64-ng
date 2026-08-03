#!/usr/bin/env sh
set -eu

rfc_dir="${BASE64_NG_RFC_DIR:-rfc}"
download_dir="${BASE64_NG_RFC_DOWNLOAD_DIR:-target/rfc-downloads}"
sources="$rfc_dir/SOURCES"

test -s "$sources"

tab="$(printf '\t')"
while IFS="$tab" read -r file url expected_sha; do
    case "$file" in
        ""|\#*) continue ;;
    esac
    case "$url" in
        https://*) ;;
        *)
            echo "RFC lock: refusing non-HTTPS source: $url" >&2
            exit 1
            ;;
    esac
    case "$file" in
        */*|*".."*)
            echo "RFC lock: invalid destination name: $file" >&2
            exit 1
            ;;
    esac
    if ! printf '%s\n' "$expected_sha" | grep -E -q '^[0-9a-f]{64}$'; then
        echo "RFC lock: invalid SHA-256 for $file" >&2
        exit 1
    fi
    downloaded="$download_dir/$file"
    test -s "$downloaded"
    actual_sha="$(sha256sum "$downloaded" | awk '{print $1}')"
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "RFC lock: downloaded $file does not match SOURCES" >&2
        exit 1
    fi
    cp "$downloaded" "$rfc_dir/$file"
done <"$sources"

(
    cd "$rfc_dir"
    for file in \
        README.md \
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
    do
        sha256sum "$file"
    done >SHA256SUMS
)

echo "RFC lock: refreshed $rfc_dir/SHA256SUMS"
