#!/usr/bin/env python3
"""Single Export Sequence sheet: actual Start/Stop/alert, muted transport observations.

Use export_sheet_fixture to create the private controller-free profile first.
The existing harness supplies only the save destination and transport commands;
sheet controls and sequencer Export menu use their normal UI actions.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import struct
import subprocess

from test_video_cache_ui import wait, hook, capture


def check_fseq(path):
    # Independent readback of the normal first-launch Falcon 2.6+ output.
    # There are no patched channels; frame count is a header contract, and each
    # complete zstd block must decode to exactly zero channel bytes.
    from compression import zstd
    data = path.read_bytes()
    assert len(data) >= 48 and data[:4] == b"PSEQ" and data[6:8] == b"\x00\x02"
    offset, fixed, channels, frames = (struct.unpack_from("<H", data, 4)[0],
        struct.unpack_from("<H", data, 8)[0], struct.unpack_from("<I", data, 10)[0],
        struct.unpack_from("<I", data, 14)[0])
    assert channels == 0 and frames == 600 and data[18] == 50 and data[19] == 0
    assert data[20:24] == b"\x01\x02\x00\x00" and fixed == 48 and offset <= len(data)
    blocks = [struct.unpack_from("<II", data, 32 + 8 * index) for index in range(2)]
    assert [first for first, _ in blocks] == [0, 10]
    cursor = offset
    for _, size in blocks:
        assert size > 0 and cursor + size <= len(data)
        decoder = zstd.ZstdDecompressor()
        assert decoder.decompress(data[cursor:cursor + size], max_length=1) == b""
        assert decoder.eof and decoder.unused_data == b""
        cursor += size
    assert cursor == len(data)
    assert data[fixed:offset] == b"\x0c\x00spVixen 3\x00"
    return {"format": "FSEQ 2.0 zstd", "framesDeclared": frames, "stepMs": data[18],
            "channels": channels, "blocks": blocks, "decodedBytes": 0,
            "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("target/build-release/Vixen.app"))
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--check-only", action="store_true", help="Recheck retained native evidence without launching or rewriting inputs")
    args = parser.parse_args()
    profile = args.profile.resolve()
    assert "target" in profile.parts and not args.profile.is_symlink()
    fixture = json.loads((profile / "fixture.json").read_text())
    assert fixture["controllers"] == 0 and fixture["channels"] == 32 and fixture["muted"]
    paths = {key: Path(fixture[key]).resolve() for key in ("long", "short", "collision")}
    assert all(path.is_relative_to(profile) and path.is_file() for path in paths.values())
    hashes = {key: hashlib.sha256(path.read_bytes()).hexdigest() for key, path in paths.items()}
    out = args.out.resolve()
    assert "target" in out.parts and (out.is_dir() if args.check_only else not out.exists())
    assert not (args.prepare_only and args.check_only)
    out.mkdir(parents=True, exist_ok=args.check_only)
    sentinel = b"earlier private export must survive Stop\n"
    stopped = profile / "stopped.csv"
    if not args.check_only and not args.prepare_only:
        assert not stopped.exists()
        stopped.write_bytes(sentinel)
    destinations = {"01-stop-resume": stopped, "02-failure-resume": paths["collision"] / "failed.csv",
                    "03-complete-resume": profile / "complete.csv"}
    report = {"passed": False, "profile": str(profile), "runs": {}, "muted": True,
              "limits": ["No actual NSSavePanel: guarded VIXEN_TEST_FILE supplies a private destination",
                         "Transport command hook and read-only engine/playhead observations; posted sheet/menu controls",
                         "Controller-free export has zero patched channels; this checks sheet lifecycle, not hardware/channel values"]}
    app_hash = hashlib.sha256((args.app.resolve() / "Contents/MacOS/Vixen").read_bytes()).hexdigest()
    baseline_path = out / "baseline.json"
    if args.check_only and baseline_path.exists():
        baseline = json.loads(baseline_path.read_text())
        assert baseline == {"inputHashes": hashes, "appSha256": app_hash}
    elif not args.check_only:
        baseline_path.write_text(json.dumps({"inputHashes": hashes, "appSha256": app_hash}, indent=2) + "\n")
    report.update(inputHashes=hashes, baselineManifest=baseline_path.exists(), appSha256=app_hash)
    for index, (name, destination) in enumerate(destinations.items()):
        sequence = paths["short"] if index == 2 else paths["long"]
        title = sequence.stem
        steps = [{"action": "window", "id": "administration"}, wait(2),
                 hook("admin.openSequence", str(sequence)), wait(3),
                 {"action": "focus", "window": title}, hook("sequencer.activate", title),
                 hook("sequencer.resize", "1400", "900"), wait(), hook("timelineState"),
                 hook("sequencer.transport", "playPause"), wait(.6), hook("timelinePlayback"),
                 {"action": "menu", "path": ["Sequence", "Export"]}, wait(.7),
                 hook("timelinePlayback"), hook("exportStatus")]
        if index == 0:
            steps += [{"action": "popup", "name": "Falcon Player Sequence 2.6+", "label": "CSV File"}, wait(.3)]
        # The first visible Start is the Network table's column heading.
        steps += capture("ready") + [{"action": "click", "label": "Start", "index": 1}, wait(.2)]
        if index == 0:
            steps += [hook("exportStatus"), hook("timelinePlayback"), wait(.6), hook("timelinePlayback")]
            steps += capture("paused") + [{"action": "click", "label": "Stop"}]
        steps += [hook("exportWait", "15"), wait(.4), hook("timelinePlayback")]
        steps += capture("finished")
        if index == 1:
            steps += [{"action": "click", "label": "OK"}, wait(.3)] + capture("dismissed")
        steps += [wait(.6), hook("timelinePlayback"), hook("exportStatus"),
                  {"action": "click", "label": "Close"}, wait(.4),
                  hook("sequencer.transport", "stop"), wait(.3), hook("timelinePlayback"),
                  {"action": "closeWindow"}, wait(.3), {"action": "quit"}]
        run = out / name
        run.mkdir(exist_ok=args.check_only)
        script = run / "steps.json"
        if args.check_only:
            assert json.loads(script.read_text()) == {"steps": steps}, "Retained native steps must match"
        else:
            script.write_text(json.dumps({"steps": steps}, indent=2) + "\n")
        if args.prepare_only:
            continue
        ui = run / "ui"
        ui.mkdir(exist_ok=args.check_only)
        env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
        env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_UI_TEST_TRACE="1",
                   VIXEN_DATA=str(profile), VIXEN_MUTE="1", VIXEN_TEST_FILE=str(destination))
        if args.check_only:
            exit_code = json.loads((run / "process.json").read_text())["exitCode"]
        else:
            with (run / "process.log").open("w") as log:
                process = subprocess.Popen([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                           stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                try:
                    process.wait(timeout=65)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                    raise
            exit_code = process.returncode
            (run / "process.json").write_text(json.dumps({"exitCode": exit_code}) + "\n")
        rows = json.loads((ui / "result.json").read_text())
        assert exit_code == 0 and len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
        playback = [json.loads(row["detail"]) for row in rows if row.get("detail", "").startswith('{"cursor"')]
        expected = ["playing", "playing"] + (["paused", "paused"] if index == 0 else []) + ["playing", "playing", "stopped"]
        assert [state["transport"] for state in playback] == expected, playback
        assert playback[1]["positionMs"] > playback[0]["positionMs"] > 0
        if index == 0:
            assert playback[2]["positionMs"] == playback[3]["positionMs"], playback
            assert playback[4]["positionMs"] > playback[3]["positionMs"], playback
        assert playback[-2]["positionMs"] > playback[-3]["positionMs"], playback
        assert playback[-1]["playbackLineX"] == -1
        statuses = [row["detail"] for row in rows if row.get("detail", "").startswith(("idle|", "working|"))]
        outcome = ["Export Canceled", "Export Failed", "Export Complete"][index]
        assert statuses[-1].startswith("idle|" + outcome + "|"), statuses
        if index == 0:
            assert any(status.startswith("working|") for status in statuses), statuses
            assert stopped.read_bytes() == sentinel
        elif index == 1:
            alert = (ui / "finished.txt").read_text()
            assert '"Export Failed"' in alert and '"OK"' in alert and "Not a directory" in alert, alert
            dismissed = (ui / "dismissed.txt").read_text()
            assert "Window: Export Sequence" in dismissed and '"OK"' not in dismissed, dismissed
        else:
            publication = check_fseq(destination)
        assert not destination.with_name(destination.name + ".part").exists()
        assert all(hashlib.sha256(path.read_bytes()).hexdigest() == hashes[key] for key, path in paths.items())
        record = {"steps": len(rows), "playback": playback, "statuses": statuses, "outcome": outcome}
        if index == 2:
            record["publication"] = publication
        report["runs"][name] = record
        (run / "evidence.json").write_text(json.dumps(record, indent=2) + "\n")
    if args.prepare_only:
        print(f"Prepared: {out}")
        return
    report.update(passed=True)
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": True, "steps": sum(run["steps"] for run in report["runs"].values()), "out": str(out)}))


if __name__ == "__main__":
    main()
