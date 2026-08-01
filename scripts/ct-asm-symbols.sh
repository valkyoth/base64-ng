#!/usr/bin/env sh

# Match a Rust function-definition label in GNU/ELF or Apple/Mach-O assembly.
# Mach-O adds one ABI underscore to Rust's already-underscored symbol.
ct_asm_symbol_is_defined() {
    assembly_file="$1"
    symbol_len="$2"
    symbol_name="$3"
    legacy_pattern="^[[:space:]]*_{1,2}ZN9base64_ng.*${symbol_len}${symbol_name}17h[[:xdigit:]]+E:"
    v0_pattern="^[[:space:]]*_{1,2}R.*9base64_ng.*${symbol_len}${symbol_name}[^:]*:"

    grep -E -q "$legacy_pattern" "$assembly_file" ||
        grep -E -q "$v0_pattern" "$assembly_file"
}
