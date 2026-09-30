#!/usr/bin/env python3
"""Compile the actual private preflight module and reject proof substitution."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile


root = Path(__file__).resolve().parent.parent
module = root / "src/decode_preflight.rs"
prefix = f'#[path = {json.dumps(str(module))}] mod decode_preflight;\n'
setup = '''
fn main() {
    let mut input = *b"Zm9v";
    let proof = decode_preflight::Preflight::validate(&input, 7_u8,
        |config, bytes| { assert_eq!(config, 7); assert_eq!(bytes, b"Zm9v"); Ok::<usize, ()>(3) }).unwrap();
    let mut output = [0; 3];
'''
write = 'proof.write(&mut output, |_, _, _, _, _| {}).unwrap();'
cases = [
    ("valid", write, None),
    ("mutate_input", 'input[0] = b\'!\';\n' + write, "E0506"),
    ("consume_twice", write + '\n' + write, "E0382"),
    ("substitute_input", 'proof.write(b"!!!!", &mut output, |_, _, _, _, _| {}).unwrap();', "E0061"),
    ("substitute_settings", 'proof.write(8_u8, &mut output, |_, _, _, _, _| {}).unwrap();', "E0061"),
    ("alter_settings", 'proof.configuration = 8;\n' + write, "E0616"),
]

with tempfile.TemporaryDirectory(prefix="base64-preflight-borrows-") as directory:
    for name, body, expected in cases:
        source = Path(directory) / f"{name}.rs"
        source.write_text(prefix + setup + body + '\n}\n', encoding="utf-8")
        result = subprocess.run(
            ["rustup", "run", sys.argv[1], "rustc", "--edition=2024", "--emit=metadata",
             "--out-dir", directory, "--error-format=json", str(source)],
            capture_output=True, text=True, check=False,
        )
        diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith("{")]
        codes = {message.get("code", {}).get("code") for message in diagnostics if message.get("code")}
        if (expected is None and result.returncode != 0) or (
            expected is not None and (result.returncode == 0 or expected not in codes)
        ):
            raise SystemExit(f"preflight borrow test {name} failed:\n{result.stderr}")
print(f"preflight borrows: source/configuration binding and single-use enforced on {sys.argv[1]}")
