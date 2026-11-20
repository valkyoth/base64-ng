"""Locate the single core library test executable in a fresh Cargo build."""
import json
import os
from pathlib import Path
import sys


def executable(messages, build):
    paths = []
    with messages.open(encoding="utf-8") as lines:
        for line in lines:
            item = json.loads(line)
            if (item["reason"] == "compiler-artifact"
                    and item["target"]["name"] == "base64_ng"
                    and item["target"]["kind"] == ["lib"]
                    and item["profile"]["test"] and item["executable"]):
                paths.append(Path(item["executable"]).resolve(strict=True))
    if (len(paths) != 1 or not paths[0].is_relative_to(build.resolve(strict=True))
            or not paths[0].is_file() or not os.access(paths[0], os.X_OK)):
        raise ValueError("expected one freshly built core test executable")
    return paths[0]


if __name__ == "__main__":
    print(executable(Path(sys.argv[1]), Path(sys.argv[2])))
