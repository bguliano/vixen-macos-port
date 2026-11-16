#!/usr/bin/env python3
"""Exercise real Options and the displayed Video Cache Size on a private profile.

First generate the profile with the video_cache_fixture Rust example. This script
launches only the requested test app, muted and offscreen, and retains AX trees,
window snapshots, actual cache-image hashes, and per-launch results. No playback.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import xml.etree.ElementTree as ET
from video_fixture_paths import fixture_path, fixture_cache, runtime_environment

# Vision's first local model initialization can exceed15s; subsequent images
# are fast. Bound that cold start without relaunching the completed test app.
OCR_TIMEOUT = 45


def wait(seconds=.5):
    return {"action": "wait", "seconds": seconds}


def hook(name, *args):
    return {"action": "hook", "name": name, "path": list(args)}


def capture(name):
    return [{"action": "dump", "name": name}, {"action": "snapshot", "name": name}]


def options(toggle_clear=False, old_type=None, new_type=None):
    # The sequencer and administration each contribute a Tools menu. The global
    # Settings item opens this same Options form from either active window.
    steps = [{"action": "menu", "path": ["Vixen", "Settings..."]}, wait(),
             {"action": "focus", "window": "Options"}]
    if toggle_clear:
        # The grouped SwiftUI form's text is not the switch's press target.
        # Observed 440pt Options window: full-size content includes the titlebar.
        steps.extend([{"action": "mouse", "x": 392, "y": 112}, wait()])
    if new_type:
        # Observed 440x385-point Options form at both 1x and 2x backing scale.
        # Vision reads the final p of bmp as Cyrillic at 1x. Open the actual
        # popup at its reviewed point; choosePopup still requires its NSMenu
        # and invokes the requested native item's action.
        steps.append({"action": "popup", "x": 392, "y": 300, "label": new_type})
    return steps + capture("options") + [{"action": "closeWindow"}, wait()]


def open_sequence(path):
    return [hook("admin.openSequence", str(path)), wait(3),
            {"action": "focus", "window": "Video Cache"},
            hook("sequencer.activate", "Video Cache"),
            hook("sequencer.resize", "1500", "1200"), wait(),
            # Mouse events are posted. Let the blank click finish before Select
            # All, otherwise its delayed mouse-up can clear the new selection.
            {"action": "mouse", "x": 900, "y": 340}, wait(),
            {"action": "menu", "path": ["Edit", "Select All Elements"]},
            wait(), hook("timelineState"),
            hook("sequencer.panel", "Effect Editor"), wait(),
            # SwiftUI's disclosure label is not its arrow's press target.
            {"action": "click", "label": "Advanced Settings", "disclosure": True}, wait(),
            hook("effectEditor.cacheProbe"), hook("sequencer.state"), hook("timelineState")]


def cache_snapshot(root):
    result = {}
    for manifest_path in sorted(root.glob("*/manifest.json")):
        manifest = json.loads(manifest_path.read_text())
        folder = manifest_path.parent
        frames = sorted(folder.glob("????????." + manifest["extension"]))
        assert len(frames) == manifest["count"], (folder, len(frames), manifest)
        files = {}
        for path in frames + [manifest_path]:
            data = path.read_bytes()
            if path.suffix == ".png":
                assert data.startswith(b"\x89PNG\r\n\x1a\n"), path
            elif path.suffix == ".bmp":
                assert data.startswith(b"BM"), path
            files[path.name] = {"bytes": len(data), "mtimeNs": path.stat().st_mtime_ns,
                                "sha256": hashlib.sha256(data).hexdigest()}
        size = sum(files[path.name]["bytes"] for path in frames)
        assert size == manifest["bytes"] and size > 0, manifest
        result[folder.name] = {"type": manifest["extension"], "bytes": size,
                               "label": f"{size / 1048576:.2f} MB", "files": files}
    return result


def cache_label(ui, name, ocr):
    text = (ui / (name + ".txt")).read_text()
    if "Cache Size" in text:
        following = text[text.index("Cache Size"):].splitlines()[:6]
        found = re.findall(r'"(\d+\.\d{2} MB)"', "\n".join(following))
        if len(found) == 1:
            assert found[0] != "0.00 MB", following
            return found[0], "accessibility"
    # Do not replace a missing SwiftUI AX label with its backend/model value.
    # Read the exact saved pixels and retain the recognized text + positions.
    output = subprocess.check_output([str(ocr), str(ui / (name + ".png"))], text=True, timeout=OCR_TIMEOUT)
    (ui / (name + ".ocr.json")).write_text(output + "\n")
    lines = json.loads(output)
    labels = [line for line in lines if "Cache Size" in line["text"]]
    assert len(labels) == 1, lines
    label = labels[0]
    row = [line for line in lines if abs(line["y"] - label["y"]) < max(line["height"], label["height"])
           and line["x"] >= label["x"]]
    found = re.findall(r'(\d+\.\d{2})\s*MB', " ".join(line["text"] for line in row))
    assert len(found) == 1 and found[0] != "0.00", row
    return found[0] + " MB", "snapshot OCR"


def stored_options(profile):
    config = next(profile.glob("SystemConfig.xml"), None)
    if config is None:
        config = next(profile.rglob("SystemConfig.xml"))
    nodes = {node.tag.rsplit("}", 1)[-1]: node.text for node in ET.parse(config).getroot().iter()}
    return {"clear": nodes["ClearEffectCacheOnExit"], "type": nodes["VideoEffectCacheFileType"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("build/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true", help="Write scripts without launching the app")
    parser.add_argument("--codec-smoke", action="store_true", help="Run just the retained PNG decode/label stage for another codec")
    parser.add_argument("--ocr-helper", type=Path, help="Precompiled read_ui_snapshot helper; otherwise compile locally")
    args = parser.parse_args()
    profile = args.profile.resolve()
    assert "target" in profile.parts, "Use only a generated private target profile"
    fixture = json.loads((profile / "fixture.json").read_text())
    assert fixture["controllers"] == 0 and fixture["muted"]
    sequence = profile / "Sequence/Video Cache.tim"
    runtime_env = runtime_environment(profile, fixture)
    cache = fixture_cache(profile, fixture)
    assert not cache_snapshot(cache), "Use an unrendered fresh fixture"
    assert stored_options(profile) == {"clear": "true", "type": "bmp"}
    out = args.out.resolve()
    assert not out.exists(), f"Use a fresh output directory: {out}"
    out.mkdir(parents=True)
    ocr = args.ocr_helper.resolve() if args.ocr_helper else out / "read-ui-snapshot"
    if args.ocr_helper:
        assert ocr.is_file() and os.access(ocr, os.X_OK), ocr
    elif not args.prepare_only:
        subprocess.run(["/usr/bin/xcrun", "swiftc", "-module-cache-path", str(out / "swift-cache"),
                        str(Path(__file__).with_name("read_ui_snapshot.swift")), "-o", str(ocr)], check=True, timeout=60)
    sources = list(profile.rglob("generated.*"))
    assert len(sources) == 1 and sources[0].is_file(), sources
    source = sources[0]
    source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
    original = fixture_path(profile, fixture, "source") if "source" in fixture else None
    if original:
        assert hashlib.sha256(original.read_bytes()).hexdigest() == source_hash
    start = [{"action": "window", "id": "administration"}, wait(2)]
    finish = [{"action": "closeWindow"}, wait(), {"action": "quit"}]
    workflows = {
        "01-retain-png": start + options(True, "bmp", "png") + open_sequence(sequence) + capture("rendered") + finish,
        "02-reopen-png": start + open_sequence(sequence) + capture("rendered") + finish,
        "03-selected-rerender-bmp": start + open_sequence(sequence) + capture("before")
            + options(False, "png", "bmp") + [{"action": "focus", "window": "Video Cache"}, hook("sequencer.activate", "Video Cache")]
            + [hook("effectEditor.cacheProbe", "render")] + capture("pending") + [wait(3)]
            + [hook("timelineState")] + capture("rendered") + [hook("effectEditor.cacheProbe"), hook("sequencer.state"),
               {"action": "menu", "path": ["Sequence", "Save"]}, wait(), hook("sequencer.state")] + finish,
        "04-clear-on-close": start + open_sequence(sequence) + capture("rendered") + options(True)
            + [{"action": "focus", "window": "Video Cache"}, hook("sequencer.state")] + finish,
    }
    if args.codec_smoke:
        workflows = {"01-retain-png": workflows["01-retain-png"]}
    report = {"passed": False, "muted": True, "profile": str(profile), "cache": str(cache), "runs": {},
              "video": str(source), "codecSmoke": args.codec_smoke,
              "limits": ["AppKit harness and actual AX/screenshot labels; no physical focus claim",
                         "Options use real controls; rerender uses the normal property-commit path through a test hook"]}
    for name, steps in workflows.items():
        run = out / name
        run.mkdir()
        script = run / "steps.json"
        script.write_text(json.dumps({"steps": steps}, indent=2) + "\n")
        if args.prepare_only:
            continue
        ui = run / "ui"
        ui.mkdir()
        env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
        env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1",
                   VIXEN_DATA=str(profile), VIXEN_MUTE="1")
        env.update(runtime_env)
        with (run / "process.log").open("w") as log:
            process = subprocess.Popen([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                       stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                process.wait(timeout=65)
            except subprocess.TimeoutExpired:
                # Only this private test application's session is ours to stop.
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
                raise
        assert (ui / "result.json").exists(), f"No harness result; inspect {run / 'process.log'}"
        rows = json.loads((ui / "result.json").read_text())
        assert process.returncode == 0 and len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
        states = [row["detail"] for row in rows if row.get("detail", "").startswith("effects=")]
        assert states and all("effects=1 selected=1" in state for state in states), states
        expected_dirty = [False, True, False] if name == "03-selected-rerender-bmp" else [False] * len(states)
        assert [("modified=true" in state) for state in states] == expected_dirty, states
        timelines = [json.loads(row["detail"]) for row in rows if '"renderStatus"' in row.get("detail", "")]
        assert timelines and timelines[-1]["renderStatus"] == {"waiting": 0, "failed": 0}, timelines
        assert len(timelines[-1]["elements"]) == 1 and timelines[-1]["elements"][0]["rendered"], timelines[-1]
        cache = fixture_cache(profile, fixture)
        report["cache"] = str(cache)
        snapshot = cache_snapshot(cache)
        label, observation = cache_label(ui, "rendered", ocr)
        record = {"steps": len(rows), "label": label, "observation": observation,
                  "cache": snapshot, "options": stored_options(profile), "states": states,
                  "renderStatus": timelines[-1]["renderStatus"]}
        (run / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
        report["runs"][name] = record
        if name == "01-retain-png":
            assert len(snapshot) == 1 and next(iter(snapshot.values()))["type"] == "png", snapshot
            assert label == next(iter(snapshot.values()))["label"]
            assert record["options"] == {"clear": "false", "type": "png"}
        elif name == "02-reopen-png":
            assert snapshot == report["runs"]["01-retain-png"]["cache"], "Disk frames were rewritten on reopen"
            assert label == report["runs"]["01-retain-png"]["label"]
        elif name == "03-selected-rerender-bmp":
            assert len(snapshot) == 1 and next(iter(snapshot.values()))["type"] == "bmp", snapshot
            assert label == next(iter(snapshot.values()))["label"]
            record["previousLabel"] = cache_label(ui, "before", ocr)[0]
            record["pendingLabel"] = cache_label(ui, "pending", ocr)[0]
            assert record["previousLabel"] != label, "Selected panel retained its old cache size"
            assert record["pendingLabel"] in (record["previousLabel"], label), record
            selections = [row["detail"] for row in rows if row.get("detail", "").startswith("selection=")]
            assert len(selections) == 3 and len(set(selections)) == 1, selections
            record["selectionAcrossRender"] = selections
            assert record["options"] == {"clear": "false", "type": "bmp"}
        else:
            assert not snapshot, "Clear-on-close retained cache frames"
            assert sorted(path.name for path in cache.iterdir()) == [".owner"], "Clear-on-close retained cache pairings/staging files"
            assert label == report["runs"]["03-selected-rerender-bmp"]["label"]
            assert record["options"] == {"clear": "true", "type": "bmp"}
        assert hashlib.sha256(source.read_bytes()).hexdigest() == source_hash, "Generated source was modified"
        if original:
            assert hashlib.sha256(original.read_bytes()).hexdigest() == source_hash, "Original codec fixture was modified"
        (run / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
    if args.prepare_only:
        print(f"Prepared scripts: {out}")
        return
    report["passed"] = True
    report["sourceSha256"] = source_hash
    report["bundledHelpers"] = {name: hashlib.sha256((args.app.resolve() / "Contents/Helpers" / name).read_bytes()).hexdigest()
                                for name in ("ffmpeg", "ffprobe")}
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "out": str(out), "runs": len(workflows)}))


if __name__ == "__main__":
    main()
