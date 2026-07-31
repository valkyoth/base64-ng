#!/usr/bin/env sh
set -eu

toolchain="${BASE64_NG_PROFILE_TOOLCHAIN:-}"

cargo_run() {
    if [ -n "$toolchain" ]; then
        cargo "+$toolchain" "$@"
    else
        cargo "$@"
    fi
}

doc="docs/2.0_PROFILES_AND_TERMINOLOGY.md"
device_queue="docs/2.0_DEVICE_VERIFICATION_QUEUE.md"

for required in \
    '`MIME_BODY_STRICT`' \
    '`PEM_BODY_LF`' \
    '`BCRYPT_ALPHABET_NO_PAD`' \
    '`PBKDF2_ALPHABET_NO_PAD`' \
    '`BINHEX_ALPHABET`' \
    '`IMAP_MUTF7_ALPHABET_NO_PAD`' \
    '`legacy::ASCII_WHITESPACE`' \
    'does not wipe on drop' \
    '`base64ct` 1.8.3' \
    '`base64` 0.23.0'
do
    if ! grep -F -q "$required" "$doc"; then
        echo "2.0 profiles: documentation is missing: $required" >&2
        exit 1
    fi
done

for device in 'Apple Silicon macOS' 'AArch64 Linux' 'Primary x86-64'; do
    if ! grep -F -q "$device" "$device_queue"; then
        echo "2.0 profiles: device queue is missing: $device" >&2
        exit 1
    fi
done

for symbol in \
    MIME_BODY_STRICT \
    PEM_BODY_LF \
    PEM_BODY_CRLF \
    BCRYPT_ALPHABET_NO_PAD \
    CRYPT_ALPHABET_NO_PAD \
    PBKDF2_ALPHABET_NO_PAD \
    BINHEX_ALPHABET \
    IMAP_MUTF7_ALPHABET_NO_PAD
do
    if ! rg -q "pub const $symbol" src/v2; then
        echo "2.0 profiles: public value is missing: $symbol" >&2
        exit 1
    fi
done

if rg -n -F \
    -e 'LegacyWhitespaceDecoder' \
    -e 'ASCII_WHITESPACE' \
    -e 'MIME_BODY_STRICT' \
    -e 'PBKDF2_ALPHABET_NO_PAD' \
    src/v2/secret.rs src/v2/secret_in_place.rs
then
    echo "2.0 profiles: ordinary compatibility leaked into secret modules" >&2
    exit 1
fi

grep -F -q 'base64ct = "=1.8.3"' perf/Cargo.toml
grep -F -q 'base64 = "=0.23.0"' perf/Cargo.toml

cargo_run test --all-features --lib 'v2::legacy_tests'
cargo_run test --all-features --lib 'v2::profile_tests'
cargo_run test --no-default-features --lib 'v2::legacy_tests'
cargo_run test --no-default-features --lib 'v2::profile_tests'

RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo_run test \
    --manifest-path perf/Cargo.toml \
    'v2_model::tests::pbkdf2_alphabet_profile_matches_pinned_base64ct'
RUSTFLAGS='--cfg base64_ng_perf_evidence' cargo_run test \
    --manifest-path perf/Cargo.toml \
    'v2_model::tests::binhex_and_imap_alphabet_profiles_match_pinned_base64'

scripts/check-2.0-migration-smoke.sh

echo "2.0 profiles: exact body, alphabet, and legacy-whitespace scopes ok"
