"""Inspect only the two validation-only monomorphizations, not decode assembly."""
from pathlib import Path
import re
import sys

files = list(Path(sys.argv[1]).glob("base64_ng-*.s"))
if len(files) != 1:
    raise SystemExit("SSSE3 validation assembly: expected one fresh assembly file")
assembly = files[0].read_text()
bodies = re.findall(
    r"^([^\s:]*validate_16_bytes_ssse3_sse41[^\s:]*):\n(.*?)^\.?Lfunc_end\d+:",
    assembly,
    re.MULTILINE | re.DOTALL,
)
if len(bodies) != 2 or not all(any(name in symbol for symbol, _ in bodies)
                               for name in ("Standard", "UrlSafe")):
    raise SystemExit("SSSE3 validation assembly: both alphabet bodies are required")
for symbol, body in bodies:
    for pattern in (r"\b[v]?movdqu\b", r"\b[v]?pcmpeqb\b", r"\b[v]?pmovmskb\b"):
        if not re.search(pattern, body):
            raise SystemExit(f"SSSE3 validation assembly: missing {pattern} in {symbol}")
    # AT&T destinations are last: a memory destination would contradict the
    # validation-only contract. Calls could hide stores or scalar delegation.
    if re.search(r",\s*[^,\n]*\([^\n]*\)\s*(?:#.*)?$", body, re.MULTILINE):
        raise SystemExit("SSSE3 validation assembly: unexpected memory destination")
    if re.search(r"\bcall[qwl]?\b|%[yz]mm", body):
        raise SystemExit("SSSE3 validation assembly: unexpected call or wider vector ISA")
print("SSSE3 validation assembly: two vector classifiers, all-lane reduction, no stores/calls")
