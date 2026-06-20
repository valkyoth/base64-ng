#!/usr/bin/env sh
set -eu

check_file() {
    file="$1"
    awk '
        BEGIN {
            failed = 0
        }
        /^#\[cfg\((test|kani)\)\]/ {
            exit failed
        }
        /^[[:space:]]*\/\/[!\/]/ {
            next
        }
        /panic!\(|unreachable!\(|\.unwrap\(|\.expect\(/ {
            allowed = 0
            if ($0 ~ /unreachable!\("stream .* was already taken"\)/) {
                allowed = 1
            }
            if ($0 ~ /unreachable!\("base64 encoder produced non-UTF-8 output"\)/) {
                allowed = 1
            }
            if ($0 ~ /_ => unreachable!\(\),/) {
                allowed = 1
            }
            if ($0 ~ /panic!\("encoded base64 length overflows usize"\)/) {
                allowed = 1
            }
            if (!allowed) {
                printf "panic policy: unreviewed panic-like site in %s:%d: %s\n", FILENAME, FNR, $0 > "/dev/stderr"
                failed = 1
            }
        }
        END {
            exit failed
        }
    ' "$file"
}

test -s docs/PANIC_POLICY.md

find src -name '*.rs' | sort | while IFS= read -r source_file; do
    case "$source_file" in
        src/kani_proofs.rs|src/tests.rs|src/simd/tests.rs)
            continue
            ;;
    esac
    check_file "$source_file"
done

echo "panic policy: ok"
