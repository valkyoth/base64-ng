#!/usr/bin/env python3
"""Run the base64-ng wasm runtime smoke page through a WebDriver browser."""

from __future__ import annotations

import argparse
import base64
import http.client
import json
import os
import pathlib
import socket
import subprocess
import sys
import time
import urllib.parse


ROOT = pathlib.Path(__file__).resolve().parents[1]
SMOKE_DIR = ROOT / "target" / "wasm-runtime-smoke"
PASS_TEXT = "BASE64_NG_BROWSER_WASM_SMOKE_PASS"


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def request(
    port: int,
    method: str,
    path: str,
    body: dict[str, object] | None = None,
) -> dict[str, object]:
    payload = None if body is None else json.dumps(body).encode("utf-8")
    headers = {"Content-Type": "application/json"} if payload is not None else {}
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    try:
        conn.request(method, path, payload, headers)
        response = conn.getresponse()
        data = response.read()
    finally:
        conn.close()

    if response.status >= 400:
        text = data.decode("utf-8", "replace")
        raise RuntimeError(f"WebDriver {method} {path} failed: {response.status} {text}")
    if not data:
        return {}
    return json.loads(data.decode("utf-8"))


def wait_for_driver(port: int, process: subprocess.Popen[bytes]) -> None:
    deadline = time.monotonic() + 15
    last_error = ""
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"WebDriver exited early with status {process.returncode}")
        try:
            request(port, "GET", "/status")
            return
        except Exception as error:  # noqa: BLE001 - report last startup error.
            last_error = str(error)
            time.sleep(0.2)
    raise RuntimeError(f"WebDriver did not become ready: {last_error}")


def build_smoke_module(target: str) -> pathlib.Path:
    wasm_file = SMOKE_DIR / "target" / target / "release" / "base64_ng_wasm_runtime_smoke.wasm"
    if wasm_file.exists() and wasm_file.stat().st_size > 0:
        return wasm_file

    print("wasm webdriver dispatch: smoke module missing; building through runtime smoke")
    subprocess.run(
        [str(ROOT / "scripts" / "check_wasm_runtime_dispatch.sh"), target],
        cwd=ROOT,
        check=True,
    )
    if not wasm_file.exists() or wasm_file.stat().st_size == 0:
        raise RuntimeError(f"smoke module was not produced at {wasm_file}")
    return wasm_file


def write_html(browser: str, wasm_file: pathlib.Path) -> pathlib.Path:
    wasm_base64 = base64.b64encode(wasm_file.read_bytes()).decode("ascii")
    html_file = SMOKE_DIR / f"browser-smoke-{browser}.html"
    html_file.write_text(
        f"""<!doctype html>
<html>
<head><meta charset="utf-8"><title>base64-ng wasm {browser} smoke</title></head>
<body data-base64-ng-wasm-smoke="pending">
<pre id="result">pending</pre>
<script>
const wasmBase64 = "{wasm_base64}";

function decodeBase64(input) {{
  const binary = atob(input);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {{
    bytes[index] = binary.charCodeAt(index);
  }}
  return bytes;
}}

try {{
  const module = new WebAssembly.Module(decodeBase64(wasmBase64));
  const instance = new WebAssembly.Instance(module, {{}});
  const result = instance.exports.base64_ng_wasm_runtime_smoke();
  if (result === 0) {{
    document.body.setAttribute("data-base64-ng-wasm-smoke", "pass");
    document.getElementById("result").textContent = "{PASS_TEXT}";
  }} else {{
    document.body.setAttribute("data-base64-ng-wasm-smoke", "fail-" + result);
    document.getElementById("result").textContent = "BASE64_NG_BROWSER_WASM_SMOKE_FAIL_" + result;
  }}
}} catch (error) {{
  document.body.setAttribute("data-base64-ng-wasm-smoke", "exception");
  document.getElementById("result").textContent = "BASE64_NG_BROWSER_WASM_SMOKE_EXCEPTION " + error;
}}
</script>
</body>
</html>
""",
        encoding="utf-8",
    )
    return html_file


def capabilities(browser: str, headless: bool) -> dict[str, object]:
    always_match: dict[str, object] = {"browserName": browser}
    if browser == "firefox" and headless:
        always_match["moz:firefoxOptions"] = {"args": ["-headless"]}
    return {"capabilities": {"alwaysMatch": always_match}}


def session_id(payload: dict[str, object]) -> str:
    value = payload.get("value")
    if isinstance(value, dict):
        session = value.get("sessionId")
        if isinstance(session, str):
            return session
    session = payload.get("sessionId")
    if isinstance(session, str):
        return session
    raise RuntimeError(f"WebDriver did not return a session id: {payload!r}")


def run(args: argparse.Namespace) -> None:
    wasm_file = build_smoke_module(args.target)
    html_file = write_html(args.browser, wasm_file)
    html_url = html_file.resolve().as_uri()

    port = free_port()
    driver_cmd = [args.driver, "--port", str(port)]
    print(f"wasm webdriver dispatch: running {args.browser} smoke with {args.driver}")
    process = subprocess.Popen(  # noqa: S603 - driver path is an explicit local argument.
        driver_cmd,
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )

    session = None
    try:
        wait_for_driver(port, process)
        payload = request(port, "POST", "/session", capabilities(args.browser, args.headless))
        session = session_id(payload)
        request(port, "POST", f"/session/{session}/url", {"url": html_url})

        deadline = time.monotonic() + args.timeout
        last_result = ""
        while time.monotonic() < deadline:
            script = (
                "const body = document.body;"
                "return body ? [body.getAttribute('data-base64-ng-wasm-smoke'), "
                "document.getElementById('result')?.textContent || ''].join(':') : 'loading';"
            )
            result = request(
                port,
                "POST",
                f"/session/{session}/execute/sync",
                {"script": script, "args": []},
            )
            value = result.get("value")
            last_result = "" if value is None else str(value)
            if PASS_TEXT in last_result:
                print("wasm webdriver dispatch: ok")
                return
            if "fail-" in last_result or "exception" in last_result:
                raise RuntimeError(f"{args.browser} wasm smoke failed: {last_result}")
            time.sleep(0.2)
        raise RuntimeError(f"{args.browser} wasm smoke timed out: {last_result}")
    finally:
        if session is not None:
            try:
                request(port, "DELETE", f"/session/{session}")
            except Exception:
                pass
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser", choices=("firefox", "safari"), required=True)
    parser.add_argument("--driver", required=True)
    parser.add_argument("--target", default="wasm32-unknown-unknown")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument(
        "--headless",
        action=argparse.BooleanOptionalAction,
        default=os.environ.get("BASE64_NG_WEBDRIVER_HEADLESS", "1") != "0",
    )
    args = parser.parse_args()
    try:
        run(args)
    except Exception as error:  # noqa: BLE001 - CLI boundary.
        print(f"wasm webdriver dispatch: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
