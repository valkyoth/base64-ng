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

    elf = "".join(f"_validate_16_bytes_neon_{name}:\n\tldr q1, [x0]\n\tcmeq v0.16b, v1.16b, v2.16b\n\tumaxv b0, v0.16b\n\tret\n.Lfunc_end{i}:\n"
                  for i, name in enumerate(("Standard", "UrlSafe")))
    verify(elf, "assembly", True)
    verify(elf.replace(".Lfunc", "Lfunc").replace("umaxv b0, v0.16b", "uminv.16b b0, v0").replace("cmeq v0.16b", "cmeq.16b v0"), "assembly", True)
    verify(re.sub(r"\.Lfunc_end\d+:", "\t.cfi_endproc", elf), "assembly", True)
    for mutation in ["", elf.replace("UrlSafe", "Standard"), elf.replace("umaxv", "umov"),
                     elf.replace("ldr q1", "ldr d1"), elf.replace("[x0]", "[x0, #16]")]:
        verify(mutation, "assembly", False)
    for instruction in ["str q0, [x1]", "stp q0, q1, [x1]", "bl helper", "b helper", "ldr q2, [x0]", "mov z0.b, #0"]:
        verify(elf.replace("\tret", f"\t{instruction}\n\tret"), "assembly", False)
    verify("define void @normal() {\nret void\n}", "production", True)
    for name in ("neon_candidate", "validation_candidate", "validate_16_bytes_neon"):
        verify(f"define void @{name}() {{}}", "production", False)
    verify("", "production", False)
    (root / "base64_ng-extra.s").write_text(elf)
    verify(elf, "assembly", False)
print("NEON validation codegen: mutation tests ok")
