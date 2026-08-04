#!/usr/bin/env sh
set -eu

plan="2.0.0-release-plan.md"
decision="docs/2.0_GOVERNANCE.md"

require_text() {
    file="$1"
    expected="$2"
    if ! grep -F -q -- "$expected" "$file"; then
        echo "2.0 governance: $file is missing required text: $expected" >&2
        exit 1
    fi
}

test -s "$plan"
test -s "$decision"

require_text "$plan" "This is the authoritative executable roadmap for 2.0."
require_text "$plan" "[2.0 governance decision](docs/2.0_GOVERNANCE.md)"
require_text "$plan" "Nothing listed here is deferred beyond 2.0.0."
require_text "$plan" 'Planning baseline: signed `v1.3.9` at commit `5af9802`.'
require_text "$plan" 'Active implementation and release toolchain at plan creation: Rust `1.97.1`.'
require_text "$plan" 'Minimum supported Rust version: Rust `1.90.0`'
require_text "$plan" 'Kani evidence baseline: `cargo-kani 0.67.0` with the documented Rust `1.90.0`'
require_text "$plan" '[base64 0.23.0][base64-0.23.0]'
require_text "$plan" '[base64ct 1.8.3][base64ct-1.8.3]'
require_text "$plan" 'the final signed `v2.0.0` tag'
require_text "$plan" 'Architecture reviews and gap analyses are source material, not'
require_text "$plan" 'Strict RFC 4648 encoding and decoding remains the ordinary default'
require_text "$plan" "External pentests may cover one checkpoint or"
require_text "$plan" "Intermediate batches do not create permanent GitHub pentest reports."
require_text "$plan" "Locally verified checkpoints may remain pentest-pending"
require_text "$plan" "## Checkpoint Record"
require_text "$plan" "Accepted pre-seal evidence amendment: the second, preferably Intel, AVX-512"

commit_count="$(grep -E -c '^### Commit [0-9]+:' "$plan")"
goal_count="$(grep -F -c '**Goal**' "$plan")"
deliverable_count="$(grep -F -c '**Deliverables**' "$plan")"
verification_count="$(grep -F -c '**Verification**' "$plan")"
exit_count="$(grep -F -c '**Exit criteria**' "$plan")"
checkpoint_count="$(grep -E -c '^\| [0-9]+ \|' "$plan")"

for value in \
    "$commit_count" \
    "$goal_count" \
    "$deliverable_count" \
    "$verification_count" \
    "$exit_count" \
    "$checkpoint_count"
do
    if [ "$value" -ne 55 ]; then
        echo "2.0 governance: expected 55 complete checkpoints, observed count $value" >&2
        exit 1
    fi
done

require_text "$decision" "Status: Accepted for implementation checkpoints"
require_text "$decision" 'Planning baseline: signed `v1.3.9`'
require_text "$decision" '`5af9802e0fd8fe25a9b50481715e6dbc4a9b87ad`'
require_text "$decision" "Everything listed in the authoritative plan is required for 2.0.0."
require_text "$decision" "Architecture reviews and gap analyses are source material."
require_text "$decision" 'Rust `1.97.1`'
require_text "$decision" 'Rust `1.90.0`'
require_text "$decision" '`cargo-kani 0.67.0`'
require_text "$decision" '`base64` `0.23.0` and `base64ct` `1.8.3`'
require_text "$decision" "pentests may cover one checkpoint or a contiguous batch of checkpoints at"
require_text "$decision" "A checkpoint is development-complete when all of the following are true:"
require_text "$decision" "One report may satisfy multiple contiguous checkpoints"
require_text "$decision" 'Root `PENTEST.md` remains temporary scratch input'
require_text "$decision" "Intermediate pentest batches are working review gates and do not"
require_text "$decision" "QEMU, compiler/codegen, runtime smoke, and"
require_text "$decision" 'The 1.x `STANDARD`, `STANDARD_NO_PAD`, `URL_SAFE`, and `URL_SAFE_NO_PAD`'
require_text "$decision" "obtain the exact unmodified RFC 4648 bytes"
require_text "$decision" "exclude the RFC text and source-lock material from every crates.io and npm"
require_text "$decision" "persistent teardown recovery without a separately named and admitted"
require_text "$decision" "The accepted pre-seal evidence amendment classifies a second AVX-512 VBMI"

require_text README.md "[2.0 commit plan](2.0.0-release-plan.md)"
require_text README.md "[governance decision](docs/2.0_GOVERNANCE.md)"
require_text CONTRIBUTING.md '[`2.0.0-release-plan.md`](2.0.0-release-plan.md)'
require_text docs/RELEASE_EVIDENCE.md '[`2.0_GOVERNANCE.md`](2.0_GOVERNANCE.md)'

echo "2.0 governance: ok"
