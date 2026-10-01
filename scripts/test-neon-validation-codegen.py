"""Fail-closed regression fixtures for the NEON validation codegen gate."""
import importlib.util
import re
from pathlib import Path
import tempfile

spec = importlib.util.spec_from_file_location("checker", Path(__file__).with_name("check-neon-validation-codegen.py"))
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    def verify(text, mode, accepted):
        path = root / ("base64_ng-test.ll" if mode == "production" else "base64_ng-test.s")
        path.write_text(text)
        try:
            checker.check(root, mode)
        except ValueError:
            assert not accepted, text
        else:
            assert accepted, text

    # Pinned compiler's complete lane classifier and invalid-mask Boolean return.
    body = """movi v0.16b, #223
ldr q1, [x0]
movi v2.16b, #165
movi v3.16b, #198
movi v4.16b, #230
mov w9, #1
movi v5.16b, #246
and v0.16b, v1.16b, v0.16b
add v0.16b, v0.16b, v2.16b
add v2.16b, v1.16b, v3.16b
movi v3.16b, #{special62}
cmhi v0.16b, v4.16b, v0.16b
cmhi v2.16b, v5.16b, v2.16b
movi v4.16b, #{special63}
cmeq v3.16b, v1.16b, v3.16b
and v0.16b, v2.16b, v0.16b
cmeq v1.16b, v1.16b, v4.16b
bic v0.16b, v0.16b, v3.16b
bic v0.16b, v0.16b, v1.16b
umaxv b0, v0.16b
fmov w8, s0
bic w0, w9, w8
ret
"""
    elf = "".join(f"_validate_16_bytes_neon_{name}:\n" +
                  "".join("\t" + line + "\n" for line in body.format(special62=a, special63=b).splitlines()) +
                  f".Lfunc_end{i}:\n"
                  for i, (name, a, b) in enumerate((("Standard", 43, 47), ("UrlSafe", 45, 95))))
    verify(elf, "assembly", True)
    macho = re.sub(r"\t(\w+) ([^\n]*\.16b[^\n]*)", lambda m: "\t" + m[1] + ".16b " + m[2].replace(".16b", ""), elf)
    verify(macho.replace(".Lfunc", "Lfunc"), "assembly", True)
    valid_mask = elf.replace("umaxv b0, v0.16b", "movi v6.16b, #255\n\teor v0.16b, v0.16b, v6.16b\n\tuminv b0, v0.16b").replace("bic w0", "and w0")
    verify(valid_mask, "assembly", True)
    # A max reduction of *valid* masks implements any-match, not all-match.
    verify(valid_mask.replace("uminv", "umaxv"), "assembly", False)
    verify(elf.replace("umaxv", "uminv"), "assembly", False)
    verify(re.sub(r"\.Lfunc_end\d+:", "\t.cfi_endproc", elf), "assembly", True)
    for mutation in ["", elf.replace("UrlSafe", "Standard"), elf.replace("umaxv", "umov"),
                     elf.replace("ldr q1", "ldr d1"), elf.replace("[x0]", "[x0, #16]")]:
        verify(mutation, "assembly", False)
    for instruction in ["str q0, [x1]", "stp q0, q1, [x1]", "bl helper", "b helper", "ldr q2, [x0]", "mov z0.b, #0"]:
        verify(elf.replace("\tret", f"\t{instruction}\n\tret"), "assembly", False)
    for mutation in [elf.replace("fmov w8, s0", "fmov w8, s1"),
                     elf.replace("fmov w8, s0", "mov w8, #0"),
                     elf.replace("bic w0", "and w0"),
                     elf.replace("bic w0, w9, w8", "mov w0, #1"),
                     elf.replace("bic w0, w9, w8", "fmov w0, s0"),
                     elf.replace("#43", "#45"),
                     elf.replace("#246", "#245"),
                     elf.replace("cmhi", "cmeq"),
                     elf.replace("and v0.16b, v1.16b, v0.16b", "and v0.16b, v7.16b, v0.16b"),
                     elf.replace("\tret", "\tret\n\tmov w0, #1"),
                     elf.replace("\tret", "\t.inst 0\n\tret"),
                     elf.replace("\tret", ""),
                     elf.replace("umaxv b0, v0.16b", "umaxv b0, v0.16b\n\tumaxv b0, v0.16b")]:
        verify(mutation, "assembly", False)
    verify("".join(f"_validate_16_bytes_neon_{name}:\n\tldr q1, [x0]\n\tcmeq v0.16b, v1.16b, v2.16b\n\tumaxv b0, v0.16b\n\tret\n.Lfunc_end{i}:\n"
                   for i, name in enumerate(("Standard", "UrlSafe"))), "assembly", False)
    verify("define void @normal() {\nret void\n}", "production", True)
    verify("define void @validate_16_bytes_neon() {}", "production", True)
    for name in ("neon_candidate", "validation_candidate"):
        verify(f"define void @{name}() {{}}", "production", False)
    verify("", "production", False)
    (root / "base64_ng-extra.s").write_text(elf)
    verify(elf, "assembly", False)
print("NEON validation codegen: mutation tests ok")
