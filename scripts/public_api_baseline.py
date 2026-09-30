"""Schema and conservative, exploratory paired performance comparisons."""

import math
import statistics


FIELDS = {"elapsed_ns", "iterations", "raw_bytes", "encoded_bytes", "allocations",
          "encode_capability", "decode_capability"}


def parse_sample(row, iterations, raw_bytes, encoded_bytes):
    if set(row) != FIELDS:
        raise ValueError("unexpected measurement schema")
    result = dict(row)
    for name in FIELDS - {"encode_capability", "decode_capability"}:
        result[name] = int(row[name])
        if result[name] < 0:
            raise ValueError("negative measurement")
    if result["elapsed_ns"] <= 0 or result["iterations"] != iterations:
        raise ValueError("invalid duration or iteration count")
    if result["raw_bytes"] != raw_bytes or result["encoded_bytes"] != encoded_bytes:
        raise ValueError("inconsistent byte accounting")
    return result


def classify(baseline, candidate):
    if len(baseline) != len(candidate) or not baseline:
        raise ValueError("missing or unpaired samples")
    if any(not math.isfinite(x) or x <= 0 for x in baseline + candidate):
        raise ValueError("invalid latency")
    ratio = statistics.median([a / b for a, b in zip(baseline, candidate)])
    if len(baseline) < 7:
        return ratio, "insufficient-samples"
    for samples in (baseline, candidate):
        median = statistics.median(samples)
        mad = statistics.median(abs(x - median) for x in samples)
        if mad / median > 0.10:
            return ratio, "noisy"
    if 0.95 <= ratio <= 1.05:
        return ratio, "within-5-percent"
    wins = sum(a > b for a, b in zip(baseline, candidate))
    # Ties must not count as evidence for either direction.
    count = wins if ratio > 1 else sum(a < b for a, b in zip(baseline, candidate))
    p = sum(math.comb(len(baseline), k) for k in range(count, len(baseline) + 1)) / 2 ** len(baseline)
    if p > 0.05:
        return ratio, "inconclusive"
    return ratio, "improvement-signal" if ratio > 1 else "regression-signal"


def summarize(rows):
    result = []
    groups = {}
    for row in rows:
        key = (row["features"], *row["case"])
        group = groups.setdefault(key, {"baseline": {}, "candidate": {}})
        if row["side"] not in group or row["sample"] in group[row["side"]]:
            raise ValueError("invalid side or duplicate sample")
        group[row["side"]][row["sample"]] = row
    if not groups:
        raise ValueError("no measurements")
    for key, pair in groups.items():
        left, right = pair["baseline"], pair["candidate"]
        if set(left) != set(right) or sorted(left) != list(range(len(left))):
            raise ValueError("unpaired or missing sample indices")
        a = [left[i]["elapsed_ns"] / left[i]["iterations"] for i in sorted(left)]
        b = [right[i]["elapsed_ns"] / right[i]["iterations"] for i in sorted(right)]
        ratio, status = classify(a, b)
        sample = left[0]
        valid = not sample["case"][-1].startswith("invalid-")
        seconds = statistics.median(b) / 1e9
        result.append(dict(
            features=key[0], case=list(key[1:]), samples=len(a), ratio=ratio, status=status,
            baseline_ns=statistics.median(a), candidate_ns=statistics.median(b),
            candidate_allocations=statistics.median(right[i]["allocations"] / right[i]["iterations"] for i in right),
            baseline_allocations=statistics.median(left[i]["allocations"] / left[i]["iterations"] for i in left),
            # Invalid input is latency-only, not successful throughput.
            payload_gib_s=sample["raw_bytes"] / seconds / 2**30 if valid else None,
            encoded_gib_s=sample["encoded_bytes"] / seconds / 2**30 if valid else None,
        ))
    return result
