#!/usr/bin/env sh
set -eu

wasm_target="${1:-wasm32-unknown-unknown}"
smoke_dir="target/wasm-runtime-smoke"
manifest="$smoke_dir/Cargo.toml"
wasm_file="$smoke_dir/target/$wasm_target/release/base64_ng_wasm_runtime_smoke.wasm"

find_wasmtime() {
    if [ -n "${WASMTIME:-}" ]; then
        printf '%s\n' "$WASMTIME"
        return
    fi

    if command -v wasmtime >/dev/null 2>&1; then
        command -v wasmtime
        return
    fi

    if [ -x "$HOME/.wasmtime/bin/wasmtime" ]; then
        printf '%s\n' "$HOME/.wasmtime/bin/wasmtime"
        return
    fi

    printf '%s\n' ""
}

if ! rustup target list --installed 2>/dev/null | grep -F -x -q "$wasm_target"; then
    echo "wasm runtime dispatch: skipping $wasm_target; Rust target is not installed"
    exit 0
fi

mkdir -p "$smoke_dir/src"

cat >"$manifest" <<'EOF'
[package]
name = "base64-ng-wasm-runtime-smoke"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
base64-ng = { path = "../..", features = ["simd", "allow-wasm32-best-effort-wipe"] }
EOF

cat >"$smoke_dir/src/lib.rs" <<'EOF'
use base64_ng::runtime::{Backend, backend_report};
use base64_ng::{STANDARD, URL_SAFE_NO_PAD};

const MAX_INPUT: usize = 200;
const MAX_ENCODED: usize = 272;

#[unsafe(no_mangle)]
pub extern "C" fn base64_ng_wasm_runtime_smoke() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(code) => code,
    }
}

fn run() -> Result<(), i32> {
    let report = backend_report();
    if report.candidate != Backend::WasmSimd128 {
        return Err(1);
    }
    if report.active != Backend::WasmSimd128 {
        return Err(2);
    }
    if report.active_decode_backend() != Backend::WasmSimd128 {
        return Err(3);
    }

    let seeds = [1u8, 17, 93];
    for len in 0..=MAX_INPUT {
        for &seed in &seeds {
            check_standard(len, seed)?;
            check_url_safe_no_pad(len, seed)?;
        }
    }

    check_rejects_malformed()?;

    Ok(())
}

fn check_standard(len: usize, seed: u8) -> Result<(), i32> {
    let mut input = [0u8; MAX_INPUT];
    fill_pattern(&mut input[..len], seed);

    let mut encoded = [0u8; MAX_ENCODED];
    let encoded_len = STANDARD.encode_slice(&input[..len], &mut encoded).map_err(|_| 4)?;

    let mut reference = [0u8; MAX_ENCODED];
    let reference_len = reference_encode(&input[..len], &mut reference, STANDARD_ALPHABET, true);
    if encoded_len != reference_len || encoded[..encoded_len] != reference[..reference_len] {
        return Err(5);
    }

    let mut decoded = [0u8; MAX_INPUT];
    let decoded_len = STANDARD
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .map_err(|_| 6)?;
    if decoded_len != len || decoded[..len] != input[..len] {
        return Err(7);
    }

    Ok(())
}

fn check_url_safe_no_pad(len: usize, seed: u8) -> Result<(), i32> {
    let mut input = [0u8; MAX_INPUT];
    fill_pattern(&mut input[..len], seed);

    let mut encoded = [0u8; MAX_ENCODED];
    let encoded_len = URL_SAFE_NO_PAD
        .encode_slice(&input[..len], &mut encoded)
        .map_err(|_| 8)?;

    let mut reference = [0u8; MAX_ENCODED];
    let reference_len = reference_encode(&input[..len], &mut reference, URL_SAFE_ALPHABET, false);
    if encoded_len != reference_len || encoded[..encoded_len] != reference[..reference_len] {
        return Err(9);
    }

    let mut decoded = [0u8; MAX_INPUT];
    let decoded_len = URL_SAFE_NO_PAD
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .map_err(|_| 10)?;
    if decoded_len != len || decoded[..len] != input[..len] {
        return Err(11);
    }

    Ok(())
}

