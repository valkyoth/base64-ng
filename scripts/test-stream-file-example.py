#!/usr/bin/env python3
"""Exercise the compiled file example, including Unix creation permissions."""

import argparse
import os
from pathlib import Path
import stat
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("toolchain")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    command = ["cargo", f"+{args.toolchain}", "run", "--quiet", "--locked",
               "--example", "stream_file", "--features", "stream", "--"]

    def run(source, destination, *, success):
        # Set the child's umask only; do not alter the test process's umask.
        options = {"umask": 0} if os.name == "posix" else {}
        result = subprocess.run(command + [str(source), str(destination)],
                                cwd=root, capture_output=True, text=True,
                                timeout=120, **options)
        if (result.returncode == 0) != success:
            raise AssertionError(f"unexpected exit {result.returncode}: {result.stderr}")

    with tempfile.TemporaryDirectory(prefix="base64-ng-file-example-") as directory:
        work = Path(directory)
        source = work / "input"
        source.write_bytes(b"foobar!")
        destination = work / "output"
        run(source, destination, success=True)
        assert destination.read_bytes() == b"Zm9vYmFyIQ=="
        if os.name == "posix":
            mode = stat.S_IMODE(destination.stat().st_mode)
            assert mode == 0o600, f"output mode under umask 000 is {mode:04o}, not 0600"

        run(source, destination, success=False)
        assert destination.read_bytes() == b"Zm9vYmFyIQ=="
        run(source, source, success=False)
        assert source.read_bytes() == b"foobar!"

        if os.name == "posix":
            link = work / "output-link"
            link.symlink_to(destination)
            run(source, link, success=False)
            assert link.is_symlink()
            assert destination.read_bytes() == b"Zm9vYmFyIQ=="
            assert stat.S_IMODE(destination.stat().st_mode) == 0o600
            dangling = work / "dangling-output"
            missing = work / "missing"
            dangling.symlink_to(missing)
            run(source, dangling, success=False)
            assert dangling.is_symlink()
            assert not missing.exists()

    print(f"stream file example: content, exclusive creation and permissions passed ({args.toolchain})")


if __name__ == "__main__":
    main()
