"""Inspect exact validation-only symbols, or prove their production exclusion."""
from pathlib import Path
import re
import sys


def check(directory, mode):
    suffix = "ll" if mode == "production" else "s"
    files = list(Path(directory).glob(f"base64_ng-*.{suffix}"))
    if len(files) != 1 or not files[0].is_file():
        raise ValueError("expected exactly one fresh codegen file")
    text = files[0].read_text(encoding="utf-8")
    if mode == "production":
        definitions = re.findall(r"^define\s+[^\n]+", text, re.MULTILINE)
        if not definitions:
            raise ValueError("missing production function definitions")
        if any(re.search(r"neon_candidate|validation_candidate", row)
               for row in definitions):
            raise ValueError("asserting NEON candidate leaked into production")
        return
    if mode != "assembly":
        raise ValueError("expected assembly or production")
    bodies = re.findall(
        r"^([^\s:]*validate_16_bytes_neon[^\s:]*):\s*\n(.*?)(?:^\.?Lfunc_end\d+:|^\s*\.cfi_endproc\b)",
        text, re.MULTILINE | re.DOTALL,
    )
    if len(bodies) != 2 or not all(any(name in symbol for symbol, _ in bodies)
                                 for name in ("Standard", "UrlSafe")):
        raise ValueError("both alphabet classifiers are required")
    for _, body in bodies:
        # ELF and Mach-O spelling; LLVM may invert validity before reducing.
        loads = re.findall(r"\bldr\s+q\d+,\s*\[x0\]", body)
        if len(loads) != 1 or len(re.findall(r"\b(?:ldr|ldur|ldp|ld1)\b", body)) != 1:
            raise ValueError("expected one exact 16-byte input load")
        if not re.search(r"\b(?:uminv|umaxv)(?:\.16b)?\s+b\d+,\s*v\d+(?:\.16b)?", body):
            raise ValueError("missing all-lane reduction")
        if not re.search(r"\bcmeq(?:\.16b)?\s+v\d+", body):
            raise ValueError("missing vector classification")
        if re.search(r"\b(?:st\w*|bl|blr|br)\b|\bb(?:\.[a-z]+)?\s|\b[cz]bnz\b|\bcbz\b|\btbn?z\b", body):
            raise ValueError("unexpected store, call, or branch")
        if re.search(r"\b[zp]\d+\b", body):
            raise ValueError("unexpected SVE ISA")


if __name__ == "__main__":
    try:
        check(sys.argv[1], sys.argv[2])
    except (ValueError, OSError, UnicodeError) as error:
        raise SystemExit(f"NEON validation codegen: {error}") from error
    print(f"NEON validation codegen: {sys.argv[2]} ok")
