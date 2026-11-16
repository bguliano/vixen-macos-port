#!/usr/bin/env python3
"""Exercise a bundled/relocated helper's private protocol without Terminal UI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import uuid


def run(app: Path, output: Path):
    helper = app.resolve() / "Contents/Helpers/vixen-launch-helper"
    assert helper.is_file() and os.access(helper, os.X_OK), helper
    output.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="vixen-launch-probe-", dir="/private/tmp"))
    token = str(uuid.uuid4())
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    process = None
    client = None
    reader = None
    try:
        listener.bind(str(root / "control.sock"))
        listener.listen(1)
        listener.settimeout(10)
        pidfile = root / "pid"
        late = root / "late"
        program = root / "program.sh"
        program.write_text(f"#!/bin/sh\necho $$ > '{pidfile}'\n(sleep 0.8; touch '{late}') &\nwait\n")
        program.chmod(0o700)
        (root / "request.json").write_text(json.dumps({"token": token, "application": False,
                                                       "program": str(program), "arguments": []}))
        process = subprocess.Popen([str(helper), str(root)], stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        client, _ = listener.accept()
        client.settimeout(10)
        reader = client.makefile("rb")
        assert reader.readline().decode().strip() == token
        client.sendall(b"START\n")
        assert json.loads(reader.readline())["event"] == "started"
        deadline = time.monotonic() + 5
        while not pidfile.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        pid = int(pidfile.read_text())
        client.sendall(b"STOP\n")
        message = json.loads(reader.readline())
        assert message == {"event": "done"}, message
        assert process.wait(timeout=6) == 0
        reader.close()
        client.close()
        client = None
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError("owned program survived Stop")
        time.sleep(0.85)
        assert not late.exists(), "owned descendant survived Stop"
        report = {"ok": True, "bundle": str(app.resolve()), "helper": str(helper),
                  "sha256": hashlib.sha256(helper.read_bytes()).hexdigest(),
                  "checks": ["private handshake", "actual program start", "Stop kills owned process group",
                             "helper exits cleanly"], "terminalWindowTested": False}
        (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
    finally:
        if reader is not None:
            reader.close()
        if client is not None:
            client.close()  # owner EOF requests child cleanup
        listener.close()
        if process is not None and process.poll() is None:
            try:
                process.wait(timeout=6)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        shutil.rmtree(root)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    arguments = parser.parse_args()
    run(arguments.app, arguments.out)
