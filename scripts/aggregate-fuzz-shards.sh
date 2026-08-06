#!/usr/bin/env sh
set -eu

collection="${1:-target/fuzz-shards}"
python3 scripts/fuzz_shard_evidence.py aggregate "$collection"
