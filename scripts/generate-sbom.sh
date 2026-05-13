#!/usr/bin/env sh
set -eu

output_dir="${BASE64_NG_SBOM_DIR:-target/release-evidence}"
mkdir -p "$output_dir"

if ! cargo sbom --version >/dev/null 2>&1; then
    echo "cargo-sbom is required; install with: cargo install --locked cargo-sbom --version 0.10.0" >&2
    exit 1
fi

spdx_output="$output_dir/base64-ng.spdx.json"
cyclonedx_output="$output_dir/base64-ng.cyclonedx.json"

cargo sbom --output-format spdx_json_2_3 > "$spdx_output"
cargo sbom --output-format cyclone_dx_json_1_4 > "$cyclonedx_output"

test -s "$spdx_output"
test -s "$cyclonedx_output"
grep -q '"spdxVersion"[[:space:]]*:[[:space:]]*"SPDX-2.3"' "$spdx_output"
grep -q '"bomFormat"[[:space:]]*:[[:space:]]*"CycloneDX"' "$cyclonedx_output"

sha256sum "$spdx_output" "$cyclonedx_output"
