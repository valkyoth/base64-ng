#!/usr/bin/env python3
"""Shared non-parametric checks for automatic-backend performance evidence."""

import math
import statistics


MINIMUM_SAMPLES = 15
MAXIMUM_ONE_SIDED_SIGN_P = 0.05


def validate_advantage(
    candidate: list[float],
    fallback: list[float],
    minimum_ratio: float,
    label: object,
) -> None:
    if len(candidate) < MINIMUM_SAMPLES or len(fallback) < MINIMUM_SAMPLES:
        raise ValueError(f"missing {MINIMUM_SAMPLES} samples for {label}")
    if len(candidate) != len(fallback):
        raise ValueError(f"unpaired sample counts for {label}")
    if any(not math.isfinite(value) or value <= 0.0 for value in candidate + fallback):
        raise ValueError(f"invalid throughput for {label}")

    ratio = statistics.median(candidate) / statistics.median(fallback)
    if ratio < minimum_ratio:
        raise ValueError(f"{label} ratio {ratio:.3f} is below {minimum_ratio:.3f}")

    # Equal lengths are checked above; avoid Python 3.10-only zip(strict=...) so
    # native evidence capture also works with the Python 3.9 shipped by macOS.
    wins = sum(left > right for left, right in zip(candidate, fallback))
    tied_or_worse = len(candidate) - wins
    one_sided_p = sum(
        math.comb(len(candidate), count)
        for count in range(wins, len(candidate) + 1)
    ) / (2 ** len(candidate))
    if one_sided_p > MAXIMUM_ONE_SIDED_SIGN_P:
        raise ValueError(
            f"{label} has {wins} wins and {tied_or_worse} ties/losses; "
            f"one-sided sign-test p={one_sided_p:.4f} exceeds "
            f"{MAXIMUM_ONE_SIDED_SIGN_P:.2f}"
        )
