"""Inspect the two validation-only monomorphizations, not decode assembly."""
from pathlib import Path
import re
import sys

isa = sys.argv[2] if len(sys.argv) > 2 else "ssse3"
if isa not in ("ssse3", "avx2"):
    raise SystemExit("validation assembly: expected ssse3 or avx2")
symbol_name = "validate_16_bytes_ssse3_sse41" if isa == "ssse3" else "validate_blocks_avx2"

files = list(Path(sys.argv[1]).glob("base64_ng-*.s"))
if len(files) != 1:
    raise SystemExit("validation assembly: expected one fresh assembly file")
assembly = files[0].read_text()
bodies = re.findall(
    rf"^([^\s:]*{symbol_name}[^\s:]*):\n(.*?)^\.?Lfunc_end\d+:",
    assembly,
    re.MULTILINE | re.DOTALL,
)
if len(bodies) != 2 or not all(any(name in symbol for symbol, _ in bodies)
                               for name in ("Standard", "UrlSafe")):
    raise SystemExit("validation assembly: both alphabet bodies are required")
for symbol, body in bodies:
    for pattern in (r"\b[v]?movdqu\b", r"\b[v]?pcmpeqb\b", r"\b[v]?pmovmskb\b"):
        if not re.search(pattern, body):
            raise SystemExit(f"validation assembly: missing {pattern} in {symbol}")
    if isa == "avx2" and not re.search(r"\bvpmovmskb\s+%ymm", body):
        raise SystemExit("validation assembly: missing cross-lane AVX2 reduction")
    # AT&T destinations are last: a memory destination would contradict the
    # validation-only contract. Calls could hide stores or scalar delegation.
    if re.search(r",\s*[^,\n]*\([^\n]*\)\s*(?:#.*)?$", body, re.MULTILINE):
        raise SystemExit("validation assembly: unexpected memory destination")
    wider = r"%[yz]mm" if isa == "ssse3" else r"%zmm"
    if re.search(r"\bcall[qwl]?\b|" + wider, body):
        raise SystemExit("validation assembly: unexpected call or wider vector ISA")
print(f"{isa} validation assembly: two vector classifiers, all-lane reduction, no stores/calls")
