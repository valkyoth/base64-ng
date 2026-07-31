#!/usr/bin/env sh
set -eu

for module in \
    alphabet \
    backend_health \
    incremental \
    ordinary \
    secret \
    specifications
do
    test -s "src/v2/$module.rs"
    grep -F -q "mod $module;" src/v2/mod.rs
done

awk '
    /^#\[cfg\(test\)\]$/ {
        gated = 1
        next
    }
    gated && /^mod (fixtures|incremental_encoder_tests|rfc4648_oracle);$/ {
        found[$2] = 1
        gated = 0
        next
    }
    gated && $0 !~ /^[[:space:]]*$/ {
        gated = 0
    }
    END {
        exit !(found["fixtures;"] && found["incremental_encoder_tests;"] && found["rfc4648_oracle;"])
    }
' src/v2/mod.rs

if rg -n 'rfc4648_oracle' src \
    --glob '!src/v2/mod.rs' \
    --glob '!src/v2/fixtures.rs' \
    --glob '!src/v2/incremental_encoder_tests.rs' \
    --glob '!src/v2/ordinary.rs' \
    --glob '!src/v2/rfc4648_oracle.rs'
then
    echo "2.0 skeleton: test oracle referenced outside its gated boundary" >&2
    exit 1
fi

cargo test --lib 'v2::'

echo "2.0 skeleton: private boundaries and independent oracle ok"