fn check_rejects_malformed() -> Result<(), i32> {
    let malformed: [&[u8]; 6] = [
        b"AAAAAAAAAAAAAAAA!",
        b"AAAAAAAAAAAAAAAA=A",
        b"AAAAAAAAAAAAAAAA====",
        b"AAAAAAAAAAAAAA=A",
        b"AAAAAAAAAAAAAAAAAAAAA",
        b"______________=_",
    ];
    for input in malformed {
        let mut output = [0x55u8; MAX_INPUT];
        if STANDARD.decode_slice(input, &mut output).is_ok()
            || URL_SAFE_NO_PAD.decode_slice(input, &mut output).is_ok()
        {
            return Err(12);
        }
    }
    Ok(())
}

const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn reference_encode(input: &[u8], output: &mut [u8], alphabet: &[u8; 64], padded: bool) -> usize {
    let mut read = 0;
    let mut write = 0;
    while read + 3 <= input.len() {
        let b0 = input[read];
        let b1 = input[read + 1];
        let b2 = input[read + 2];
        output[write] = alphabet[(b0 >> 2) as usize];
        output[write + 1] = alphabet[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize];
        output[write + 2] = alphabet[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize];
        output[write + 3] = alphabet[(b2 & 0x3f) as usize];
        read += 3;
        write += 4;
    }

    match input.len() - read {
        1 => {
            let b0 = input[read];
            output[write] = alphabet[(b0 >> 2) as usize];
            output[write + 1] = alphabet[((b0 & 0x03) << 4) as usize];
            write += 2;
            if padded {
                output[write] = b'=';
                output[write + 1] = b'=';
                write += 2;
            }
        }
        2 => {
            let b0 = input[read];
            let b1 = input[read + 1];
            output[write] = alphabet[(b0 >> 2) as usize];
            output[write + 1] = alphabet[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize];
            output[write + 2] = alphabet[((b1 & 0x0f) << 2) as usize];
            write += 3;
            if padded {
                output[write] = b'=';
                write += 1;
            }
        }
        _ => {}
    }

    write
}

fn fill_pattern(output: &mut [u8], seed: u8) {
    let mut value = seed.wrapping_mul(19);
    for byte in output {
        *byte = value;
        value = value.wrapping_add(73);
    }
}
EOF

echo "wasm runtime dispatch: building smoke module for $wasm_target"
CARGO_TARGET_DIR="$smoke_dir/target" \
RUSTFLAGS='-C target-feature=+simd128' \
    cargo build --manifest-path "$manifest" --target "$wasm_target" --release

if command -v node >/dev/null 2>&1; then
    echo "wasm runtime dispatch: running Node/V8 smoke"
    node - "$wasm_file" <<'EOF'
const fs = require("fs");
const wasm = fs.readFileSync(process.argv[2]);
WebAssembly.instantiate(wasm, {}).then(({ instance }) => {
  const result = instance.exports.base64_ng_wasm_runtime_smoke();
  if (result !== 0) {
    console.error(`Node/V8 wasm smoke failed with code ${result}`);
    process.exit(result);
  }
}).catch((error) => {
  console.error(error);
  process.exit(125);
});
EOF
else
    echo "wasm runtime dispatch: skipping Node/V8 smoke; node is not installed"
fi

wasmtime_bin="$(find_wasmtime)"
if [ -n "$wasmtime_bin" ]; then
    echo "wasm runtime dispatch: running Wasmtime smoke with $wasmtime_bin"
    mkdir -p "$smoke_dir/cache"
    "$wasmtime_bin" run -C cache=n --invoke base64_ng_wasm_runtime_smoke "$wasm_file" >/dev/null
else
    echo "wasm runtime dispatch: skipping Wasmtime smoke; set WASMTIME=/path/to/wasmtime"
fi

echo "wasm runtime dispatch: ok"
