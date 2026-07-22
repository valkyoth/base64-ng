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
            if ($0 ~ /unreachable!\("tokio .* writer inner writer was already taken"\)/) {
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
            if ($0 ~ /panic!\("base64-ng-sanitization locked secret integrity failure: \{error\}"\)/) {
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

for test_file in src/*_tests.rs; do
    test -e "$test_file" || continue
    module_name="$(basename "$test_file" .rs)"
    if ! awk -v module_name="$module_name" '
        /^#\[cfg\(test\)\]/ {
            saw_cfg_test = 1
            next
        }
        saw_cfg_test && /^[[:space:]]*$/ {
            next
        }
        saw_cfg_test {
            expected = "^[[:space:]]*mod " module_name ";"
            if ($0 ~ expected) {
                found = 1
            }
            saw_cfg_test = 0
        }
        END {
            exit found ? 0 : 1
        }
    ' src/lib.rs; then
        echo "panic policy: $test_file must be declared behind #[cfg(test)] in src/lib.rs" >&2
        exit 1
    fi
done

find src crates/*/src -name '*.rs' | sort | while IFS= read -r source_file; do
    case "$source_file" in
        src/*_tests.rs|src/kani_proofs.rs|src/tests.rs|src/simd/tests.rs|src/simd/wasm.rs|src/simd/neon_decode_tests.rs|src/simd/x86_decode_tests.rs|crates/*/src/tests.rs|crates/*/src/*_tests.rs)
            continue
            ;;
    esac
    check_file "$source_file"
done

echo "panic policy: ok"
