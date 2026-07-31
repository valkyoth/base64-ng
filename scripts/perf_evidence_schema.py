"""Shared schema constants for performance evidence tooling."""

from __future__ import annotations

import re

BENCHMARK_FIELDS = [
    "schema_version",
    "campaign_id",
    "run_id",
    "sample_index",
    "engine",
    "operation",
    "alphabet",
    "padding",
    "input_len",
    "encoded_len",
    "iterations",
    "elapsed_ns",
    "throughput_mib_s",
    "backend",
    "active_encode_backend",
    "active_decode_backend",
    "target_arch",
    "target_os",
    "allocation_count",
]
AVAILABILITY_FIELDS = [
    "schema_version",
    "backend",
    "available",
    "target_arch",
    "target_os",
]
RESOURCE_FIELDS = [
    "schema_version",
    "category",
    "name",
    "feature_set",
    "value",
    "unit",
    "method",
]
PROFILES = {
    ("standard", "padded"),
    ("standard", "unpadded"),
    ("url-safe", "padded"),
    ("url-safe", "unpadded"),
}
OPERATIONS = {"encode", "decode"}
ENGINES = {"base64-ng", "base64-0.23.0", "base64ct-1.8.3"}
BACKEND_MINIMUM = {
    "auto": {"encode": 1, "decode": 1},
    "ssse3-sse4.1": {"encode": 12, "decode": 12},
    "avx2": {"encode": 24, "decode": 24},
    "avx512-vbmi": {"encode": 48, "decode": 48},
    "neon": {"encode": 12, "decode": 12},
    "wasm-simd128": {"encode": 12, "decode": 12},
}
BACKENDS = set(BACKEND_MINIMUM) | {"scalar"}
EXPECTED_LENGTHS = {
    "1",
    "2",
    "3",
    "11",
    "12",
    "15",
    "16",
    "23",
    "24",
    "31",
    "32",
    "47",
    "48",
    "63",
    "64",
    "1024",
    "65536",
}
EVIDENCE_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}\Z")
COMMIT_ID = re.compile(r"[0-9a-f]{40}\Z")
