"""Mutation coverage for the narrowly scoped validation assembly check."""
from pathlib import Path
import subprocess
import sys
import tempfile

checker = Path(__file__).with_name("check-x86-validation-asm.py")
body = "\tmovdqu (%rdi), %xmm0\n\tpcmpeqb %xmm1, %xmm0\n\tpmovmskb %xmm0, %eax\n\tretq\n"
valid = "".join(
    f"validate_16_bytes_ssse3_sse41_{name}:\n{body}.Lfunc_end{index}:\n"
    for index, name in enumerate(("Standard", "UrlSafe"))
)
cases = [
    (valid, True),
    (valid.replace(".Lfunc_end", "Lfunc_end"), True),
    (valid.replace("pmovmskb", "movl"), False),
    (valid.replace("pcmpeqb", "por"), False),
    (valid.replace("movdqu", "movq"), False),
    (valid.replace("\tretq", "\tmovdqu %xmm0, (%rsi)\n\tretq"), False),
    (valid.replace("\tretq", "\tcallq hidden_decoder\n\tretq"), False),
    (valid.replace("%xmm0", "%ymm0"), False),
    (valid.replace("UrlSafe", "Standard"), False),
    (valid.split("validate_16_bytes_ssse3_sse41_UrlSafe")[0], False),
]
with tempfile.TemporaryDirectory() as root:
    path = Path(root) / "base64_ng-fixture.s"
    for text, accepted in cases:
        path.write_text(text)
        result = subprocess.run([sys.executable, str(checker), root], capture_output=True)
        if (result.returncode == 0) != accepted:
            raise SystemExit(f"SSSE3 assembly mutation accepted={accepted}: {result.stderr!r}")
        avx = text.replace("validate_16_bytes_ssse3_sse41", "validate_blocks_avx2")
        avx = avx.replace("%ymm", "%zmm").replace("%xmm", "%ymm")
        avx = avx.replace("movdqu", "vmovdqu").replace("pcmpeqb", "vpcmpeqb").replace("pmovmskb", "vpmovmskb")
        path.write_text(avx)
        result = subprocess.run([sys.executable, str(checker), root, "avx2"], capture_output=True)
        if (result.returncode == 0) != accepted:
            raise SystemExit(f"AVX2 assembly mutation accepted={accepted}: {result.stderr!r}")
    avx = valid.replace("validate_16_bytes_ssse3_sse41", "validate_blocks_avx2").replace("pmovmskb", "vpmovmskb")
    path.write_text(avx)
    if subprocess.run([sys.executable, str(checker), root, "avx2"], capture_output=True).returncode == 0:
        raise SystemExit("AVX2 assembly check accepted a half-width reduction")
    (Path(root) / "base64_ng-stale.s").write_text(valid)
    if subprocess.run([sys.executable, str(checker), root], capture_output=True).returncode == 0:
        raise SystemExit("SSSE3 assembly check accepted ambiguous files")
print("x86 validation assembly mutations: valid bodies accepted; omissions/stores/calls and half-width AVX2 rejected")
