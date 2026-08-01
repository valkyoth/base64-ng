#!/usr/bin/env sh
set -eu

for module in \
    alphabet \
    backend_health \
    bounded \
    chunks \
    const_transforms \
    formatting \
    incremental \
    incremental_decoder \
    in_place \
    lifecycle \
    legacy \
    ordinary \
    profiles \
    secret \
    secret_in_place \
    specifications
do
    test -s "src/v2/$module.rs"
    grep -F -q "mod $module;" src/v2/mod.rs
done

test -s src/v2/ordinary_alloc.rs
grep -F -q 'mod ordinary_alloc;' src/v2/mod.rs
test -s src/v2/append.rs
grep -F -q 'mod append;' src/v2/mod.rs
test -s src/v2/compat.rs
grep -F -q 'pub mod compat;' src/v2/mod.rs
test -s src/v2/web/mod.rs
test -s src/v2/web/decoder.rs
test -s src/v2/web/one_shot.rs
grep -F -q 'pub mod web;' src/v2/mod.rs
test -s src/v2/secret/exposure.rs
test -s src/v2/secret/owned.rs
grep -F -q 'pub mod secret;' src/v2/mod.rs

awk '
    /^#\[cfg\(test\)\]$/ {
        gated = 1
        next
    }
gated && /^mod (append_tests|chunk_tests|const_buffer_tests|fixtures|formatting_tests|incremental_decoder_tests|incremental_decoder_unpadded_tests|incremental_encoder_tests|in_place_tests|legacy_tests|one_shot_tests|profile_tests|rfc4648_oracle|secret_encoder_tests|secret_in_place_tests|secret_storage_tests|web_no_alloc_tests|web_tests);$/ {
        found[$2] = 1
        gated = 0
        next
    }
    gated && $0 !~ /^[[:space:]]*$/ {
        gated = 0
    }
    END {
        exit !(found["append_tests;"] && found["chunk_tests;"] && found["const_buffer_tests;"] && found["fixtures;"] && found["formatting_tests;"] && found["incremental_decoder_tests;"] && found["incremental_decoder_unpadded_tests;"] && found["incremental_encoder_tests;"] && found["in_place_tests;"] && found["legacy_tests;"] && found["one_shot_tests;"] && found["profile_tests;"] && found["rfc4648_oracle;"] && found["secret_encoder_tests;"] && found["secret_in_place_tests;"] && found["secret_storage_tests;"] && found["web_no_alloc_tests;"] && found["web_tests;"])
    }
' src/v2/mod.rs

if rg -n 'rfc4648_oracle' src \
    --glob '!src/v2/mod.rs' \
    --glob '!src/v2/const_buffer_tests.rs' \
    --glob '!src/v2/fixtures.rs' \
    --glob '!src/v2/incremental_decoder_tests.rs' \
    --glob '!src/v2/incremental_decoder_unpadded_tests.rs' \
    --glob '!src/v2/incremental_encoder_tests.rs' \
    --glob '!src/v2/one_shot_tests.rs' \
    --glob '!src/v2/ordinary.rs' \
    --glob '!src/v2/secret_encoder_tests.rs' \
    --glob '!src/v2/rfc4648_oracle.rs'
then
    echo "2.0 skeleton: test oracle referenced outside its gated boundary" >&2
    exit 1
fi

cargo test --lib 'v2::'

echo "2.0 skeleton: private boundaries and independent oracle ok"
