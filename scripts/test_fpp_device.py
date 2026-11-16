#!/usr/bin/env python3
"""Loopback-only FPP/ESPixelStick fixture. State JSON selects fpp, esp, zip, fail, or slow.

Usage: python3 scripts/test_fpp_device.py target/direct-ui/device
Writes port.txt, requests.jsonl and captured upload bytes inside that test directory.
"""
import hashlib
import http.server
import json
from pathlib import Path
import sys
import time
import urllib.parse

root = Path(sys.argv[1]).resolve()
root.mkdir(parents=True, exist_ok=True)
state_file = root / "state.json"
if not state_file.exists():
    state_file.write_text('{"mode":"fpp"}')


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        self.respond()

    def do_POST(self):
        self.respond()

    def respond(self):
        state = json.loads(state_file.read_text())
        mode = state.get("mode", "fpp")
        length = int(self.headers.get("Content-Length", "0"))
        if mode == "slow" and self.command == "POST":
            time.sleep(5)
        body = self.rfile.read(length)
        parsed = urllib.parse.urlsplit(self.path)
        filename = urllib.parse.parse_qs(parsed.query).get("filename", [urllib.parse.unquote(parsed.path.rsplit("/", 1)[-1])])[0]
        stored = None
        if body:
            stored = mode + "-" + Path(filename).name
            (root / stored).write_bytes(body)
        record = {"method": self.command, "path": self.path, "mode": mode, "size": len(body), "expected": length, "sha256": hashlib.sha256(body).hexdigest(), "file": stored}
        with (root / "requests.jsonl").open("a") as stream:
            stream.write(json.dumps(record) + "\n")
        status = 200
        if parsed.path == "/api/system/info":
            response = {"HostName": "Loopback Test Device", "HostDescription": "Simulated device; no hardware", "Platform": "ESPixelStick" if mode in ("esp", "zip") else "Raspberry Pi", "Variant": "Loopback", "Zip": mode == "zip"}
        elif mode == "fail":
            status, response = 500, {}
        elif parsed.path == "/fpp":
            response = {"Name": filename}
        elif parsed.path == "/X6":
            response = {"status": "Rebooting"}
        else:
            response = {"Status": "OK"}
        payload = json.dumps(response).encode()
        try:
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(payload)
        except (BrokenPipeError, ConnectionResetError):
            pass


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
(root / "port.txt").write_text(str(server.server_port))
print(server.server_address, flush=True)
server.serve_forever()
