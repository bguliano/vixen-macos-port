#!/usr/bin/env python3
"""Raise a native exception only in a dedicated UI-test app and a newly copied profile."""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path


def hashes(folder: Path) -> dict[str, str]:
    return {str(path.relative_to(folder)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in folder.rglob("*") if path.is_file()}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-scheduler/Vixen.app"))
    parser.add_argument("--source", type=Path, default=Path("target/direct-ui/profile"))
    parser.add_argument("--out", type=Path, default=Path("target/admin-fatal-ui"))
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    profile = out / "profile"
    assert not profile.exists(), f"Use a fresh output directory: {profile}"
    source = args.source.resolve()
    original = hashes(source / "SystemData")
    shutil.copytree(source, profile)
    # A copied fixture may carry old diagnostic logs, but no live lock may be inherited.
    (profile / ".lock").unlink(missing_ok=True)
    shutil.rmtree(profile / "logs", ignore_errors=True)
    steps = {"steps": [{"action": "window", "id": "administration"},
                       {"action": "wait", "seconds": 1},
                       {"action": "hook", "name": "admin.fatal", "path": ["raise"]},
                       {"action": "wait", "seconds": 10}, {"action": "quit"}]}
    script = out / "fatal.json"
    script.write_text(json.dumps(steps, indent=2))
    env = os.environ.copy()
    env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(out / "ui"),
               VIXEN_DATA=str(profile), VIXEN_MUTE="1", VIXEN_TEST_FATAL="uncaught-exception")
    started = time.monotonic()
    with (out / "process.log").open("w") as log:
        result = subprocess.run([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=30)
    elapsed = time.monotonic() - started
    assert result.returncode == 1, f"expected fatal_exit(1), got {result.returncode}"
    fatal = (profile / "logs/Application.Fatal.log").read_text()
    info = (profile / "logs/Application.Info.log").read_text()
    assert "[VixenApplication.Program]" in fatal
    assert "An application error occurred. Please contact the Vixen Dev Team" in fatal
    assert "VixenFatalPathFixture: Deliberate exception in disposable UI-test process" in fatal
    assert "Profile lock present before deliberate native exception" in info
    assert not (profile / ".lock").exists(), "fatal exit retained the profile lock"
    assert hashes(source / "SystemData") == original, "source profile was changed"
    result = {"passed": True, "exitCode": 1, "seconds": elapsed,
              "nativeExceptionLogged": True, "lockRemoved": True, "sourceUnchanged": True}
    (out / "result.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result))


if __name__ == "__main__":
    main()
