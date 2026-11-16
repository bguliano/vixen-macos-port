#!/usr/bin/env python3
"""Check selected-only lyric conversion through the real text view and Convert button.

Copies a private controller-free profile, uses the harness's private pasteboard,
and never plays audio or saves the source sequence. Text/selection setup uses the
existing hooks; radio, checkbox, Convert and alert actions use actual controls.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import xml.etree.ElementTree as ET


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--sequence", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    source = args.profile.resolve()
    source_files = {p.relative_to(source): digest(p) for p in source.rglob("*") if p.is_file()}
    config = ET.parse(source / "SystemData/SystemConfig.xml")
    assert not [e for e in config.iter() if e.tag.split("}")[-1] == "Controller"], "Use a controller-free test profile"
    out = args.out.resolve()
    assert not out.exists() and not out.is_relative_to(source), "Use a fresh output outside the source profile"
    out.mkdir(parents=True)
    profile = out / "profile"
    shutil.copytree(source, profile)
    (profile / ".lock").unlink(missing_ok=True)
    sequence = profile / "Sequence/Selection Test.tim"
    shutil.copyfile(args.sequence.resolve(), sequence)
    sequence_hash = digest(sequence)
    hook = lambda name, *path: {"action": "hook", "name": name, "path": list(path)}
    wait = lambda seconds=.3: {"action": "wait", "seconds": seconds}
    text = "🦊 zzyqxv hello zzyqxv"
    start = len("🦊 zzyqxv ".encode("utf-16-le")) // 2
    steps = [
        {"action": "window", "id": "administration"}, wait(2),
        hook("admin.openSequence", str(sequence)), wait(3),
        {"action": "focus", "window": "Selection Test"}, hook("sequencer.activate", "Selection Test"),
        {"action": "menu", "path": ["Tools", "LipSync", "Lyric Converter"]}, wait(),
        {"action": "focus", "window": "LipSync Text Convert"},
        {"action": "click", "label": "Clipboard / Even Spacing"}, wait(),
        hook("lyricText", text), wait(), hook("lyricSelect", str(start), "5"), wait(),
        {"action": "snapshot", "name": "selected-text"},
    ]

    def convert(name):
        # The OCR label also matches the window title. This observed point is
        # the actual footer button in the default 340-point converter window.
        steps.extend([{"action": "mouse", "x": 292, "y": 458}, wait(.6), hook("lyricState", name),
                      {"action": "snapshot", "name": name}, {"action": "click", "label": "OK"}, wait()])

    convert("selected-only")
    steps.extend([hook("lyricText", "hello world"), wait(), hook("lyricSelect", "0", "0"), wait()])
    convert("whole-text")
    steps.extend([hook("lyricSelect", "6", "5"), wait(),
                  {"action": "click", "label": "Clear after conversion"}, wait()])
    convert("selected-and-clear")
    steps.extend([{ "action": "snapshot", "name": "cleared-text"}, {"action": "quit"}])
    script = out / "steps.json"
    script.write_text(json.dumps({"steps": steps}, indent=2) + "\n")
    ui = out / "ui"
    ui.mkdir()
    env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
    env.update(VIXEN_DATA=str(profile), VIXEN_MUTE="1", VIXEN_UI_TEST=str(script),
               VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1")
    executable = args.app.resolve() / "Contents/MacOS/Vixen"
    with (out / "process.log").open("w") as log:
        process = subprocess.Popen([str(executable)], env=env, stdout=log, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            process.wait(timeout=80)
        except BaseException:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            raise
    rows = json.loads((ui / "result.json").read_text())
    assert process.returncode == 0 and len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
    selections = [row["detail"] for row in rows if row.get("detail", "").startswith("selected ")]
    assert selections == ["selected hello", "selected ", "selected world"], selections
    hello, world = ["ETC", "AI", "L", "O"], ["WQ", "E", "L", "ETC"]
    checks = {}
    for name, phonemes, words, remaining in [
        ("selected-only", hello, ["hello"] * 4, text),
        ("whole-text", hello + world, ["hello"] * 4 + ["world"] * 4, "hello world"),
        ("selected-and-clear", world, ["world"] * 4, ""),
    ]:
        state = json.loads((ui / (name + ".json")).read_text())
        assert state["placement"] == "clipboard" and not state["failure"], state
        assert state["alertTitle"] == "Convert Text" and state["text"] == remaining, state
        assert state["pasteboardCount"] == len(phonemes), state["pasteboardCount"]
        assert state["lastOutcome"] == f"delivered {len(phonemes)} clipboard at 0 pasted 0", state["lastOutcome"]
        models = [entry["EffectModel"] for entry in state["pasteboard"]["EffectModelCandidates"]]
        data = [{e.tag.split("}")[-1]: e.text for e in ET.fromstring(model["EffectData"])} for model in models]
        assert [row["StaticPhoneme"] for row in data] == phonemes, data
        assert [row["LyricData"] for row in data] == words, data
        assert all(model["Duration"] == 2_500_000 for model in models), models
        assert state["clearAfter"] == (name == "selected-and-clear")
        checks[name] = {"count": len(models), "phonemes": phonemes, "lyrics": words,
                        "remainingText": remaining, "outcome": state["lastOutcome"]}
    assert digest(sequence) == sequence_hash, "Clipboard conversion changed the sequence file"
    assert {p.relative_to(source): digest(p) for p in source.rglob("*") if p.is_file()} == source_files
    report = {"passed": True, "nativeSteps": len(rows), "releaseSha256": digest(executable),
              "selectionDetails": selections, "checks": checks, "inputFilesUnchanged": True,
              "limits": "Native text-view selection setup and real controls; no physical drag/typing or general pasteboard claim"}
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "nativeSteps": len(rows), "out": str(out)}))


if __name__ == "__main__":
    main()
