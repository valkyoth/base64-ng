#!/usr/bin/env python3
"""Verify scoped review packets without modifying or authorizing release evidence."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tarfile

from scoped_campaign import POLICY_ENV, ROOT, commit_id, load_policy, verify


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--historical-packet", required=True, type=Path)
    parser.add_argument("--recapture-packet", required=True, type=Path)
    parser.add_argument("--inspect-proposal", action="store_true",
                        help="verify pinned history and transports only; no approval or candidate claim")
    args = parser.parse_args()
    try:
        anchor = os.environ.get(POLICY_ENV)
        if args.inspect_proposal:
            if anchor is not None:
                raise ValueError("proposal inspection must not receive a reviewed policy anchor")
        else:
            if not anchor:
                raise ValueError(POLICY_ENV + " must come from independent review, never HEAD")
            commit_id(anchor)
        report = verify(ROOT, load_policy(ROOT), {
            "historical": args.historical_packet, "recapture": args.recapture_packet,
        }, anchor)
    except (ValueError, OSError, KeyError, TypeError, tarfile.TarError, subprocess.TimeoutExpired) as error:
        raise SystemExit("scoped campaign: " + str(error)) from error
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
