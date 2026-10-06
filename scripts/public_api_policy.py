"""Commit 25 policy matrix. Measurements inform review, never auto-admit an ISA."""

SURFACES = ("canonical", "historical", "validate", "owned", "historical-owned",
            "append", "in-place", "incremental", "sync", "bytes", "tokio")


def policy_cases(names, controls=False):
    if controls:
        if not {"canonical", "historical", "canonical-reference", "historical-reference"} <= set(names):
            raise ValueError("missing reference/automatic control operations")
        pairs = [(left + "->canonical", direction)
                 for left in ("scalar", "historical", "base64", "base64ct") if left in names
                 for direction in ("encode", "decode")]
        pairs += [(name + "-reference->" + name, "decode")
                  for name in ("canonical", "historical")]
        # These are existing exact public paths, not the test-only fast validator.
        pairs += [("avx2->avx512-vbmi", direction) for direction in ("encode", "decode")
                  if {"avx2", "avx512-vbmi"} <= set(names)]
    else:
        pairs = [(name, direction) for name in SURFACES if name in names
                 for direction in ("encode", "decode")
                 if name != "validate" or direction == "decode"]
    for name, direction in pairs:
        for profile in ("sp", "su", "up", "uu"):
            sizes = [32, 768, 65536, 1048576]
            if name in ("canonical", "historical"):
                sizes += [0, 3, 383, 384, 3071, 3072]
            for size in sizes:
                yield [name, direction, profile, str(size), "random", "4096", "warm"]
            if not controls and name in ("canonical", "historical"):
                for size in (32, 65536):
                    yield [name, direction, profile, str(size), "random", "4096", "cold"]
            if not controls and name in ("canonical", "historical", "validate") and direction == "decode":
                encoded = (65536 + 2) // 3 * 4 if profile.endswith("p") else (65536 * 8 + 5) // 6
                for position in (0, 64, encoded - 1):
                    yield [name, direction, profile, "65536", "random", "4096", f"invalid-{position}"]
            if name in ("incremental", "sync", "bytes", "tokio"):
                for fragment in (1, 7):
                    yield [name, direction, profile, "768", "structured", str(fragment), "warm"]


def iterations(case):
    if case[-1] == "cold":
        return 1
    # Enough work to avoid timer quantization; bound slow scalar and fragmented cells.
    return min(16384, max(8, 1048576 // max(int(case[3]), 64)))


def operation(case, side):
    names = case[0].split("->")
    return names[0] if len(names) == 1 or side == "baseline" else names[1]
