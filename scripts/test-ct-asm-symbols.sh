#!/usr/bin/env sh
set -eu

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

. scripts/ct-asm-symbols.sh

assert_matches() {
    fixture="$1"
    if ! ct_asm_symbol_is_defined "$fixture" 10 wipe_bytes; then
        echo "ct asm symbol test: expected definition was not recognized: $fixture" >&2
        exit 1
    fi
}

assert_rejected() {
    fixture="$1"
    if ct_asm_symbol_is_defined "$fixture" 10 wipe_bytes; then
        echo "ct asm symbol test: reference was mistaken for a definition: $fixture" >&2
        exit 1
    fi
}

printf '%s\n' '_RNvNtCsabc_9base64_ng7cleanup10wipe_bytes:' >"$tmp/elf-v0.s"
printf '%s\n' '__RNvNtCsabc_9base64_ng7cleanup10wipe_bytes:' >"$tmp/macho-v0.s"
printf '%s\n' '_ZN9base64_ng7cleanup10wipe_bytes17h0123456789abcdefE:' >"$tmp/elf-legacy.s"
printf '%s\n' '__ZN9base64_ng7cleanup10wipe_bytes17h0123456789abcdefE:' >"$tmp/macho-legacy.s"
printf '%s\n' 'callq _RNvNtCsabc_9base64_ng7cleanup10wipe_bytes' >"$tmp/call.s"
printf '%s\n' '.globl __RNvNtCsabc_9base64_ng7cleanup10wipe_bytes' >"$tmp/global.s"

assert_matches "$tmp/elf-v0.s"
assert_matches "$tmp/macho-v0.s"
assert_matches "$tmp/elf-legacy.s"
assert_matches "$tmp/macho-legacy.s"
assert_rejected "$tmp/call.s"
assert_rejected "$tmp/global.s"

echo "ct asm symbol test: ELF and Mach-O definition matching ok"
