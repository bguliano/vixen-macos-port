#!/usr/bin/env python3
"""Bounded, muted Image Maps verification on a generated controller-free profile.

Use image_maps_fixture to generate inputs. Only existing controls/hooks are used;
native file-panel results are supplied, no external editor/physical drag runs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import xml.etree.ElementTree as ET

from test_video_cache_ui import wait, hook, capture


def opening(sequence):
    return [{"action": "window", "id": "administration"}, wait(2),
            hook("admin.openSequence", str(sequence)), wait(3),
            {"action": "focus", "window": "Image Maps Verification"},
            hook("sequencer.activate", "Image Maps Verification"), hook("sequencer.resize", "1400", "900"), wait(),
            hook("timelineState"), hook("sequencer.state")]


def maps():
    return [hook("sequencer.activate", "Image Maps Verification"), wait(),
            {"action": "menu", "path": ["Tools", "LipSync", "Edit Image Maps"]}, wait()]


def key(name):
    return [{"action": "key", "key": name}, wait()]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--stage", choices=["probe", "keys", "close", "delete", "mapping", "overwrite"], default="probe")
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--inspector", type=Path, default=Path("target/vixen-session/debug/examples/image_maps_fixture"))
    args = parser.parse_args()
    profile, out = args.profile.resolve(), args.out.resolve()
    assert "target" in profile.parts and not out.exists()
    fixture = json.loads((profile / "fixture.json").read_text())
    assert fixture["controllers"] == 0 and fixture["muted"]
    sequence = profile / "Sequence/Image Maps Verification.tim"
    assert sequence.is_file()
    inputs = [profile / "Inputs/red.png", profile / "Inputs/green.png"]
    hashes = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
    open_maps = opening(sequence) + maps()
    select = lambda name: {"action": "selectRow", "label": name}
    click = lambda label: {"action": "click", "label": label}
    finish = [hook("timelineState"), hook("sequencer.state"), {"action": "menu", "path": ["Sequence", "Save"]}, wait(), {"action": "quit"}]
    if args.stage == "probe":
        steps = open_maps + capture("maps") + [select("Active Map"), click("Edit"), wait()] + capture("editor") \
            + [hook("marksNextFile", *map(str, inputs)), click("File"), wait()] + capture("mapper") \
            + [click("Cancel"), wait(), click("Cancel"), wait(), click("OK"), wait(), {"action": "quit"}]
    elif args.stage == "close":
        steps = open_maps + key("return") + [{"action":"windows"}] + maps() + key("escape") \
            + [{"action":"windows"}, hook("timelineState"), hook("sequencer.state"), {"action":"quit"}]
    elif args.stage == "keys":
        steps = open_maps + capture("initial") + key("return") + [{"action":"windows"}] + maps() + key("escape") \
            + [{"action":"windows"}] + maps() \
            + [select("Rename Me"), {"action":"popup","x":75,"y":159,"button":"right","label":"Rename"}, wait(), {"action":"key","key":"a","modifiers":["cmd"]},
               {"action":"type","text":"Discarded Rename"}] + key("escape") + capture("rename-cancel") \
            + [select("Active Map"), select("Rename Me"), {"action":"popup","x":75,"y":159,"button":"right","label":"Rename"}, wait(), {"action":"key","key":"a","modifiers":["cmd"]},
               {"action":"type","text":"Renamed Map"}] + key("return") + capture("rename-commit") \
            + [select("Delete Me"), {"action":"mouse","x":75,"y":111}] + key("delete") + capture("delete-question") \
            + [click("Cancel"), wait(), select("Delete Me"), {"action":"mouse","x":75,"y":111}] + key("delete") \
            + [click("Delete"), wait()] + capture("deleted") + key("return") + finish
    elif args.stage == "delete":
        steps = open_maps + [select("Delete Me"), {"action":"mouse","x":75,"y":111}] + key("delete") \
            + capture("delete-question") + [click("Cancel"), wait(), select("Delete Me"), {"action":"mouse","x":75,"y":111}] \
            + key("delete") + [click("Delete"), wait()] + capture("deleted") + key("return") + finish
    elif args.stage == "mapping":
        steps = open_maps + [select("Active Map"), click("Edit"), wait(), hook("marksNextFile", *map(str, inputs)), click("File"), wait()] \
            + capture("mapper-before") + [{"action":"popup","name":"red.png","label":"green.png"}, wait(),
                {"action":"popup","name":"green.png","index":1,"label":"red.png"}, wait()] + capture("mapper-changed") \
            + [click("Accept"), wait(), click("OK"), wait(), click("OK"), wait(2)] \
            + capture("after-mapping") + finish
    else:
        steps = open_maps + [select("Active Map"), click("Edit"), wait(),
                {"action":"type","text":"Overwrite Me"}, click("OK"), wait()] + capture("overwrite-question") \
            + [click("Overwrite"), wait(), click("OK"), wait(2)] + capture("after-overwrite") + finish
    out.mkdir(parents=True)
    script = out / "steps.json"
    script.write_text(json.dumps({"steps":steps}, indent=2) + "\n")
    if args.prepare_only:
        print(script)
        return
    app = args.app.resolve()
    before = json.loads(subprocess.check_output([str(args.inspector.resolve()), str(profile), "--inspect"], text=True, timeout=20))
    app_sha = hashlib.sha256((app / "Contents/MacOS/Vixen").read_bytes()).hexdigest()
    ui = out / "ui"
    ui.mkdir()
    runtime = out / "runtime"
    runtime.mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
    env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1", VIXEN_DATA=str(profile), VIXEN_MUTE="1", TMPDIR=str(runtime)+"/")
    with (out / "process.log").open("w") as log:
        process = subprocess.Popen([str(app / "Contents/MacOS/Vixen")], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            process.wait(timeout=110)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise
    rows = json.loads((ui / "result.json").read_text())
    report = {"stage":args.stage,"passed":process.returncode==0 and len(rows)==len(steps)-1 and all(row["ok"] for row in rows),
              "exitCode":process.returncode,"steps":len(rows),"appSha256":app_sha,"profile":str(profile),"inputSha256":hashes}
    (out / "result.json").write_text(json.dumps(report, indent=2)+"\n")
    assert all(hashlib.sha256(Path(path).read_bytes()).hexdigest()==digest for path,digest in hashes.items())
    assert hashlib.sha256((app / "Contents/MacOS/Vixen").read_bytes()).hexdigest()==app_sha
    assert report["passed"], rows
    if args.stage != "probe":
        # These independent disk checks are distinct from the in-process native
        # action result. Inspection only loads/renders; it never saves anything.
        files = [sequence, profile / "SystemData/SystemConfig.xml", profile / "SystemData/ModuleStore.xml"]
        disk_before = {str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
        inspected = json.loads(subprocess.check_output([str(args.inspector.resolve()), str(profile), "--inspect"], text=True, timeout=20))
        assert disk_before == {str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
        expected = set(before["names"])
        if args.stage in ("keys", "delete"): expected.remove("Delete Me")
        if args.stage == "keys": expected.remove("Rename Me"); expected.add("Renamed Map")
        if args.stage == "overwrite": expected.remove("Active Map")
        assert set(inspected["names"]) == expected, inspected
        assert not inspected["modified"] and inspected["effects"][0]["rendered"]
        colors = set(inspected["effects"][0]["colors"])
        assert colors == ({0xff000000,0xff00ff00} if args.stage in ("mapping","overwrite") else set(before["effects"][0]["colors"])), colors
        if args.stage == "mapping":
            assert inspected["images"]["Active Map"] == {"AI":[[0,255,0,255]],"E":[[255,0,0,255]]}, inspected["images"]
        if args.stage == "close":
            windows = [row["detail"] for row in rows if row["action"]=="windows"]
            assert len(windows)==2 and all("Untitled" not in value and "Image Maps Verification" in value for value in windows), windows
        if args.stage == "overwrite":
            assert inspected["default"] == "Overwrite Me" and inspected["effects"][0]["mapping"] == "Overwrite Me", inspected
            assert inspected["images"]["Overwrite Me"] == {"AI":[[0,255,0,255]],"E":[[255,0,0,255]]}, inspected["images"]
            entries = ET.parse(profile / "SystemData/ModuleStore.xml").getroot()
            values = [el for el in entries.iter() if el.tag.rsplit("}",1)[-1] == "Value"]
            matching = [value for value in values if any(child.tag.rsplit("}",1)[-1]=="LibraryReferenceName" and child.text=="Overwrite Me" for child in value)]
            assert len(matching) == 1
            assert any(child.tag.rsplit("}",1)[-1]=="Notes" and child.text=="Original Active Map" for child in matching[0])
            folder = profile / "Module Data Files/LipSync/Overwrite Me"
            assert {p.name for p in folder.iterdir()} == {"AI.bmp","E.bmp"}, list(folder.iterdir())
            assert not (folder.parent / "Active Map").exists()
        timelines = [json.loads(row["detail"]) for row in rows if '"renderStatus"' in row.get("detail","")]
        assert timelines and all(t["renderStatus"]=={"waiting":0,"failed":0} and t["elements"][0]["rendered"] for t in timelines), timelines
        report.update(inspection=inspected, timelinePictures=[t["pictures"] for t in timelines], savedSha256=disk_before,
                      limits=["Table selection supplied through native table API; file panel uses queued private paths",
                              "Posted keys/menu actions do not establish physical dragging, external editor or global focus",
                              "Inspector proves persisted render output; timeline exposes completion/picture counts, not live effect generation/intent bytes"])
        (out / "result.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps({"stage":args.stage,"passed":True,"steps":len(rows),"report":str(out / "result.json")}))


if __name__ == "__main__":
    main()
