#!/usr/bin/env python3
"""Independent readback for the private normal-app Export Save-panel flow."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import unicodedata


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check_fseq(path, frames=60):
    # This fixture has no patched controllers. Check all bytes of the compressed
    # file structure/payload, including complete frame indexes and absence of
    # extra payload. The generated identifier at bytes24–31 is not deterministic.
    from compression import zstd
    data = Path(path).read_bytes()
    assert len(data) >= 60 and data[:4] == b"PSEQ"
    assert data[6:8] == b"\x00\x02", "Expected FSEQ 2.0"
    offset = struct.unpack_from("<H", data, 4)[0]
    fixed = struct.unpack_from("<H", data, 8)[0]
    channels, actual_frames = struct.unpack_from("<II", data, 10)
    assert channels == 0 and actual_frames == frames and data[18:20] == b"\x32\x00"
    assert data[20:24] == b"\x01\x02\x00\x00" and fixed == 48
    assert offset == 60 and data[fixed:offset] == b"\x0c\x00spVixen 3\x00"
    # Checkpoint values must retain their shape after JSON is written and read.
    blocks = [list(struct.unpack_from("<II", data, 32 + 8 * index)) for index in range(2)]
    assert [start for start, _ in blocks] == [0, 10]
    cursor = offset
    for _, size in blocks:
        assert size > 0 and cursor + size <= len(data)
        decoder = zstd.ZstdDecompressor()
        assert decoder.decompress(data[cursor:cursor + size], max_length=1) == b""
        assert decoder.eof and decoder.unused_data == b""
        cursor += size
    assert cursor == len(data), "Unexpected trailing FSEQ data"
    assert not list(Path(path).parent.glob("*.part")), "Partial export remains"
    return dict(format="FSEQ 2.0 zstd", frames=actual_frames, stepMs=50,
                channels=channels, blocks=blocks, decodedBytes=0,
                bytes=len(data), sha256=sha(path),
                scope="File-panel result and zero-channel structure/payload; variable identifier bytes24–31 excluded; no hardware or nonzero output claim")


def exact_names(actual, expected):
    # Cocoa/filesystem spellings may differ only by Unicode normalization.
    # Preserve strict case and reject multiple entries collapsing to one name.
    normalize = lambda value: unicodedata.normalize("NFC", value)
    actual_names = [normalize(value) for value in actual]
    expected_names = [normalize(value) for value in expected]
    assert len(actual_names) == len(set(actual_names)), "Ambiguous normalized output names"
    assert len(expected_names) == len(set(expected_names)), "Ambiguous expected output names"
    assert set(actual_names) == set(expected_names), "Unexpected export-folder contents"


def verify(prepared, stage):
    m = json.loads(Path(prepared).read_text())
    assert stage in ["cancelled", "exported", "default-exported"]
    for path, expected in m["sourceInputs"].items():
        assert sha(path) == expected, "Source fixture changed: " + path
    assert sha(m["openFile"]) == m["openFileSha256"], "Private sequence changed"
    assert sha(m["cancelSentinel"]) == m["cancelSentinelSha256"]
    output = Path(m["exportFile"])
    default_output = Path(m["defaultExportFile"])
    assert default_output.parent == Path(m["profile"]) / "Export"
    assert default_output.name == m["sequenceTitle"] + ".fseq"
    assert default_output.parent != output.parent
    expected = {Path(m["cancelSentinel"]).name}
    if stage != "cancelled":
        expected.add(output.name)
    exact_names([p.name for p in output.parent.iterdir()], expected)
    exact_names([p.name for p in default_output.parent.iterdir()],
                {default_output.name} if stage == "default-exported" else set())
    result = dict(passed=True, stage=stage, sourceInputsUnchanged=True,
                  sequenceUnchanged=True, cancelledSentinelUnchanged=True)
    if stage != "cancelled":
        result["fseq"] = check_fseq(output, m["expectedFrames"])
    else:
        assert not output.exists()
    if stage == "default-exported":
        result["defaultFseq"] = check_fseq(default_output, m["expectedFrames"])
        result["defaultFileName"] = default_output.name
    else:
        assert not default_output.exists()
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepared", required=True, type=Path)
    parser.add_argument("--stage", required=True, choices=["cancelled", "exported", "default-exported"])
    args = parser.parse_args()
    print(json.dumps(verify(args.prepared, args.stage)))
