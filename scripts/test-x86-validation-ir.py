"""Regression coverage for missing artifacts and errors previously hidden by grep."""
from pathlib import Path
import subprocess
import sys
import tempfile

checker = Path(__file__).with_name("check-x86-validation-ir.py")
valid = "define void @ordinary_decode() {\n  ret void\n}\n"


def check(directory, accepted, diagnostic):
    result = subprocess.run(
        [sys.executable, str(checker), str(directory)], capture_output=True, text=True
    )
    if (result.returncode == 0) != accepted or diagnostic not in result.stdout + result.stderr:
        raise AssertionError(f"accepted={accepted}: {result.stdout}{result.stderr}")


with tempfile.TemporaryDirectory(prefix="base64-ng-avx2-ir-") as temporary:
    root = Path(temporary)
    path = root / "base64_ng-fixture.ll"
    check(root / "missing", False, "exactly one")
    check(root, False, "exactly one")
    (root / "unexpected-name.ll").write_text(valid)
    check(root, False, "exactly one")
    path.mkdir()
    check(root, False, "regular IR file")
    path.rmdir()
    path.symlink_to(root / "missing.ll")
    check(root, False, "regular IR file")
    path.unlink()
    for text in ("", "; LLVM IR comment only\n", "declare void @ordinary_decode()\n"):
        path.write_text(text)
        check(root, False, "no function definitions")
    path.write_bytes(b"\xff")
    check(root, False, "cannot read artifact")
    path.write_text(valid)
    check(root, True, "candidate definitions absent")
    path.chmod(0)
    try:
        # Root can read mode-000 files; exercise this case when permissions apply.
        try:
            path.read_bytes()
        except PermissionError:
            check(root, False, "cannot read artifact")
    finally:
        path.chmod(0o600)
    path.write_text(valid + valid.replace("ordinary_decode", "validate_blocks_avx2"))
    check(root, True, "candidate definitions absent")
    for symbol in ("ssse3_candidate", "candidate_validate_16", "candidate_decode_16", "avx2_candidate",
                   "candidate_validate_avx2", "candidate_decode_avx2",
                   "validate_blocks_avx512", "avx512_candidate",
                   "candidate_validate_avx512", "candidate_decode_avx512"):
        path.write_text(valid + valid.replace("ordinary_decode", symbol))
        check(root, False, "candidate leaked")
    path.write_text(valid)
    (root / "base64_ng-stale.ll").write_text(valid)
    check(root, False, "exactly one")

print("x86 production IR mutations: valid artifact accepted; missing, ambiguous, unreadable, empty and leaked definitions rejected")
