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
        pending_encode_array_assert == 1 {
            if ($0 ~ /^[[:space:]]*required == OUTPUT_LEN,/) {
                pending_encode_array_assert = 2
                next
            }
            failed = 1
        }
        pending_encode_array_assert == 2 {
            if ($0 ~ /^[[:space:]]*"base64 output array has incorrect length"/) {
                pending_encode_array_assert = 0
                next
            }
            failed = 1
        }
        /debug_assert!\(|debug_assert_eq!\(|debug_assert_ne!\(/ {
            next
        }
        FILENAME == "src/engine/encode.rs" && $0 ~ /^[[:space:]]*assert!\($/ {
            pending_encode_array_assert = 1
            next
        }
        /panic!\(|unreachable!\(|\.unwrap\(|\.expect\(|assert!\(|assert_eq!\(|assert_ne!\(/ {
            allowed = 0
            if ($0 ~ /assert!\(line_len != 0, "base64 line wrap length must be non-zero"\)/) {
                allowed = 1
            }
            if ($0 ~ /assert!\(len <= CAP, "visible length exceeds array capacity"\)/) {
                allowed = 1
            }
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
            if ($0 ~ /\.expect\("base64-ng encode_vec failed for byte input"\)/) {
                allowed = 1
            }
            if ($0 ~ /\.expect\("base64-ng encode_string failed for byte input"\)/) {
                allowed = 1
            }
            if ($0 ~ /\.expect\("base64-ng profile encode_vec failed for byte input"\)/) {
                allowed = 1
            }
            if ($0 ~ /\.expect\("base64-ng profile encode_string failed for byte input"\)/) {
                allowed = 1
            }
            if (!allowed) {
                printf "panic policy: unreviewed panic-like site in %s:%d: %s\n", FILENAME, FNR, $0 > "/dev/stderr"
                failed = 1
            }
        }
        END {
            exit failed || pending_encode_array_assert
        }
    ' "$file"
}

test -s docs/PANIC_POLICY.md

find src -name '*.rs' | sort | while IFS= read -r source_file; do
    case "$source_file" in
        src/kani_proofs.rs|src/tests.rs|src/simd/tests.rs|src/simd/wasm.rs|src/simd/x86_decode_tests.rs)
            continue
            ;;
    esac
    check_file "$source_file"
done

echo "panic policy: ok"
