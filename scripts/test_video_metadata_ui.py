#!/usr/bin/env python3
"""Observe FileName-triggered Video metadata using a generated, muted profile.

Generate with video_cache_fixture --metadata. The guarded hook commits through
the normal file chooser path; it never assigns computed fields. Duration/cache
are read from saved window pixels, and quality from the actual NSSlider.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess

from test_video_cache_ui import wait, hook, capture, options, open_sequence, cache_snapshot, cache_label, stored_options, OCR_TIMEOUT
from video_fixture_paths import fixture_path, fixture_cache, runtime_environment


def visible_duration(ui, name):
    lines = json.loads((ui / (name + ".ocr.json")).read_text())
    labels = [line for line in lines if "video length" in line["text"].lower()]
    assert len(labels) == 1, lines
    label = labels[0]
    row = sorted([line for line in lines if abs(line["y"] - label["y"]) < max(line["height"], label["height"])
                  and line["x"] >= label["x"]], key=lambda line: line["x"])
    text = " ".join(line["text"] for line in row)
    match = re.search(r"video length\s*\(?sec\)?\s*([0-9.]+)", text, re.I)
    assert match, text
    return float(match[1])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--ocr-helper", type=Path, help="Precompiled read_ui_snapshot helper; otherwise compile locally")
    args = parser.parse_args()
    profile = args.profile.resolve()
    assert "target" in profile.parts
    fixture = json.loads((profile / "fixture.json").read_text())
    assert fixture["metadata"] and fixture["muted"] and fixture["controllers"] == 0
    assert stored_options(profile) == {"clear": "true", "type": "bmp"}, "Use a fresh default-options fixture"
    choices = [fixture_path(profile, {**fixture, "choice": path}, "choice") for path in fixture["choices"]]
    sequence = fixture_path(profile, fixture, "sequence")
    assert sequence.is_file() and sequence.is_relative_to(profile)
    assert len(choices) == 2 and all(path.is_file() and path.is_relative_to(profile) for path in choices)
    hashes = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in [fixture_path(profile, fixture, "video"), *choices]}
    runtime_env = runtime_environment(profile, fixture)
    cache = fixture_cache(profile, fixture)
    assert not cache_snapshot(cache), "Use a fresh, unrendered metadata fixture"
    out = args.out.resolve()
    assert not out.exists(), out
    out.mkdir(parents=True)
    start = [{"action": "window", "id": "administration"}, wait(2)]
    finish = [{"action": "closeWindow"}, wait(), {"action": "quit"}]
    observe = lambda name: capture(name) + [hook("effectEditor.cacheProbe", "metadata"), hook("timelineState"), hook("sequencer.state")]
    workflows = {
        "01-file-selection": start + options(True, "bmp", "png") + open_sequence(sequence)
            + observe("before")
            + [hook("effectEditor.cacheProbe", "file", str(choices[0])), wait(3)] + observe("mov")
            + [hook("effectEditor.cacheProbe", "quality", "80"), wait(1), hook("effectEditor.cacheProbe", "metadata")]
            + [hook("effectEditor.cacheProbe", "file", str(choices[0])), wait(3)] + observe("same-file")
            + [hook("effectEditor.cacheProbe", "file", str(choices[1])), wait(3)] + observe("avi")
            + [{"action": "menu", "path": ["Sequence", "Save"]}, wait(), hook("sequencer.state")] + finish,
        "02-saved-metadata": start + open_sequence(sequence) + observe("reopened") + finish,
    }
    ocr = args.ocr_helper.resolve() if args.ocr_helper else out / "read-ui-snapshot"
    if args.ocr_helper:
        assert ocr.is_file() and os.access(ocr, os.X_OK), ocr
    elif not args.prepare_only:
        subprocess.run(["/usr/bin/xcrun", "swiftc", "-module-cache-path", str(out / "swift-cache"),
                        str(Path(__file__).with_name("read_ui_snapshot.swift")), "-o", str(ocr)], check=True, timeout=60)
    report = {"passed": False, "muted": True, "profile": str(profile), "runs": {},
              "limits": ["File commit uses normal chooser callback through guarded hook; no physical file-dialog gesture claim",
                         "Duration/cache observed in screenshot OCR; quality read from the native slider; selection never reassigned during render"]}
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
        env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1", VIXEN_DATA=str(profile), VIXEN_MUTE="1")
        env.update(runtime_env)
        with (run / "process.log").open("w") as log:
            process = subprocess.Popen([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                       stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                process.wait(timeout=80)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
                raise
        rows = json.loads((ui / "result.json").read_text())
        assert process.returncode == 0 and len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
        metadata = [json.loads(row["detail"]) for row in rows if '"qualityControl"' in row.get("detail", "")]
        states = [row["detail"] for row in rows if row.get("detail", "").startswith("effects=")]
        assert states and all("effects=1 selected=1" in state for state in states), states
        assert all(len(value["selection"]) == 1 and value["selection"] == metadata[0]["selection"] for value in metadata), metadata
        names = ["before", "mov", "same-file", "avi"] if name == "01-file-selection" else ["reopened"]
        labels = {}
        for capture_name in names:
            label, observation = cache_label(ui, capture_name, ocr)
            # Always retain OCR for the duration/FileName even when Cache Size
            # happened to be exposed through accessibility on this OS build.
            ocr_path = ui / (capture_name + ".ocr.json")
            if not ocr_path.exists():
                ocr_path.write_text(subprocess.check_output([str(ocr), str(ui / (capture_name + ".png"))], text=True, timeout=OCR_TIMEOUT))
            duration = visible_duration(ui, capture_name)
            assert duration == (0 if capture_name == "before" else 3), (capture_name, duration)
            if capture_name != "before":
                expected_file = choices[0].name if capture_name in ("mov", "same-file") else choices[1].name
                assert any(expected_file in line["text"] for line in json.loads(ocr_path.read_text())), (capture_name, expected_file)
            labels[capture_name] = {"duration": duration, "cache": label, "cacheObservation": observation}
        if name == "01-file-selection":
            assert [m["qualityControl"] for m in metadata] == [80, 50, 80, 50, 50], metadata
            assert [m["undoCount"] for m in metadata] == [0, 1, 2, 2, 3], metadata
            assert "modified=false" in states[0] and "modified=false" in states[-1], states
            assert all("modified=true" in state for state in states[2:-1]), states
        else:
            assert metadata == [{"selection": metadata[0]["selection"], "qualityControl": 50, "undoCount": 0}], metadata
            assert all("modified=false" in state for state in states), states
        timelines = [json.loads(row["detail"]) for row in rows if '"renderStatus"' in row.get("detail", "")]
        assert timelines[-1]["renderStatus"] == {"waiting": 0, "failed": 0}, timelines
        assert timelines[-1]["elements"][0]["rendered"]
        cache = fixture_cache(profile, fixture)
        snapshot = cache_snapshot(cache)
        assert len(snapshot) == 1 and next(iter(snapshot.values()))["label"] == labels[names[-1]]["cache"], snapshot
        if name == "02-saved-metadata":
            assert snapshot == report["runs"]["01-file-selection"]["cache"], "Reopen rewrote retained frames"
        record = {"steps": len(rows), "metadata": metadata, "states": states, "labels": labels, "cache": snapshot}
        report["runs"][name] = record
        (run / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
        assert all(hashlib.sha256(Path(path).read_bytes()).hexdigest() == expected for path, expected in hashes.items())
    if args.prepare_only:
        print(f"Prepared scripts: {out}")
        return
    report.update(passed=True, sourceSha256=hashes,
                  appSha256=hashlib.sha256((args.app.resolve() / "Contents/MacOS/Vixen").read_bytes()).hexdigest())
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "out": str(out), "steps": sum(r["steps"] for r in report["runs"].values())}))


if __name__ == "__main__":
    main()
