#!/usr/bin/env python3
"""Bounded F62 Layer drag acceptance, using a private app bundle and profile.

Python 3.11+. Prepare-only never launches an app. Run only during approved,
exclusive desktop time. The AX helper sends actual OS pointer drags, checks
private PID/window ownership before each event, and restores the prior app.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import xml.etree.ElementTree as ET


def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def hashes(root): return {str(p.relative_to(root)): digest(p) for p in root.rglob("*") if p.is_file()}
def hook(name, *path): return {"action": "hook", "name": name, "path": list(path)}
def wait(seconds=.3): return {"action": "wait", "seconds": seconds}


def layer_xml(path):
    root = ET.parse(path).getroot()
    child = lambda node, name: next(c for c in node if c.tag.rsplit("}", 1)[-1] == name)
    values = lambda node: {c.tag.rsplit("}", 1)[-1]: c.text or "" for c in node}
    layers = [values(row) for row in child(child(root, "SequenceLayers"), "Layers")]
    surrogates = {values(row)["LayerReferenceId"].lower(): values(row) for row in child(root, "_layerMixingFilterSurrogates")}
    def canonical(node): return [node.tag, [[k,v] for k,v in sorted(node.attrib.items())], (node.text or "").strip(), [canonical(c) for c in node]]
    models = sorted((canonical(row) for row in child(root, "_layerMixingFilterDataModels")), key=str)
    return {"layers": layers, "surrogates": surrogates, "models": models}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", required=True, type=Path, help="fresh unique-bundle-ID test copy, never the ordinary app")
    parser.add_argument("--profile", required=True, type=Path, help="generated layer_drag_fixture profile")
    parser.add_argument("--helper", type=Path, default=Path("target/foreground-layer-drag"))
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--run-prepared", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    source, out, bundle = args.profile.resolve(), args.out.resolve(), args.app.resolve()
    app, helper = bundle / "Contents/MacOS/Vixen", args.helper.resolve()
    info = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
    clone_path = bundle.parent / "private-app.json"
    clone = json.loads(clone_path.read_text())
    assert clone["passed"] and clone["signatureVerified"] and clone["sourceUnchanged"]
    assert clone["app"] == str(bundle) and clone["bundleIdentifier"] == info["CFBundleIdentifier"]
    assert info["CFBundleIdentifier"].startswith("com.vixenlights.verification.")
    assert digest(app) == clone["executableSha256"], "Use prepare_private_test_app.py first"
    assert "target" in source.parts and "target" in out.parts and not out.is_relative_to(source)
    assert app.is_file() and helper.is_file()
    if args.run_prepared or args.verify_only:
        manifest = json.loads((out / "prepared.json").read_text())
        assert manifest["source"] == str(source) and manifest["app"] == str(app)
        assert digest(app) == manifest["appSha256"] and digest(helper) == manifest["helperSha256"]
    else:
        assert not out.exists(), "Use a fresh output folder"
        fixture = json.loads((source / "fixture.json").read_text())
        assert fixture["controllers"] == 0 and fixture["muted"]
        original = Path(fixture["sequence"]).resolve()
        assert original.is_relative_to(source)
        assert [r["name"] for r in fixture["layers"]] == ["Zebra Layer", "Alpha Layer", "Middle Layer", "Default"]
        out.mkdir(parents=True)
        profile = out / "profile"
        shutil.copytree(source, profile)
        (profile / ".lock").unlink(missing_ok=True)
        sequence = profile / original.relative_to(source)
        steps = [{"action": "window", "id": "administration"}, wait(2),
                 hook("admin.openSequence", str(sequence)), wait(3),
                 {"action": "focus", "window": "Layer Drag Verification"},
                 hook("sequencer.activate", "Layer Drag Verification"), hook("sequencer.resize", "1280", "800"),
                 hook("sequencer.panel", "Layers"), wait(.5)]
        def observe(name, pause):
            return [hook("sequencer.layers", "state"), hook("sequencer.state"), hook("sequencer.keyboardState"),
                    {"action": "snapshot", "name": name}, wait(pause)]
        for name in ["ready", "same-row", "default-source", "outside", "move-up"]:
            steps += observe(name, 7)
        steps += observe("above-default", .5)
        steps += [{"action": "menu", "path": ["Sequence", "Save"]}, wait(.7)] + observe("saved", .3)
        steps += [{"action": "closeWindow"}, wait(.7), hook("admin.openSequence", str(sequence)), wait(2),
                  {"action": "focus", "window": "Layer Drag Verification"}, hook("sequencer.activate", "Layer Drag Verification"),
                  hook("sequencer.panel", "Layers"), wait(.5)] + observe("reopened", .2) + [{"action": "quit"}]
        (out / "steps.json").write_text(json.dumps({"steps": steps}, indent=2) + "\n")
        manifest = {"app":str(app), "appSha256":digest(app), "bundleIdentifier":info["CFBundleIdentifier"],
                    "privateApp":clone, "privateAppManifestSha256":digest(clone_path),
                    "helperSha256":digest(helper), "source":str(source), "sourceHashes":hashes(source),
                    "fixture":fixture, "sequence":str(sequence), "originalXML":layer_xml(original),
                    "steps":len(steps)-1, "stepsSha256":digest(out / "steps.json"),
                    "scope":"five real OS drags; actual private key/main window; UUID/filter/level save and reopen; no new app hooks"}
        (out / "prepared.json").write_text(json.dumps(manifest, indent=2) + "\n")
    assert digest(out / "steps.json") == manifest["stepsSha256"]
    assert digest(clone_path) == manifest["privateAppManifestSha256"]
    if args.prepare_only:
        print(out / "prepared.json")
        return
    profile, ui, runtime = out / "profile", out / "ui", out / "runtime"
    sequence = Path(manifest["sequence"])
    fallback = None
    if args.verify_only:
        previous = json.loads((out / "result.json").read_text())
        if not (out / "result.initial.json").exists(): shutil.copyfile(out / "result.json", out / "result.initial.json")
        return_code = previous["exitCode"]
    else:
        assert not ui.exists() and not runtime.exists(), "Use a fresh prepared run"
        ui.mkdir(); runtime.mkdir()
        env = {k:v for k,v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
        env.update(VIXEN_DATA=str(profile), VIXEN_MUTE="1", VIXEN_UI_TEST=str(out / "steps.json"),
                   VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1", TMPDIR=str(runtime)+"/")
        driver = None
        with (out / "process.log").open("w") as log, (out / "foreground.log").open("w") as driver_log:
            process = subprocess.Popen([str(app)], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                driver = subprocess.Popen([str(helper), str(process.pid), str(out), str(app)], stdout=driver_log, stderr=subprocess.STDOUT)
                process.wait(timeout=95)
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                if driver is not None:
                    try: driver.wait(timeout=12)
                    except subprocess.TimeoutExpired:
                        driver.kill(); driver.wait()
                        start = out / "foreground-start.json"
                        if start.exists():
                            prior = json.loads(start.read_text())["previousPID"]
                            if prior > 0: fallback = subprocess.run([str(helper), "--restore", str(prior)], capture_output=True, text=True, timeout=5)
        return_code = process.returncode
    rows = json.loads((ui / "result.json").read_text())
    foreground = json.loads((out / "foreground.json").read_text()) if (out / "foreground.json").exists() else {}
    report = {"passed":False, "exitCode":return_code, "steps":len(rows), "appSha256":digest(app),
              "foreground":foreground, "fallbackRestore":None if fallback is None else {"returnCode":fallback.returncode,"output":fallback.stdout}}
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    assert return_code == 0 and len(rows) == manifest["steps"] and all(r["ok"] for r in rows), [r for r in rows if not r["ok"]]
    assert foreground.get("passed") and foreground.get("restoredPrevious") and foreground.get("targetExited"), foreground
    assert len(foreground["drags"]) == 5 and all(r["observedOrder"] == r["expectedOrder"] for r in foreground["drags"])
    observed = [r["detail"] for r in rows if r.get("detail", "").startswith(("Zebra Layer:", "Middle Layer:"))]
    names = lambda value: [row.split(":")[0] for row in value.split(" | ")]
    initial = ["Zebra Layer", "Alpha Layer", "Middle Layer", "Default"]
    final = ["Middle Layer", "Alpha Layer", "Zebra Layer", "Default"]
    assert [names(value) for value in observed] == [initial]*4 + [["Middle Layer","Zebra Layer","Alpha Layer","Default"]] + [final]*3, observed
    states = [r["detail"] for r in rows if r.get("detail", "").startswith("effects=")]
    assert len(states) == 8 and all("undo=-" in s and "effects=0 " in s for s in states), states
    assert ["modified=true" in s for s in states] == [False]*4 + [True, True, False, False], states
    transport = [r["detail"] for r in rows if r.get("detail", "").startswith("running=")]
    assert len(transport) == 8 and all("running=false paused=false" in s for s in transport), transport
    saved = layer_xml(sequence)
    original = manifest["originalXML"]
    by_name = {layer["LayerName"]:layer for layer in original["layers"]}
    assert [row["LayerName"] for row in saved["layers"]] == final
    for level, row in zip([3,2,1,0], saved["layers"]):
        expected = dict(by_name[row["LayerName"]], LayerLevel=str(level))
        assert row == expected, (row, expected)
    assert saved["surrogates"] == original["surrogates"] and saved["models"] == original["models"], saved
    assert hashes(source) == manifest["sourceHashes"]
    assert digest(app) == manifest["appSha256"] and digest(helper) == manifest["helperSha256"]
    report.update(passed=True, sourceUnchanged=True, states=states, layerStates=observed, savedXML=saved)
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed":True, "steps":len(rows), "report":str(out / "result.json")}))


if __name__ == "__main__": main()
