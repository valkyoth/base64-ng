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
body512 = "\tvmovdqu64 (%rdi), %zmm0\n\tvpcmpltub %zmm1, %zmm0, %k1\n\tvpcmpneqb %zmm2, %zmm0, %k0\n\tktestq %k1, %k0\n\tretq\n"
valid512 = "".join(
    f"validate_blocks_avx512_{name}:\n{body512}.Lfunc_end{index}:\n"
    for index, name in enumerate(("Standard", "UrlSafe"))
)
cases512 = [
    (valid512, True),
    (valid512.replace(".Lfunc_end", "Lfunc_end"), True),
    (valid512.replace("ktestq", "ktestd"), False),
    (valid512.replace("ktestq", "ktestw"), False),
    (valid512.replace("vmovdqu64", "vmovdqa64"), False),
    (valid512.replace("vpcmpltub", "vpaddb"), False),
    (valid512.replace("vpcmpneqb", "vpaddb"), False),
    (valid512.replace("%zmm", "%ymm"), False),
    (valid512.replace("\tretq", "\tcallq hidden_classifier\n\tretq"), False),
    (valid512.replace("\tretq", "\tvmovdqu8 %zmm0, (%rsi) {%k1}\n\tretq"), False),
    (valid512.replace("\tretq", "\tvmovdqu64 %zmm0, (%rsi)\n\tretq"), False),
    (valid512.replace("UrlSafe", "Standard"), False),
    (valid512.split("validate_blocks_avx512_UrlSafe")[0], False),
]
with tempfile.TemporaryDirectory() as root:
    path = Path(root) / "base64_ng-fixture.s"
    for text, accepted in cases512:
        path.write_text(text)
        result = subprocess.run([sys.executable, str(checker), root, "avx512"], capture_output=True)
        if (result.returncode == 0) != accepted:
            raise SystemExit(f"AVX-512 assembly mutation accepted={accepted}: {result.stderr!r}")
    path.unlink()
    if subprocess.run([sys.executable, str(checker), root, "avx512"], capture_output=True).returncode == 0:
        raise SystemExit("AVX-512 assembly check accepted a missing artifact")
    path.write_text(valid512)
    (Path(root) / "base64_ng-stale.s").write_text(valid512)
    if subprocess.run([sys.executable, str(checker), root, "avx512"], capture_output=True).returncode == 0:
        raise SystemExit("AVX-512 assembly check accepted ambiguous files")
print("x86 validation assembly mutations: valid bodies accepted; omissions/stores/calls and partial-width reductions rejected")

# LLVM 23's promoted SSSE3 argument is loaded by the direct caller.
promoted = valid.replace("\tmovdqu (%rdi), %xmm0\n", "").replace("pcmpeqb %xmm1, %xmm0", "pcmpeqb %xmm0, %xmm1")
promoted += "".join(f"\tmovups (%r12,%rbx), %xmm0\n\tcallq validate_16_bytes_ssse3_sse41_{name}\n"
                    for name in ("Standard", "UrlSafe"))
with tempfile.TemporaryDirectory() as root:
    path = Path(root) / "base64_ng-fixture.s"
    for text, accepted in [
        (promoted, True),
        (promoted.replace("movups", "movq"), False),
        (promoted.replace("movups", "vmovups").replace("%xmm0\n\tcallq", "%ymm0\n\tcallq"), False),
        (promoted.replace("%xmm0\n\tcallq", "%xmm1\n\tcallq"), False),
        (promoted.replace("\tcallq", "\tpxor %xmm0, %xmm0\n\tcallq"), False),
        (promoted.replace("\tpcmpeqb", "\tpxor %xmm0, %xmm0\n\tpcmpeqb"), False),
        (promoted.replace("\tretq", "\tmovq (%rdi), %rax\n\tretq"), False),
        (promoted.rsplit("\tmovups", 1)[0], False),
    ]:
        path.write_text(text)
        result = subprocess.run([sys.executable, str(checker), root], capture_output=True)
        if (result.returncode == 0) != accepted:
            raise SystemExit(f"SSSE3 promoted argument mutation accepted={accepted}: {result.stderr!r}")
print("SSSE3 promoted argument mutations: exact-width caller witnesses required")
