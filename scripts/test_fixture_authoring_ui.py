#!/usr/bin/env python3
"""Real Fixture picker/Add/Undo controls on a generated controller-free profile.

Uses existing admin/timeline hooks only to open/focus and read render state.
Authoring uses the actual picker menu actions, Add button and Edit menu commands.
Retains screenshots/OCR and an independent read-only saved-sequence inspection.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess

from test_video_cache_ui import wait, hook, capture


def open_sequence(sequence):
    return [hook("admin.openSequence", str(sequence)), wait(3),
            {"action": "focus", "window": "Fixture Authoring"}, hook("sequencer.activate", "Fixture Authoring"),
            hook("sequencer.resize", "1500", "1200"), wait(),
            {"action": "mouse", "x": 900, "y": 340}, wait(),
            {"action": "menu", "path": ["Edit", "Select All Elements"]}, wait(),
            hook("sequencer.panel", "Effect Editor"), wait()]


def observed(ui, name, ocr):
    output = subprocess.check_output([str(ocr), str(ui / (name + ".png"))], text=True, timeout=45)
    (ui / (name + ".ocr.json")).write_text(output + "\n")
    lines = json.loads(output)
    add = [line for line in lines if re.search(r"\bAdd\b", line["text"]) and line["x"] > .6]
    assert len(add) == 1, (name, add)
    below = [line["text"] for line in lines if line["y"] < add[0]["y"] - add[0]["height"] / 2 and line["x"] > .6]
    headers = [header for header in ("Gobo", "RGB") if any(re.search(r"\b" + header + r"\b", text) for text in below)]
    return {"headers": headers, "textBelowAdd": below, "allText": [line["text"] for line in lines]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--inspector", type=Path, default=Path("target/build-tools/rust/debug/examples/fixture_authoring_fixture"))
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--resume", action="store_true", help="Recheck completed native artifacts after a postprocessing failure; launch only unfinished stages")
    args = parser.parse_args()
    profile = args.profile.resolve()
    assert "target" in profile.parts
    fixture = json.loads((profile / "fixture.json").read_text())
    assert fixture["muted"] and fixture["controllers"] == 0 and fixture["channels"] == 2
    sequence = Path(fixture["sequence"]).resolve()
    assert sequence.is_relative_to(profile) and sequence.is_file()
    out = args.out.resolve()
    assert out.is_dir() if args.resume else not out.exists(), out
    out.mkdir(parents=True, exist_ok=args.resume)
    original_hash = None if args.resume else hashlib.sha256(sequence.read_bytes()).hexdigest()
    app_hash = hashlib.sha256((args.app.resolve() / "Contents/MacOS/Vixen").read_bytes()).hexdigest()
    start = [{"action": "window", "id": "administration"}, wait(2)]
    finish = [{"action": "closeWindow"}, wait(), {"action": "quit"}]
    observe = lambda name: capture(name) + [hook("timelineState"), hook("sequencer.state")]
    workflows = {
        "01-add-functions": start + open_sequence(sequence) + observe("empty")
            + [{"action": "popup", "name": "Pan", "label": "Gobo"}, wait(), {"action": "click", "label": "Add"}, wait(1)] + observe("gobo")
            + [{"action": "popup", "name": "Star", "label": "Circle"}, wait(1)] + observe("circle")
            + [{"action": "popup", "name": "Gobo", "label": "RGB"}, wait(), {"action": "click", "label": "Add"}, wait(1)] + observe("both")
            + [{"action": "menu", "path": ["Edit", "Undo 1 Effect(s) Fixture Functions modified"]}, wait(1)] + observe("undo")
            + [{"action": "menu", "path": ["Edit", "Redo 1 Effect(s) Fixture Functions modified"]}, wait(1)] + observe("redo")
            + [{"action": "menu", "path": ["Sequence", "Save"]}, wait(), hook("sequencer.state")] + finish,
        "02-reopen": start + open_sequence(sequence) + observe("reopened") + finish,
    }
    ocr = out / "read-ui-snapshot"
    if not args.prepare_only:
        subprocess.run(["/usr/bin/xcrun", "swiftc", "-module-cache-path", str(out / "swift-cache"),
                        str(Path(__file__).with_name("read_ui_snapshot.swift")), "-o", str(ocr)], check=True, timeout=60)
    report = {"passed": False, "muted": True, "runs": {}, "profile": str(profile),
              "limits": ["Posted Add/menu events and actual picker menu actions; no physical focus/drag claim",
                         "UI headers/choices observed in screenshots; persisted values/intents inspected separately without writing"]}
    for name, steps in workflows.items():
        run = out / name
        run.mkdir(exist_ok=args.resume)
        script = run / "steps.json"
        if script.exists():
            assert json.loads(script.read_text()) == {"steps": steps}, "Resume must use the same native script"
        script.write_text(json.dumps({"steps": steps}, indent=2) + "\n")
        if args.prepare_only:
            continue
        ui = run / "ui"
        ui.mkdir(exist_ok=args.resume)
        env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
        env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1", VIXEN_DATA=str(profile), VIXEN_MUTE="1")
        status_path = run / "process.json"
        reused = args.resume and (ui / "result.json").exists()
        if reused:
            status = json.loads(status_path.read_text())
            assert status["appSha256"] == app_hash, "Resume requires the same app executable"
        else:
            with (run / "process.log").open("w") as log:
                process = subprocess.Popen([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                           stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    process.wait(timeout=80)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                    raise
            status = {"exitCode": process.returncode, "appSha256": app_hash}
            status_path.write_text(json.dumps(status, indent=2) + "\n")
        rows = json.loads((ui / "result.json").read_text())
        assert status["exitCode"] == 0 and len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
        names = ["empty", "gobo", "circle", "both", "undo", "redo"] if name == "01-add-functions" else ["reopened"]
        screenshots = {capture_name: observed(ui, capture_name, ocr) for capture_name in names}
        expected = [[], ["Gobo"], ["Gobo"], ["Gobo", "RGB"], ["Gobo"], ["Gobo", "RGB"]] if name == "01-add-functions" else [["Gobo", "RGB"]]
        assert [screenshots[n]["headers"] for n in names] == expected, screenshots
        for capture_name in names:
            text = " ".join(screenshots[capture_name]["textBelowAdd"])
            if capture_name == "empty": continue
            assert "Index Value" in text and ("Star" if capture_name == "gobo" else "Circle") in text, (capture_name, text)
            assert ("Range" in text) == (capture_name != "gobo"), (capture_name, text)
            if "RGB" in screenshots[capture_name]["headers"]:
                assert "Color" in text, (capture_name, text)
        if name == "02-reopen":
            assert any(re.search(r"\bPan\b", text) for text in screenshots["reopened"]["allText"]), screenshots
        states = [row["detail"] for row in rows if row.get("detail", "").startswith("effects=")]
        assert all("effects=1 selected=1" in state for state in states), states
        if name == "01-add-functions":
            assert "modified=false" in states[0] and "modified=false" in states[-1]
            assert all("modified=true" in state for state in states[1:-1]), states
        else:
            assert all("modified=false" in state and "undo=-" in state for state in states), states
        timelines = [json.loads(row["detail"]) for row in rows if '"renderStatus"' in row.get("detail", "")]
        assert timelines and all(t["renderStatus"] == {"waiting": 0, "failed": 0} and t["elements"][0]["rendered"] for t in timelines), timelines
        saved_hash = hashlib.sha256(sequence.read_bytes()).hexdigest()
        if name == "01-add-functions":
            if original_hash is not None: assert saved_hash != original_hash
        else: assert saved_hash == report["runs"]["01-add-functions"]["savedSha256"], "Reopen rewrote sequence"
        inspection = json.loads(subprocess.check_output([str(args.inspector.resolve()), str(profile), "--inspect"], text=True, timeout=20))
        assert hashlib.sha256(sequence.read_bytes()).hexdigest() == saved_hash, "Inspector mutated sequence"
        assert [r["name"] for r in inspection["rows"]] == ["Gobo", "RGB"], inspection
        gobo, rgb = inspection["rows"]
        assert gobo["type"] == "Indexed" and gobo["identity"] == "Gobo" and gobo["index"] == "Circle" and gobo["zeroRange"]
        assert gobo["timelineColor"] == 0xFF0000FF and rgb["timelineColor"] == 0xFF008000
        assert rgb["type"] == "RGBColor" and rgb["fullRange"] and rgb["white"]
        assert inspection["rendered"] and len(inspection["leaves"]) == 2 and all(leaf["gobo"] and leaf["whiteRgb"] for leaf in inspection["leaves"]), inspection
        assert inspection["selectedFunction"] == "Pan" and not inspection["modified"] and inspection["undoCount"] == 0
        record = {"steps": len(rows), "screenshots": screenshots, "states": states, "savedSha256": saved_hash, "savedInspection": inspection,
                  "reusedNativeArtifacts": reused}
        report["runs"][name] = record
        (run / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
    if args.prepare_only:
        print(f"Prepared scripts: {out}")
        return
    report.update(passed=True, appSha256=hashlib.sha256((args.app.resolve() / "Contents/MacOS/Vixen").read_bytes()).hexdigest())
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "out": str(out), "steps": sum(r["steps"] for r in report["runs"].values())}))


if __name__ == "__main__":
    main()
