#!/usr/bin/env python3
"""Run the staged private startup/playback/media checks; never infer macOS26 from host smoke."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import stat

# The entry point itself also imports a packaged helper. Keep that import from
# writing __pycache__ into the immutable transferred pack.
sys.dont_write_bytecode = True
from prepare_private_test_app import prepare


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True, help="Fresh results directory outside this immutable pack")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--require-macos26", action="store_true")
    mode.add_argument("--host-smoke", action="store_true", help="Verify only the actual host; never mark minimumOSVerified")
    args = parser.parse_args()
    assert sys.version_info >= (3, 11), "Python3.11 or newer is required; no third-party packages"
    pack = Path(__file__).resolve().parents[1]
    out = args.out.resolve()
    assert not out.exists() and not out.is_relative_to(pack), "Use a fresh results directory outside the pack"
    manifest = json.loads((pack / "manifest.json").read_text())
    for name, expected in manifest["files"].items():
        path = (pack / name).resolve()
        assert path.is_relative_to(pack) and digest(path) == expected, f"Transfer integrity failed: {name}"
        if "fileModes" in manifest:
            assert stat.S_IMODE(path.stat().st_mode) == manifest["fileModes"][name], f"Transfer permissions changed: {name}"
    host = {"version": subprocess.check_output(["/usr/bin/sw_vers", "-productVersion"], text=True).strip(),
            "build": subprocess.check_output(["/usr/bin/sw_vers", "-buildVersion"], text=True).strip(),
            "architecture": platform.machine(), "python": platform.python_version()}
    out.mkdir(parents=True)
    report = {"passed": False, "minimumOSVerified": False, "mode": "macOS26" if args.require_macos26 else "host-smoke",
              "host": host, "releaseSha256": manifest["releaseSha256"], "transferHashesVerified": True, "checks": [],
              "buildStatus": manifest.get("buildStatus", "accepted-release"), "promoted": manifest.get("promoted", True),
              "sourceNativeCheckpoint": manifest.get("currentNativeCheckpoint"),
              "pendingVerification": manifest.get("pendingVerification", []),
              "limits": ["Muted offscreen AppKit; no physical focus/gesture evidence", "No audible/controller output",
                         "Direct launch; no Finder, permission, Developer ID/notarization or Gatekeeper claim"]}
    def publish():
        (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    if host["architecture"] != "arm64" or (args.require_macos26 and host["version"].split(".")[0] != "26"):
        report["error"] = "Requires an Apple-silicon Mac running macOS26; this host does not satisfy that gate."
        publish()
        print(report["error"], file=sys.stderr)
        return 2
    work = out / "target"
    work.mkdir()
    temporary = work / "runtime"
    temporary.mkdir()
    env = {key: value for key, value in os.environ.items() if not key.startswith(("VIXEN_", "DYLD_"))}
    settings = work / "settings"
    settings.mkdir()
    env.update(TMPDIR=str(temporary) + "/", PYTHONDONTWRITEBYTECODE="1", VIXEN_SETTINGS_DIR=str(settings))
    app = None
    def run(name, script, *arguments):
        print(f"Running {name}", flush=True)
        with (out / (name + ".log")).open("w") as log:
            subprocess.run([sys.executable, str(pack / "scripts" / script), "--app", str(app), *map(str, arguments)],
                           cwd=work, env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=180)
        report["checks"].append({"name": name, "passed": True, "log": name + ".log"})
        publish()
    try:
        private = prepare(pack / "Vixen.app", work / "private-app", output_root=work)
        app = Path(private["app"])
        report["privateApp"] = private
        report["settingsIsolation"] = {
            "parentSettingsDirectory": str(settings),
            "appLaunches": "Each Vixen child runner clears inherited VIXEN_* and sets its own VIXEN_UI_TEST plus VIXEN_UI_TEST_OUT; the latter is its Rust settings directory.",
            "cocoaPreferences": "All Vixen launches, including the relocated copy, use the new private bundle identifier.",
            "launchHelper": "The non-harness helper inherits the parent override and runs only its generated shell fixture, not Vixen.",
        }
        publish()
        run("relocation", "test_bundle_relocation.py", "--out", work / "relocation")
        run("playback", "test_editor_ownership.py", "--profile", pack / "fixtures/playback",
            "--sequence", pack / "fixtures/playback/Sequence/Video Cache.tim", "--out", work / "playback")
        profile = work / "metadata-profile"
        shutil.copytree(pack / "fixtures/metadata", profile)
        run("video-metadata", "test_video_metadata_ui.py", "--profile", profile, "--out", work / "video-metadata",
            "--ocr-helper", pack / "helpers/read-ui-snapshot")
        for name, expected in manifest["files"].items():
            assert digest(pack / name) == expected, f"Verification modified packaged input: {name}"
            if "fileModes" in manifest:
                assert stat.S_IMODE((pack / name).stat().st_mode) == manifest["fileModes"][name], f"Verification changed packaged permissions: {name}"
        report.update(passed=True, minimumOSVerified=args.require_macos26, nativeSteps=164,
                      inputsUnchanged=True, hostSmokeOnly=not args.require_macos26)
        publish()
        print(json.dumps({"passed": True, "minimumOSVerified": report["minimumOSVerified"], "out": str(out)}))
        return 0
    except Exception as error:
        report["error"] = str(error)
        publish()
        raise


if __name__ == "__main__":
    sys.exit(main())
