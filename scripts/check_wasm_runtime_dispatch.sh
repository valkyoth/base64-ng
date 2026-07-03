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

    let mut input = [0u8; 96];
    fill_pattern(&mut input, 17);

    let mut encoded = [0u8; 128];
    let encoded_len = STANDARD
        .encode_slice(&input, &mut encoded)
        .map_err(|_| 4)?;
    if encoded_len != 128 {
        return Err(5);
    }

    let mut decoded = [0u8; 96];
    let decoded_len = STANDARD
        .decode_slice(&encoded[..encoded_len], &mut decoded)
        .map_err(|_| 6)?;
    if decoded_len != input.len() || decoded != input {
        return Err(7);
    }

    let input = &input[..95];
    let mut url_encoded = [0u8; 128];
    let url_encoded_len = URL_SAFE_NO_PAD
        .encode_slice(input, &mut url_encoded)
        .map_err(|_| 8)?;
    let mut url_decoded = [0u8; 95];
    let url_decoded_len = URL_SAFE_NO_PAD
        .decode_slice(&url_encoded[..url_encoded_len], &mut url_decoded)
        .map_err(|_| 9)?;
    if url_decoded_len != input.len() || url_decoded != input {
        return Err(10);
    }

    Ok(())
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
