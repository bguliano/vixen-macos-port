#!/usr/bin/env python3
"""Local build/error and capture-validator checks. Never a Windows decoder test."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dotnet", type=Path, required=True)
    parser.add_argument("--capture-dll", type=Path, required=True)
    args = parser.parse_args()
    command = [str(args.dotnet.resolve()), str(args.capture_dll.resolve())]
    spec = importlib.util.spec_from_file_location("capture", Path(__file__).with_name("compare_capture.py"))
    capture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(capture)
    checks = []

    def run(name, tail, status, contains):
        result = subprocess.run(command + tail, text=True, capture_output=True, timeout=10)
        assert result.returncode == status, (name, result)
        assert contains in result.stdout + result.stderr, (name, result)
        checks.append(name)

    run("help", ["--help"], 0, "Requires Windows")
    run("missing arguments", [], 1, "--vixen-bin")
    run("unknown argument", ["--unknown", "x"], 1, "ArgumentException")
    run("duplicate argument", ["--input", "a", "--input", "b"], 1, "ArgumentException")
    with tempfile.TemporaryDirectory(prefix="vixen-pcm-check-") as temporary:
        root = Path(temporary)
        install = root / "install"
        install.mkdir()
        mp3 = root / "synthetic.mp3"
        mp3.write_bytes(b"not decoded: local negative-path fixture")
        source_hash = hashlib.sha256(mp3.read_bytes()).hexdigest()
        out = root / "fresh-output"
        tail = ["--vixen-bin", str(install), "--input", str(mp3), "--out", str(out)]
        run("non-Windows capture rejected", tail, 1, "Actual capture requires Windows")
        assert not out.exists() and hashlib.sha256(mp3.read_bytes()).hexdigest() == source_hash
        out.mkdir()
        run("existing output rejected", tail, 1, "Output already exists")
        # These bytes only test validation; they are explicitly not decoder evidence.
        samples = struct.pack("<4f", .25, -.25, .5, -.5)
        (out / "cached.f32le").write_bytes(samples)
        manifest = {"schema": 1, "status": "captured-on-windows", "runtime": {"os": "Windows synthetic validator fixture"},
            "input": {"sha256": source_hash, "bytes": mp3.stat().st_size},
            "decoder": {"separateReaderSamples": 4, "separateReaderPcmSha256": hashlib.sha256(samples).hexdigest()},
            "pcm": {"file": "cached.f32le", "sha256": hashlib.sha256(samples).hexdigest(), "bytes": len(samples), "samples": 4, "frames": 2,
                "nonFiniteSamples": 0, "format": {"Encoding": "IeeeFloat", "SampleRate": 44100, "Channels": 2, "BitsPerSample": 32, "BlockAlign": 8}}}
        capture.INPUT_SHA256 = source_hash
        def write():
            (out / "manifest.json").write_text(json.dumps(manifest))
        def rejected(name):
            try:
                capture.validate(out, mp3)
            except ValueError:
                checks.append(name)
            else:
                raise AssertionError(name)
        write()
        capture.validate(out, mp3)
        checks.append("synthetic validator positive")
        (out / "cached.f32le").write_bytes(bytes(len(samples)))
        rejected("changed PCM rejected")
        (out / "cached.f32le").write_bytes(samples)
        manifest["pcm"]["frames"] = 3
        write()
        rejected("incomplete frame count rejected")
        manifest["pcm"]["frames"] = 2
        manifest["pcm"]["format"]["SampleRate"] = 48000
        write()
        rejected("wrong sample rate rejected")
        manifest["pcm"]["format"]["SampleRate"] = 44100
        manifest["decoder"]["separateReaderSamples"] = 5
        write()
        rejected("separate reader count mismatch rejected")
        manifest["decoder"]["separateReaderSamples"] = 4
        manifest["decoder"]["separateReaderPcmSha256"] = "0" * 64
        write()
        rejected("separate reader hash mismatch rejected")
        manifest["decoder"]["separateReaderPcmSha256"] = hashlib.sha256(samples).hexdigest()
        manifest["runtime"]["os"] = "macOS"
        write()
        rejected("non-Windows manifest rejected")
    print(json.dumps({"scope": "Local argument/error paths and synthetic integrity validation only; Windows decoder unverified", "passed": checks}, indent=2))


if __name__ == "__main__":
    main()
