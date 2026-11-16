#!/usr/bin/env python3
"""Open a private version0 sequence through the real native editor, muted.

Verifies that its reachable audio is copied before loading, while its original
XML/audio remain unchanged and the opened document remains unmodified. No input
profile, original show, sound device or external service is used.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import uuid
import wave
import xml.etree.ElementTree as ET


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=Path("build/Vixen.app"))
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--audio", type=Path, help="Optional private non-silent codec fixture to copy; otherwise generate PCM WAV")
    args = parser.parse_args()
    out = args.out.resolve()
    assert not out.exists(), f"Use a fresh output directory: {out}"
    out.mkdir(parents=True)
    profile = out / "profile"
    profile.mkdir()
    originals = out / "original files"
    originals.mkdir()
    audio = originals / ("Song & ü" + (args.audio.suffix if args.audio else ".wav"))
    fixture_hash = digest(args.audio) if args.audio else None
    if args.audio:
        audio.write_bytes(args.audio.read_bytes())
    else:
        with wave.open(str(audio), "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(8000)
            wav.writeframes(struct.pack("<h", 8192) * 16000)
    root = ET.Element("{http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed}TimedSequenceData", version="0")
    ET.SubElement(root, "ModuleInstanceId").text = str(uuid.uuid4())
    ET.SubElement(root, "ModuleTypeId").text = "296bdba2-9bf3-4bff-a9f2-13efac5c8ecb"
    ET.SubElement(root, "Length").text = "PT2S"
    ET.SubElement(root, "_dataModels")
    ET.SubElement(root, "_effectNodeSurrogates")
    media = ET.SubElement(ET.SubElement(root, "_mediaSurrogates"), "MediaSurrogate")
    ET.SubElement(media, "TypeId").text = "fe460392-3fab-4c63-99dd-d76c48354150"
    ET.SubElement(media, "InstanceId").text = str(uuid.uuid4())
    ET.SubElement(media, "FilePath").text = str(audio)
    timing = ET.SubElement(root, "_selectedTimingProviderSurrogate")
    ET.SubElement(timing, "ProviderType").text = "Media"
    ET.SubElement(timing, "SourceName").text = str(audio)
    sequence = originals / "Legacy.tim"
    ET.ElementTree(root).write(sequence, encoding="utf-8", xml_declaration=True)
    before = {str(path): digest(path) for path in (audio, sequence)}
    steps = [
        {"action": "window", "id": "administration"}, {"action": "wait", "seconds": 2},
        {"action": "hook", "name": "admin.openSequence", "path": [str(sequence)]},
        {"action": "wait", "seconds": 3}, {"action": "focus", "window": "Legacy"},
        {"action": "hook", "name": "sequencer.state"},
        {"action": "hook", "name": "sequencer.waveformState"},
        {"action": "dump", "name": "legacy"}, {"action": "snapshot", "name": "legacy"},
        {"action": "closeWindow"}, {"action": "wait", "seconds": .5},
        {"action": "quit"},
    ]
    script = out / "steps.json"
    script.write_text(json.dumps({"steps": steps}, indent=2))
    ui = out / "ui"
    ui.mkdir()
    env = {k: v for k, v in os.environ.items() if not k.startswith(("VIXEN_", "DYLD_"))}
    env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(ui), VIXEN_DATA=str(profile), VIXEN_MUTE="1")
    with (out / "process.log").open("w") as log:
        result = subprocess.run([str(args.app.resolve() / "Contents/MacOS/Vixen")], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=35)
    assert result.returncode == 0, result.returncode
    rows = json.loads((ui / "result.json").read_text())
    assert len(rows) == len(steps) - 1 and all(row["ok"] for row in rows), rows
    states = [row.get("detail", "") for row in rows if row.get("detail", "").startswith("effects=")]
    assert len(states) == 1 and "effects=0" in states[0] and "modified=false" in states[0], states
    waveforms = [json.loads(row["detail"]) for row in rows
                 if row.get("detail", "").startswith("{") and '"waveform"' in row["detail"]]
    assert len(waveforms) == 1, waveforms
    waveform = waveforms[0]
    assert not waveform["building"] and not waveform["failed"] and not waveform["blank"], waveform
    assert waveform["height"] > 0 and waveform["columns"] > 0 and waveform["peak"] > 0, waveform
    after = {path: digest(Path(path)) for path in before}
    assert before == after, "Opening changed an original fixture file"
    restored = profile / "Media" / audio.name
    assert digest(restored) == before[str(audio)], "Restored media differs from its source"
    assert sorted(path.name for path in restored.parent.iterdir()) == [audio.name]
    if args.audio:
        assert digest(args.audio) == fixture_hash, "The supplied audio fixture changed"
    report = {"passed": True, "steps": len(rows), "muted": True, "state": states[0],
              "waveform": waveform, "codecFixture": str(args.audio) if args.audio else "generated PCM WAV",
              "restored": str(restored), "originalSha256": before, "afterSha256": after,
              "limits": ["Local AppKit harness actions; no physical focus claim", "Audio loaded without playback"]}
    (out / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
