#!/usr/bin/env python3
"""Verify two real editor windows, their displayed playheads, and stale close/stop.

Pass a disposable profile and its sequence (at least 3 seconds long). The script
copies both, mutes output, opens two copies, and never changes the inputs.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def check(output: Path) -> None:
    rows = json.loads((output / "result.json").read_text())
    assert len(rows) == 47 and all(r["ok"] for r in rows), rows
    states = [json.loads(r["detail"]) for r in rows if r.get("detail", "").startswith("[{")]
    assert len(states) == 11, len(states)
    states = [{r["window"]: r for r in s} for s in states]
    for state in states[:8]:
        assert set(state) == {"Ruler", "Second"}, state
    expected = [("stopped", "stopped"), ("stopped", "stopped"),
                ("playing", "stopped"), ("playing", "stopped"), ("playing", "stopped"),
                ("stopped", "playing"), ("stopped", "playing"), ("stopped", "paused")]
    for state, pair in zip(states, expected):
        for name, transport in zip(("Ruler", "Second"), pair):
            observed = state[name]
            assert observed["transport"] == transport, (state, pair)
            if transport == "stopped":
                assert observed["positionMs"] == 0 and observed["playbackLineX"] == -1, observed
            else:
                assert observed["positionMs"] > 0 and observed["playbackLineX"] >= 0, observed
    assert states[2]["Ruler"]["positionMs"] < states[3]["Ruler"]["positionMs"] < states[4]["Ruler"]["positionMs"]
    assert states[5]["Second"]["positionMs"] < states[6]["Second"]["positionMs"]
    for state in states[8:]:
        assert set(state) == {"Second"}, state
    paused = states[7]["Second"]["positionMs"]
    assert states[8]["Second"]["transport"] == "paused" and states[8]["Second"]["positionMs"] == paused
    assert states[8]["Second"]["playbackLineX"] >= 0
    assert states[9]["Second"]["transport"] == "playing" and states[9]["Second"]["positionMs"] > paused
    assert states[10]["Second"]["transport"] == "stopped" and states[10]["Second"]["playbackLineX"] == -1
    print("PASS: 47 native steps; separate playheads, replacement, stale Stop, pause, old-editor close and resume")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("build/Vixen.app"))
    parser.add_argument("--profile", type=Path)
    parser.add_argument("--sequence", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    out = args.out.resolve()
    if args.check_only:
        check(out)
        return
    assert args.profile and args.sequence and not out.exists(), "Provide inputs and a fresh output directory"
    out.mkdir(parents=True)
    profile = out / "profile"
    shutil.copytree(args.profile, profile)
    (profile / ".lock").unlink(missing_ok=True)
    sequences = profile / "Sequence"
    sequences.mkdir(exist_ok=True)
    first, second = sequences / "Ruler.tim", sequences / "Second.tim"
    shutil.copyfile(args.sequence, first)
    shutil.copyfile(args.sequence, second)
    steps = [{"action": "window", "id": "administration"}, {"action": "wait", "seconds": 2}]
    for sequence in (first, second):
        steps += [{"action": "hook", "name": "admin.openSequence", "path": [str(sequence)]}, {"action": "wait", "seconds": 3}]
    steps += [{"action": "hook", "name": "timelineAllPlayback"}]
    def act(name, command):
        steps.extend([{"action": "hook", "name": "sequencer.activate", "path": [name]},
                      {"action": "hook", "name": "sequencer.transport", "path": [command]},
                      {"action": "wait", "seconds": .6}, {"action": "hook", "name": "timelineAllPlayback"}])
    for name, command in [("Ruler", "loop"), ("Ruler", "playPause"), ("Second", "stop"), ("Second", "loop"),
                          ("Second", "playPause"), ("Ruler", "stop"), ("Second", "playPause")]:
        act(name, command)
    steps += [{"action": "focus", "window": "Ruler"}, {"action": "closeWindow"}, {"action": "wait", "seconds": .6},
              {"action": "hook", "name": "timelineAllPlayback"}]
    act("Second", "playPause")
    act("Second", "stop")
    steps += [{"action": "quit"}]
    script = out / "steps.json"
    script.write_text(json.dumps({"steps": steps}, indent=2))
    env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
    env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(out), VIXEN_DATA=str(profile), VIXEN_MUTE="1")
    with (out / "process.log").open("w") as log:
        result = subprocess.run([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=45)
    assert result.returncode == 0, result.returncode
    check(out)


if __name__ == "__main__":
    main()
