#!/usr/bin/env python3
"""Run the extracted base64-ng-wasm-loader npm package in a real browser."""

from __future__ import annotations

import argparse
import contextlib
import functools
import http.server
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time

from wasm_webdriver_smoke import capabilities, free_port, request, session_id, wait_for_driver


ROOT = pathlib.Path(__file__).resolve().parents[1]
SERVE_ROOT = ROOT / "target" / "wasm-loader-package"
PAGE = "browser-smoke.html"
PASS_TEXT = "BASE64_NG_WASM_LOADER_BROWSER_PASS"


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, _format: str, *_args: object) -> None:
        pass


@contextlib.contextmanager
def serve() -> str:
    if not (SERVE_ROOT / "package" / "src" / "index.js").is_file():
        subprocess.run([str(ROOT / "scripts" / "check-2.0-wasm-loader.sh")], cwd=ROOT, check=True)
    handler = functools.partial(QuietHandler, directory=SERVE_ROOT)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/{PAGE}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def run_chromium(binary: str, url: str, timeout: float) -> None:
    profile = pathlib.Path(tempfile.mkdtemp(prefix="base64-ng-wasm-loader-", dir=SERVE_ROOT))
    try:
        result = subprocess.run(
            [
                binary,
                "--headless=new",
                "--disable-gpu",
                "--no-sandbox",
                "--disable-dev-shm-usage",
                "--virtual-time-budget=30000",
                f"--user-data-dir={profile}",
                "--dump-dom",
                url,
            ],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        output = result.stdout + result.stderr
        if result.returncode != 0 or PASS_TEXT not in output:
            raise RuntimeError(f"Chromium loader smoke failed ({result.returncode}):\n{output[-4000:]}")
        marker = next((line.strip() for line in output.splitlines() if PASS_TEXT in line), PASS_TEXT)
        print(f"2.0 wasm loader browser: {marker}")
    finally:
        shutil.rmtree(profile, ignore_errors=True)


def run_webdriver(browser: str, driver: str, url: str, timeout: float, headless: bool) -> None:
    port = free_port()
    log_path = SERVE_ROOT / f"webdriver-loader-{browser}.log"
    with log_path.open("wb") as log:
        process = subprocess.Popen(
            [driver, "--port", str(port)],
            cwd=ROOT,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        session = None
        try:
            wait_for_driver(port, process)
            payload = request(port, "POST", "/session", capabilities(browser, headless))
            session = session_id(payload)
            request(
                port,
                "POST",
                f"/session/{session}/timeouts",
                {
                    "implicit": 0,
                    "pageLoad": int(timeout * 1000),
                    "script": int(timeout * 1000),
                },
            )
            request(port, "POST", f"/session/{session}/url", {"url": url})
            deadline = time.monotonic() + timeout
            last = ""
            while time.monotonic() < deadline:
                result = request(
                    port,
                    "POST",
                    f"/session/{session}/execute/sync",
                    {
                        "script": "return document.getElementById('result')?.textContent || 'loading'",
                        "args": [],
                    },
                )
                last = str(result.get("value", ""))
                if PASS_TEXT in last:
                    print(f"2.0 wasm loader browser: {last}")
                    return
                if "_FAIL" in last:
                    raise RuntimeError(f"{browser} loader smoke failed: {last}")
                time.sleep(0.2)
            raise RuntimeError(f"{browser} loader smoke timed out: {last}")
        finally:
            if session is not None:
                with contextlib.suppress(Exception):
                    request(port, "DELETE", f"/session/{session}")
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser", choices=("chromium", "firefox", "safari"), required=True)
    parser.add_argument("--binary")
    parser.add_argument("--driver")
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--no-headless", action="store_true")
    args = parser.parse_args()
    try:
        with serve() as url:
            if args.browser == "chromium":
                if not args.binary:
                    raise RuntimeError("--binary is required for Chromium")
                run_chromium(args.binary, url, args.timeout)
            else:
                if not args.driver:
                    raise RuntimeError("--driver is required for WebDriver browsers")
                run_webdriver(args.browser, args.driver, url, args.timeout, not args.no_headless)
        print(f"2.0 wasm loader browser: {args.browser} ok")
        return 0
    except Exception as error:  # noqa: BLE001 - command boundary reports context.
        print(f"2.0 wasm loader browser: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
