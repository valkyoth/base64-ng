#!/usr/bin/env python3
"""Inspect the actual production assembler output, without test cfg."""
import sys
from pathlib import Path
from rvv_validation_codegen import check


if __name__ == "__main__":
    try:
        files = list(Path(sys.argv[1]).glob("base64_ng-*.s"))
        if len(files) != 1:
            raise ValueError("expected one fresh production assembly file")
        check(files[0].read_text())
    except (ValueError, IndexError) as error:
        sys.exit(f"RVV validation codegen: {error}")
    print("RVV validation codegen: complete production leaves match reviewed contract")
