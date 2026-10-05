#!/usr/bin/env python3
"""Exercise the compiled file example, including Unix creation permissions."""

import argparse
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile


def build_example(root, toolchain, target_dir):
    target_dir = target_dir.resolve()
    command = ["cargo", f"+{toolchain}", "build", "--locked",
               "--example", "stream_file", "--features", "stream",
               "--message-format=json", "--target-dir", str(target_dir)]
    env = dict(os.environ, CARGO_TARGET_DIR=str(target_dir),
               CARGO_BUILD_BUILD_DIR=str(target_dir / "build"),
               RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="")
    # Never give Cargo, rustc, the linker or build scripts the hostile test umask.
    options = {"umask": 0o077} if os.name == "posix" else {}
    result = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True,
                            check=True, timeout=120, **options)
    executables = []
    for line in result.stdout.splitlines():
        message = json.loads(line)
        target = message.get("target", {})
        if (message.get("reason") == "compiler-artifact"
                and target.get("name") == "stream_file"
                and "example" in target.get("kind", [])
                and message.get("executable")):
            executables.append(message["executable"])
    if len(executables) != 1:
        raise AssertionError("expected exactly one stream_file executable")
    executable = Path(executables[0]).resolve()
    executable.relative_to(target_dir)
    return str(executable)


def run_example(executable, source, destination, root, *, success):
    # Only the already-built example receives umask 000, never a Cargo runner.
    options = {"umask": 0} if os.name == "posix" else {}
    result = subprocess.run([executable, str(source), str(destination)],
                            cwd=root, capture_output=True, text=True,
                            timeout=120, **options)
    if (result.returncode == 0) != success:
        raise AssertionError(f"unexpected exit {result.returncode}: {result.stderr}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("toolchain")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="base64-ng-file-example-") as directory:
        work = Path(directory)
        executable = build_example(root, args.toolchain, work / "target")

        def run(source, destination, *, success):
            run_example(executable, source, destination, root, success=success)

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
