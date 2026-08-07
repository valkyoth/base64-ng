#!/usr/bin/env sh
set -eu

collection="${1:-target/fuzz-shards}"
if [ -n "${BASE64_NG_FUZZ_SOURCE_COMMIT:-}" ]; then
    python3 scripts/fuzz_shard_evidence.py aggregate "$collection" \
        --source-commit "$BASE64_NG_FUZZ_SOURCE_COMMIT"
else
    python3 scripts/fuzz_shard_evidence.py aggregate "$collection"
fi
