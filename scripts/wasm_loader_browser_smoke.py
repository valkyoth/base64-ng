#!/usr/bin/env python3
"""Run the extracted base64-ng-wasm-loader npm package in a real browser."""

from __future__ import annotations

import argparse
import contextlib
import functools
import http.server
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass, field

from wasm_webdriver_smoke import capabilities, free_port, request, session_id, wait_for_driver


ROOT = pathlib.Path(__file__).resolve().parents[1]
SERVE_ROOT = ROOT / "target" / "wasm-loader-package"
PAGE = "browser-smoke.html"
PASS_TEXT = "BASE64_NG_WASM_LOADER_BROWSER_PASS"
FAIL_TEXT = "BASE64_NG_WASM_LOADER_BROWSER_FAIL"


@dataclass
class BrowserResult:
    event: threading.Event = field(default_factory=threading.Event)
    text: str = ""


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args: object, result: BrowserResult, **kwargs: object) -> None:
        self.result = result
        super().__init__(*args, **kwargs)

    def log_message(self, _format: str, *_args: object) -> None:
        pass

    def do_POST(self) -> None:  # noqa: N802 - inherited HTTP handler API.
        if self.path != "/__base64_ng_wasm_loader_result":
            self.send_error(404)
            return
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self.send_error(400)
            return
        if not 0 < length <= 4096:
            self.send_error(400)
            return
        body = self.rfile.read(length).decode("utf-8", errors="replace")
        self.result.text = body
        self.result.event.set()
        self.send_response(204)
        self.end_headers()


@contextlib.contextmanager
def serve() -> tuple[str, BrowserResult]:
    if not (SERVE_ROOT / "package" / "src" / "index.js").is_file():
        subprocess.run([str(ROOT / "scripts" / "check-2.0-wasm-loader.sh")], cwd=ROOT, check=True)
    result = BrowserResult()
    handler = functools.partial(QuietHandler, directory=SERVE_ROOT, result=result)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/{PAGE}", result
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)


def run_chromium(
    binary: str, url: str, timeout: float, browser_result: BrowserResult
) -> None:
    profile = pathlib.Path(tempfile.mkdtemp(prefix="base64-ng-wasm-loader-", dir=SERVE_ROOT))
    log_path = profile / "chromium.log"
    environment = os.environ.copy()
    environment.update(
        {
            "HOME": str(profile),
            "XDG_CACHE_HOME": str(profile / "cache"),
            "XDG_CONFIG_HOME": str(profile / "config"),
        }
    )
    try:
        with log_path.open("wb") as log:
            process = subprocess.Popen(
                [
                    binary,
                    "--headless=new",
                    "--disable-gpu",
                    "--no-sandbox",
                    "--disable-dev-shm-usage",
                    "--disable-background-networking",
                    "--disable-breakpad",
                    "--no-first-run",
                    "--password-store=basic",
                    f"--user-data-dir={profile}",
                    url,
                ],
                cwd=ROOT,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            deadline = time.monotonic() + timeout
            while not browser_result.event.wait(timeout=0.1):
                if process.poll() is not None:
                    break
                if time.monotonic() >= deadline:
                    break
            marker = browser_result.text
            if PASS_TEXT in marker:
                print(f"2.0 wasm loader browser: {marker}")
                return
            process_status = process.poll()
            detail = "timed out" if process_status is None else f"exited {process_status}"
            log.flush()
            output = log_path.read_text(encoding="utf-8", errors="replace")[-4000:]
            if FAIL_TEXT in marker:
                raise RuntimeError(f"Chromium loader smoke reported failure: {marker}\n{output}")
            raise RuntimeError(f"Chromium loader smoke {detail} without a result callback:\n{output}")
    finally:
        if "process" in locals() and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
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
        with serve() as (url, browser_result):
            if args.browser == "chromium":
                if not args.binary:
                    raise RuntimeError("--binary is required for Chromium")
                run_chromium(args.binary, url, args.timeout, browser_result)
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
