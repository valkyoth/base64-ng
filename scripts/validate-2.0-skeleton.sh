#!/usr/bin/env sh
set -eu

for module in \
    alphabet \
    backend_health \
    incremental \
    incremental_decoder \
    lifecycle \
    ordinary \
    secret \
    specifications
do
    test -s "src/v2/$module.rs"
    grep -F -q "mod $module;" src/v2/mod.rs
done

test -s src/v2/ordinary_alloc.rs
grep -F -q 'mod ordinary_alloc;' src/v2/mod.rs

awk '
    /^#\[cfg\(test\)\]$/ {
        gated = 1
        next
    }
    gated && /^mod (fixtures|incremental_decoder_tests|incremental_decoder_unpadded_tests|incremental_encoder_tests|one_shot_tests|rfc4648_oracle);$/ {
        found[$2] = 1
        gated = 0
        next
    }
    gated && $0 !~ /^[[:space:]]*$/ {
        gated = 0
    }
    END {
        exit !(found["fixtures;"] && found["incremental_decoder_tests;"] && found["incremental_decoder_unpadded_tests;"] && found["incremental_encoder_tests;"] && found["one_shot_tests;"] && found["rfc4648_oracle;"])
    }
' src/v2/mod.rs

if rg -n 'rfc4648_oracle' src \
    --glob '!src/v2/mod.rs' \
    --glob '!src/v2/fixtures.rs' \
    --glob '!src/v2/incremental_decoder_tests.rs' \
    --glob '!src/v2/incremental_decoder_unpadded_tests.rs' \
    --glob '!src/v2/incremental_encoder_tests.rs' \
    --glob '!src/v2/one_shot_tests.rs' \
    --glob '!src/v2/ordinary.rs' \
    --glob '!src/v2/rfc4648_oracle.rs'
then
    echo "2.0 skeleton: test oracle referenced outside its gated boundary" >&2
    exit 1
fi

cargo test --lib 'v2::'

echo "2.0 skeleton: private boundaries and independent oracle ok"
