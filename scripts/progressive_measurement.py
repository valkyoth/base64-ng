"""Parse retained measurements of the retired Commit 21 progressive experiment."""
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parents[1]
SIZES = (4096, 16384, 65536, 1048576)


def parse(text):
    samples, backends = {}, {}
    for line in text.splitlines():
        if line.startswith("progressive-backend,"):
            _, profile, size, backend = line.split(",")
            key = (int(profile), int(size))
            if key in backends or backend not in {"avx2", "ssse3-sse4.1"}:
                raise ValueError("duplicate or unexpected native backend")
            backends[key] = backend
        elif line.startswith("progressive-bench,"):
            _, profile, size, sample, mode, rounds, nanos = line.split(",")
            key = tuple(map(int, (profile, size, sample, mode)))
            if key in samples or int(rounds) != max(4, 4194304 // int(size)) or int(nanos) <= 0:
                raise ValueError("duplicate or invalid timing sample")
            samples[key] = int(nanos)
    expected = {(p, n, s, m) for p in range(4) for n in SIZES for s in range(11) for m in range(3)}
    if set(samples) != expected or set(backends) != {(p, n) for p in range(4) for n in SIZES}:
        raise ValueError("incomplete paired samples or backend observations")
    rows = []
    for profile in range(4):
        for size in SIZES:
            paired = [samples[profile, size, s, 0] / samples[profile, size, s, 1] for s in range(11)]
            rows.append({"profile": profile, "decoded_bytes": size,
                         "backend": backends[profile, size],
                         "transactional_over_progressive": paired,
                         "median_speedup": statistics.median(paired),
                         "median_ns_per_call": {
                             mode: statistics.median(samples[profile, size, s, m] for s in range(11))
                                   / max(4, 4194304 // size)
                             for m, mode in enumerate(("transactional", "progressive", "incremental"))}})
    return rows
