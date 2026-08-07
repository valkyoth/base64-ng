#!/usr/bin/env sh
set -eu

root="$(mktemp -d "${TMPDIR:-/tmp}/base64-ng-evidence-outcomes.XXXXXX")"
cleanup() {
    rm -rf "$root"
}
trap cleanup EXIT INT TERM

mkdir -p \
    "$root/miri" \
    "$root/2.0-memory-sanitizers" \
    "$root/dudect" \
    "$root/backend" \
    "$root/fuzz" \
    "$root/kani/normal" \
    "$root/kani/advanced" \
    "$root/commit-53"

cat >"$root/miri/MANIFEST.txt" <<'EOF'
no_default_features=0
all_features=0
base64_ng_bytes=0
base64_ng_tokio_readers=0
base64_ng_tokio_writers=0
EOF
cat >"$root/2.0-memory-sanitizers/MANIFEST.txt" <<'EOF'
address_status=0
leak_status=0
thread_status=0
target=x86_64-unknown-linux-gnu
EOF
cat >"$root/dudect/MANIFEST.txt" <<'EOF'
samples=20000
iterations=64
warmup=1000
threshold=10
status=0
EOF
cat >"$root/backend/MANIFEST.txt" <<'EOF'
runtime_backend_report=0
simd_prototype_equivalence=0
EOF
printf '%s\n' PASS >"$root/kani/normal/status.txt"
printf '%s\n' PASS >"$root/kani/advanced/status.txt"
cat >"$root/commit-53/MANIFEST.txt" <<'EOF'
neon_automatic_dispatch=retained-native-performance
rvv=exact-linux-spacemit-x60-native-admission
EOF
python3 - "$root/riscv-native-admission" <<'PY'
import runpy
import sys
from pathlib import Path

fixture = runpy.run_path("scripts/test-rvv-admission-bundle.py")
fixture["write_bundle"](Path(sys.argv[1]))
PY
cat >"$root/fuzz/MANIFEST.txt" <<'EOF'
mode=release-duration
campaign_argument=-max_total_time=3600
EOF
for target in \
    decode in_place stream_chunks differential profiles x86_encode x86_decode \
    neon mime_body pem_document multibase_family imap_payload password_records \
    openpgp_armor v2_runtime_codec v2_incremental v2_async v2_assurance
do
    echo "$target=ok corpus=1 artifacts=0" >>"$root/fuzz/MANIFEST.txt"
done

scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null

mv "$root/riscv-native-admission" "$root/riscv-native-admission.good"
if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
    echo "release evidence outcome tests: accepted missing native RVV evidence" >&2
    exit 1
fi
mv "$root/riscv-native-admission.good" "$root/riscv-native-admission"

cp "$root/dudect/MANIFEST.txt" "$root/dudect/MANIFEST.good"
sed 's/^status=0$/status=1/' "$root/dudect/MANIFEST.good" \
    >"$root/dudect/MANIFEST.txt"
if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
    echo "release evidence outcome tests: accepted failed dudect evidence" >&2
    exit 1
fi
mv "$root/dudect/MANIFEST.good" "$root/dudect/MANIFEST.txt"

cp "$root/dudect/MANIFEST.txt" "$root/dudect/MANIFEST.good"
for replacement in missing duplicate nonnumeric weakened; do
    case "$replacement" in
        missing)
            sed '/^threshold=/d' "$root/dudect/MANIFEST.good" \
                >"$root/dudect/MANIFEST.txt"
            ;;
        duplicate)
            cp "$root/dudect/MANIFEST.good" "$root/dudect/MANIFEST.txt"
            echo 'threshold=10' >>"$root/dudect/MANIFEST.txt"
            ;;
        nonnumeric)
            sed 's/^threshold=10$/threshold=ten/' "$root/dudect/MANIFEST.good" \
                >"$root/dudect/MANIFEST.txt"
            ;;
        weakened)
            sed 's/^threshold=10$/threshold=1000000/' "$root/dudect/MANIFEST.good" \
                >"$root/dudect/MANIFEST.txt"
            ;;
    esac
    if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
        echo "release evidence outcome tests: accepted $replacement dudect threshold" >&2
        exit 1
    fi
done
mv "$root/dudect/MANIFEST.good" "$root/dudect/MANIFEST.txt"

cp "$root/2.0-memory-sanitizers/MANIFEST.txt" \
    "$root/2.0-memory-sanitizers/MANIFEST.good"
sed 's/^thread_status=0$/thread_status=1/' \
    "$root/2.0-memory-sanitizers/MANIFEST.good" \
    >"$root/2.0-memory-sanitizers/MANIFEST.txt"
if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
    echo "release evidence outcome tests: accepted failed sanitizer evidence" >&2
    exit 1
fi
mv "$root/2.0-memory-sanitizers/MANIFEST.good" \
    "$root/2.0-memory-sanitizers/MANIFEST.txt"

cp "$root/fuzz/MANIFEST.txt" "$root/fuzz/MANIFEST.good"
sed '/^v2_assurance=/d' "$root/fuzz/MANIFEST.good" >"$root/fuzz/MANIFEST.txt"
if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
    echo "release evidence outcome tests: accepted incomplete fuzz evidence" >&2
    exit 1
fi
cp "$root/fuzz/MANIFEST.good" "$root/fuzz/MANIFEST.txt"
echo 'v2_assurance=ok corpus=1 artifacts=0' >>"$root/fuzz/MANIFEST.txt"
if scripts/validate-release-evidence-outcomes.sh "$root" >/dev/null 2>&1; then
    echo "release evidence outcome tests: accepted duplicate fuzz evidence" >&2
    exit 1
fi

echo "release evidence outcome tests: ok"
