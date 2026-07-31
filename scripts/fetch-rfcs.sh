#!/usr/bin/env sh
set -eu

rfc_dir="${BASE64_NG_RFC_DIR:-rfc}"
download_dir="${BASE64_NG_RFC_DOWNLOAD_DIR:-target/rfc-downloads}"
sources="$rfc_dir/SOURCES"

test -s "$sources"
mkdir -p "$download_dir"

tab="$(printf '\t')"
while IFS="$tab" read -r file url expected_sha; do
    case "$file" in
        ""|\#*) continue ;;
    esac
    case "$url" in
        https://*) ;;
        *)
            echo "RFC fetch: refusing non-HTTPS source: $url" >&2
            exit 1
            ;;
    esac
    case "$file" in
        */*|*".."*)
            echo "RFC fetch: invalid destination name: $file" >&2
            exit 1
            ;;
    esac
    if ! printf '%s\n' "$expected_sha" | grep -E -q '^[0-9a-f]{64}$'; then
        echo "RFC fetch: invalid SHA-256 for $file" >&2
        exit 1
    fi

    destination="$download_dir/$file"
    temporary="$destination.tmp"
    echo "RFC fetch: $url"
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
        --location --output "$temporary" "$url"
    test -s "$temporary"
    actual_sha="$(sha256sum "$temporary" | awk '{print $1}')"
    if [ "$actual_sha" != "$expected_sha" ]; then
        rm -f "$temporary"
        echo "RFC fetch: checksum mismatch for $file" >&2
        echo "expected: $expected_sha" >&2
        echo "actual:   $actual_sha" >&2
        exit 1
    fi
    mv "$temporary" "$destination"
done <"$sources"

echo "RFC fetch: downloaded locked sources to $download_dir"
