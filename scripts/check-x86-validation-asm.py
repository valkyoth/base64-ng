"""Inspect the two validation-only monomorphizations, not decode assembly."""
from pathlib import Path
import re
import sys

isa = sys.argv[2] if len(sys.argv) > 2 else "ssse3"
if isa not in ("ssse3", "avx2", "avx512"):
    raise SystemExit("validation assembly: expected ssse3, avx2 or avx512")
symbol_name = "validate_16_bytes_ssse3_sse41" if isa == "ssse3" else f"validate_blocks_{isa}"

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
    patterns = (r"\b[v]?movdqu\b", r"\b[v]?pcmpeqb\b", r"\b[v]?pmovmskb\b")
    if isa == "ssse3" and not re.search(patterns[0], body):
        # LLVM 23 promotes the reference argument to xmm0 and lifts its load
        # into callers. Require a direct, exact-width load/call witness for
        # each alphabet; constants are the only memory reads in the callee.
        # This is a code-shape check, supplemented by native guard-page tests.
        witness = rf"\bmovups\s+\([^\n]+\),\s*%xmm0\n\s*callq\s+{re.escape(symbol)}(?:\s|$)"
        if not re.search(witness, assembly):
            raise SystemExit("validation assembly: missing promoted 16-byte input load/call")
        if any("(%rip)" not in line for line in body.splitlines() if "(" in line):
            raise SystemExit("validation assembly: unexpected promoted classifier memory read")
        first_use = next((line for line in body.splitlines() if "%xmm0" in line), "")
        if not re.search(r"\b(?:pand|pcmpeqb)\s+%xmm0,\s*%xmm[1-9]\d?", first_use):
            raise SystemExit("validation assembly: promoted argument is not consumed")
        patterns = patterns[1:]
    if isa == "avx512":
        # Pinned LLVM folds the range masks into unsigned compares and ktestq.
        patterns = (r"\bvmovdqu(?:8|64)\s+[^\n]*,\s*%zmm", r"\bvpcmpltub\b[^\n]*%zmm",
                    r"\bvpcmpneqb\b[^\n]*%zmm", r"\bktestq\s+%k\d,\s*%k\d")
    for pattern in patterns:
        if not re.search(pattern, body):
            raise SystemExit(f"validation assembly: missing {pattern} in {symbol}")
    if isa == "avx2" and not re.search(r"\bvpmovmskb\s+%ymm", body):
        raise SystemExit("validation assembly: missing cross-lane AVX2 reduction")
    # AT&T destinations are last: a memory destination would contradict the
    # validation-only contract. Calls could hide stores or scalar delegation.
    if re.search(r",\s*[^,\n]*\([^\n]*\)\s*(?:\{[^}\n]+\}\s*)*(?:#.*)?$", body, re.MULTILINE):
        raise SystemExit("validation assembly: unexpected memory destination")
    wider = r"%[yz]mm" if isa == "ssse3" else r"%zmm" if isa == "avx2" else r"(?!)"
    if re.search(r"\bcall[qwl]?\b|" + wider, body):
        raise SystemExit("validation assembly: unexpected call or wider vector ISA")
print(f"{isa} validation assembly: two vector classifiers, all-lane reduction, no stores/calls")
