#!/usr/bin/env python3
"""Independent assertions over the native direct-upload fixture's captures."""
from pathlib import Path
import hashlib
import json
import struct
import sys
import zipfile
import xml.etree.ElementTree as ET

root = Path(sys.argv[1] if len(sys.argv) > 1 else "target/direct-ui")
device = root / "device"
requests = [json.loads(line) for line in (device / "requests.jsonl").read_text().splitlines()]


def sequence(data, name):
    assert data[:4] == b"PSEQ" and data[7] == 2
    channels, frames = struct.unpack_from("<II", data, 10)
    assert channels > 0 and frames == 40 and data[18] == 25
    offset = struct.unpack_from("<H", data, 4)[0]
    variable = struct.unpack_from("<H", data, 8)[0]
    media = []
    while variable + 4 <= offset:
        size = struct.unpack_from("<H", data, variable)[0]
        if size < 4:
            break
        if data[variable + 2 : variable + 4] == b"mf":
            media.append(data[variable + 4 : variable + size].rstrip(b"\0").decode())
        variable += size
    assert media == [name + ".wav"], media


for request in requests:
    if request["file"]:
        data = (device / request["file"]).read_bytes()
        assert len(data) == request["size"] == request["expected"]
        assert hashlib.sha256(data).hexdigest() == request["sha256"]

fpp = [r for r in requests if r["mode"] == "fpp"]
assert len([r for r in fpp if r["path"].startswith("/api/file/sequences/")]) == 6
assert len([r for r in fpp if r["path"].startswith("/api/file/music/")]) == 6
assert len([r for r in fpp if "/rename/co-universes.json/" in r["path"]]) == 1
assert fpp[-1]["path"] == "/api/system/fppd/restart"
for i in range(1, 7):
    name = f"Device {i:02}"
    sequence((device / ("fpp-" + name + ".fseq")).read_bytes(), name)
    assert (device / ("fpp-" + name + ".wav")).read_bytes() == (root / "profile/Media/fixture.wav").read_bytes()
assert "universes" in json.loads((device / "fpp-co-universes.json").read_text())["channelOutputs"][0]

zip_requests = [r for r in requests if r["mode"] == "zip" and r["file"]]
assert len(zip_requests) == 2
for request, count in zip(zip_requests, [5, 1]):
    with zipfile.ZipFile(device / request["file"]) as archive:
        assert len(archive.namelist()) == count
        for filename in archive.namelist():
            sequence(archive.read(filename), Path(filename).stem)
zip_all = [r for r in requests if r["mode"] == "zip"]
assert [r["path"] for r in zip_all if r["method"] == "POST"][-1] == "/X6"
assert len([r for r in zip_all if r["path"] == "/X6"]) == 1
assert not any("/music/" in r["path"] or "/config/" in r["path"] for r in zip_all)

for mode in ["fail", "slow"]:
    sent = [r for r in requests if r["mode"] == mode and r["method"] == "POST"]
    assert len(sent) == 1 and "/sequences/Device%2001.fseq" in sent[0]["path"], sent

for name, final in [("fpp-out2", "Export Complete"), ("zip-out", "Export Complete"), ("fail-out", "Failed"), ("cancel-out", "Cancelled"), ("continue-out", "Export Complete"), ("stop-out", "Cancelled")]:
    steps = json.loads((root / name / "result.json").read_text())
    assert all(step["ok"] for step in steps), name
    assert any("final " + final in step.get("detail", "") for step in steps), (name, final)
module = ET.parse(root / "profile/SystemData/ModuleStore.xml")
saved = []
for element in module.iter():
    fields = {child.tag.rsplit("}", 1)[-1]: child.text for child in element}
    if fields.get("Name") == "Direct Loopback":
        saved.append(fields)
assert len(saved) == 1
for key, value in {"Format":"Falcon Player Sequence 2.6+", "FppDirectUpload":"true", "FppHostAddress":"127.0.0.1", "Interval":"25", "IncludeAudio":"true", "RenameAudio":"true", "CreateUniverseFile":"true", "BackupUniverseFile":"true"}.items():
    assert saved[0][key] == value, (key, saved[0][key])
print("PASS native FPP/ZIP bytes, routes, media names, failure, cancel, load-continue and load-stop")
