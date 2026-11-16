#!/usr/bin/env python3
"""Launch a copied release bundle offscreen from an unrelated cwd and fresh muted profile.

No source profile is used. The copied app and working directory are temporary;
JSON, text and screenshots remain under --out. No external/network APIs are used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from check_app_bundle import audit
from test_native_launch_helper import run as check_launch_helper


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--out", type=Path, default=Path("target/bundle-relocation"))
    args = parser.parse_args()
    app = args.app.resolve()
    source_executable = app / "Contents/MacOS/Vixen"
    source_sha256 = hashlib.sha256(source_executable.read_bytes()).hexdigest()
    out = args.out.resolve()
    repo = Path(__file__).resolve().parents[1]
    assert not out.exists(), f"Use a fresh output directory: {out}"
    out.mkdir(parents=True)
    profile = out / "profile"
    profile.mkdir()
    ui = out / "ui"
    ui.mkdir()
    steps = {"steps": [
        {"action": "window", "id": "administration"}, {"action": "wait", "seconds": 0.6},
        {"action": "hook", "name": "admin.webServer", "path": []},
        {"action": "dump", "name": "administration"}, {"action": "snapshot", "name": "administration"},
        {"action": "window", "id": "about"}, {"action": "wait", "seconds": 0.4},
        {"action": "dump", "name": "about"}, {"action": "snapshot", "name": "about"},
        {"action": "closeWindow"},
        {"action": "window", "id": "release-notes"}, {"action": "wait", "seconds": 0.5},
        {"action": "dump", "name": "release-notes"}, {"action": "snapshot", "name": "release-notes"},
        {"action": "closeWindow"}, {"action": "quit"},
    ]}
    script = out / "steps.json"
    script.write_text(json.dumps(steps, indent=2))
    # An unrelated directory plus spaces/non-ASCII in the bundle path catches cwd/path assumptions.
    with tempfile.TemporaryDirectory(prefix="Vixen relocation ü ", dir="/private/tmp") as folder:
        folder = Path(folder)
        moved = folder / "Moved Apps" / "Vixen ü.app"
        shutil.copytree(app, moved)
        moved_executable = moved / "Contents/MacOS/Vixen"
        moved_sha256 = hashlib.sha256(moved_executable.read_bytes()).hexdigest()
        assert moved_sha256 == source_sha256, "Relocated executable differs from source"
        cwd = folder / "Unrelated working directory"
        cwd.mkdir()
        checked = audit(moved, repo / "app/Resources", "arm64")
        (out / "bundle-audit.json").write_text(json.dumps(checked, indent=2))
        assert checked["ok"], checked["errors"]
        # The helper must run from the relocated copy, without a checkout path.
        # Its private command fixture does not open Terminal or any user app.
        check_launch_helper(moved, out / "launch-helper")
        env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
        env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_DATA=str(profile), VIXEN_MUTE="1")
        started = time.monotonic()
        with (out / "process.log").open("w") as log:
            result = subprocess.run([str(moved / "Contents/MacOS/Vixen")], env=env, cwd=cwd,
                                    stdout=log, stderr=subprocess.STDOUT, timeout=40)
        elapsed = time.monotonic() - started
        assert result.returncode == 0, f"Relocated app exited {result.returncode}; inspect process.log"
        rows = json.loads((ui / "result.json").read_text())
        assert len(rows) >= 15 and all(row["ok"] for row in rows), rows
        details = [row.get("detail", "") for row in rows]
        assert any("running=false" in detail for detail in details), details
        about = (ui / "about.txt").read_text()
        notes = (ui / "release-notes.txt").read_text()
        assert "Copyright 2023 Vixen Team" in about, "Bundled license missing from About"
        assert "Vixen is free to use." in about, "Bundled license text is incomplete"
        assert "Release Notes - Vixen 3" in notes and "VIX-4012" in notes, "Bundled release-note fallback missing"
        # WMA is rejected by the primary decoder, so a nonblank waveform proves
        # that this relocated process resolved and used its bundled media helpers.
        # The child runner copies this synthetic fixture into a fresh private profile.
        media_out = out / "media-fallback"
        fixture = repo / "crates/vixen-audio/tests/assets/tone.wma"
        media_started = time.monotonic()
        with (out / "media-fallback-runner.log").open("w") as log:
            media_process = subprocess.run(
                [sys.executable, str(repo / "scripts/test_legacy_media_open.py"),
                 "--app", str(moved), "--out", str(media_out), "--audio", str(fixture)],
                env={k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))},
                cwd=cwd, stdout=log, stderr=subprocess.STDOUT, timeout=50)
        assert media_process.returncode == 0, (
            f"Relocated media check exited {media_process.returncode}; inspect media-fallback-runner.log")
        media_report = json.loads((media_out / "result.json").read_text())
        assert media_report["passed"] and not media_report["waveform"]["blank"], media_report
        media_evidence = {
            "report": str(media_out / "result.json"), "steps": media_report["steps"],
            "seconds": time.monotonic() - media_started, "cwd": str(cwd),
            "codecFixture": str(fixture), "waveform": media_report["waveform"],
            "helpers": [str(moved / "Contents/Helpers" / name) for name in ("ffmpeg", "ffprobe")],
        }
        # The harness exits directly; this check deliberately makes no graceful-shutdown claim.
        assert hashlib.sha256(source_executable.read_bytes()).hexdigest() == source_sha256
        assert hashlib.sha256(moved_executable.read_bytes()).hexdigest() == moved_sha256
        report = {"passed": True, "steps": len(rows), "seconds": elapsed, "exitCode": result.returncode,
                  "sourceBundle": str(app), "sourceExecutableSha256": source_sha256,
                  "relocatedExecutableSha256": moved_sha256, "executableHashesUnchanged": True,
                  "relocatedBundle": str(moved), "cwd": str(cwd), "profile": str(profile),
                  "resources": checked["resources"]["files"], "licenseLoaded": True, "releaseNotesLoaded": True,
                  "webStopped": True, "muted": True, "signatureAdhoc": checked["signature"]["adhoc"],
                  "nativeLaunchHelper": str(out / "launch-helper/result.json"),
                  "mediaFallback": media_evidence,
                  "limits": ["Direct executable launch; no Gatekeeper/notarization claim", "Fresh private empty profile; no real show or hardware", "Harness direct exit; no graceful-shutdown claim"]}
        (out / "result.json").write_text(json.dumps(report, indent=2))
        print(json.dumps(report))


if __name__ == "__main__":
    main()
