#!/usr/bin/env sh
set -eu

ledger="docs/2.0_API_MIGRATION_LEDGER.md"
topology="docs/2.0_PACKAGE_TOPOLOGY.md"

require_text() {
    file="$1"
    expected="$2"

    if ! grep -F -q "$expected" "$file"; then
        echo "2.0 API ledger: $file is missing required text: $expected" >&2
        exit 1
    fi
}

test -s "$ledger"
test -s "$topology"

for disposition in Retain Rename Replace Move Remove; do
    require_text "$ledger" "**$disposition**"
done

for canonical_name in \
    encode_into \
    decode_into \
    encode_to_string \
    decode_to_vec \
    encode_in_place \
    decode_in_place \
    encoder \
    decoder \
    encode_redacted \
    decode_redacted
do
    require_text "$ledger" "\`$canonical_name\`"
done

for core_surface in \
    "Engine<A, PAD>" \
    "Profile<A, PAD>" \
    "Alphabet" \
    "EncodedBuffer" \
    "DecodedBuffer" \
    "SecretBuffer" \
    "CtEngine" \
    "LineWrap" \
    "EncodeError" \
    "DecodeError" \
    "runtime::*" \
    "clear-tail" \
    "STANDARD_NO_PAD" \
    "URL_SAFE_NO_PAD" \
    "MIME" \
    "PEM_CRLF" \
    "BCRYPT_NO_PAD" \
    "CRYPT_NO_PAD"
do
    require_text "$ledger" "$core_surface"
done

for package in \
    base64-ng \
    base64-ng-derive \
    base64-ng-sanitization \
    base64-ng-serde \
    base64-ng-bytes \
    base64-ng-subtle \
    base64-ng-tokio \
    base64-ng-mime \
    base64-ng-pem \
    base64-ng-multibase \
    base64-ng-imap \
    base64-ng-password \
    base64-ng-openpgp \
    base64-ng-wasm-loader
do
    require_text "$topology" "\`$package\`"
done

for edge in \
    "std -> alloc" \
    "stream -> std" \
    "checked-backend -> simd" \
    "secrets -> (no implicit feature)"
do
    require_text "$topology" "$edge"
done

require_text "$ledger" \
    "Every snapshot line has a disposition."
require_text "$ledger" \
    "is not undecided."
require_text "$ledger" \
    "\`secret::*\` is reserved exclusively for constant-time-oriented computation"
require_text "$topology" \
    "ordinary types keep identical size, alignment, \`needs_drop\`, methods,"
require_text "$topology" \
    "High assurance is an attested build/runtime policy, not a Cargo feature."

if ! grep -q '^secrets = \[\]$' Cargo.toml; then
    echo "2.0 API ledger: secrets must remain dependency-free" >&2
    exit 1
fi

require_text "$ledger" \
    "Commit 18 activates \`base64_ng::secret\` as the canonical storage and exposure"
require_text "$ledger" "\`Base64String<S>\`"
require_text "$ledger" "\`base64_ng::prelude\`"
require_text src/v2/mod.rs "pub mod secret;"

if ! grep -q '^checked-backend = \["simd"\]$' Cargo.toml; then
    echo "2.0 API ledger: checked-backend must imply simd exactly" >&2
    exit 1
fi

echo "2.0 API ledger: ok"
