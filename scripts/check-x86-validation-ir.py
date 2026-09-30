"""Fail closed unless one readable production IR artifact excludes the candidate."""
from pathlib import Path
import re
import sys

try:
    files = list(Path(sys.argv[1]).glob("base64_ng-*.ll"))
    if len(files) != 1 or not files[0].is_file():
        raise SystemExit("x86 production IR: expected exactly one regular IR file")
    ir = files[0].read_text(encoding="utf-8")
except (OSError, UnicodeError) as error:
    raise SystemExit(f"x86 production IR: cannot read artifact: {error}") from error

definitions = re.findall(r"^define\s+[^\n]+", ir, re.MULTILINE)
if not definitions:
    raise SystemExit("x86 production IR: no function definitions found")
if any(re.search(r"validate_blocks_avx(?:2|512)|avx(?:2|512)_candidate|candidate_(validate|decode)_avx(?:2|512)",
                 definition) for definition in definitions):
    raise SystemExit("x86 production IR: candidate leaked into production IR")
print("x86 production IR: one readable artifact; candidate definitions absent")
