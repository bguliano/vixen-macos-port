#!/usr/bin/env python3
"""Validate a returned Windows capture before the existing diagnostic FSEQ comparison.

This does not alter production decoding, trim PCM, or claim a Windows execution
from manifest text alone. Retain the capture log and its environment provenance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess

INPUT_SHA256 = "c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3"
TIM_SHA256 = "0a75560184e31e25e752f6883b18d42e7428be346983012ca009dcd9b85eaad9"
FSEQ_SHA256 = "25f66c9343a9177094c598868ce39598ccbab89b824a0b101af2d5d8fc3ea902"


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def validate_binary_contract(manifest):
    """Validate the separate legacy provenance without implying a real decode."""
    contract = manifest.get("referenceBinaryContract")
    if contract is None:
        expected_source = json.loads((Path(__file__).parent / "reference.json").read_text())
        if manifest.get("referenceSourceContract") != expected_source:
            raise ValueError("Capture is missing a recognized source or binary contract")
        return  # This identifies the strict host contract, not arbitrary install equivalence.
    expected = json.loads((Path(__file__).parent / "legacy-net8/contract.json").read_text())
    for key in ("schema", "id", "vixenVersion", "audioPlayerAssemblyVersion", "productCommit", "runtimeMajor", "naudioAssemblyVersion", "inventoryArchiveSha256"):
        if contract.get(key) != expected[key]:
            raise ValueError("Unknown legacy binary contract: " + key)
    files = contract.get("files", [])
    pins = {row["name"]: row for row in expected["files"]}
    if len(files) != len(pins) or {row.get("name") for row in files} != set(pins):
        raise ValueError("Incomplete or duplicated legacy binary contract")
    for row in files:
        pin = pins[row["name"]]
        if row.get("bytes") != pin["bytes"] or row.get("sha256") != pin["sha256"]:
            raise ValueError("Legacy dependency contract mismatch: " + row["name"])
    runtime = manifest.get("runtime", {})
    if not runtime.get("framework", "").startswith(".NET 8.0.") or runtime.get("processArchitecture") != "X64":
        raise ValueError("Legacy capture requires the observed .NET8 x64 runtime")
    if not manifest.get("decoder", {}).get("type", "").startswith("NAudio.Wave.MediaFoundationReader,"):
        raise ValueError("Legacy capture did not report the audited original MP3 reader")
    loaded = {}
    for row in manifest.get("managedAssemblies", []):
        name = row.get("identity", "").split(",", 1)[0] + ".dll"
        if name in pins:
            if name in loaded:
                raise ValueError("Duplicate loaded legacy dependency: " + name)
            loaded[name] = row.get("file", {})
            if loaded[name].get("sha256") != pins[name]["sha256"] or loaded[name].get("bytes") != pins[name]["bytes"]:
                raise ValueError("Loaded legacy dependency differs from contract: " + name)
    if not {"AudioPlayer.dll", "NAudio.Core.dll", "NAudio.Wasapi.dll", "NLog.dll"}.issubset(loaded):
        raise ValueError("Missing loaded reader/decoder dependency provenance")


def validate(capture, input_path):
    capture = Path(capture).resolve()
    manifest = json.loads((capture / "manifest.json").read_text())
    if manifest.get("schema") != 1 or manifest.get("status") != "captured-on-windows":
        raise ValueError("Not a completed Windows capture manifest")
    if "windows" not in manifest.get("runtime", {}).get("os", "").lower():
        raise ValueError("Capture manifest does not identify a Windows runtime")
    validate_binary_contract(manifest)
    source = manifest["input"]
    if source["sha256"] != INPUT_SHA256 or digest(input_path) != INPUT_SHA256:
        raise ValueError("This comparison requires the exact Coldest Winter MP3")
    if source["bytes"] != Path(input_path).stat().st_size:
        raise ValueError("Source byte count mismatch")
    pcm = manifest["pcm"]
    if pcm["file"] != "cached.f32le":
        raise ValueError("Unexpected PCM filename")
    path = capture / "cached.f32le"
    if path.is_symlink() or not path.is_file():
        raise ValueError("PCM must be an ordinary capture file")
    fmt = pcm["format"]
    if (fmt["Encoding"], fmt["SampleRate"], fmt["Channels"], fmt["BitsPerSample"], fmt["BlockAlign"]) != ("IeeeFloat", 44100, 2, 32, 8):
        raise ValueError("The current Coldest Winter diagnostic requires stereo 44100 Hz float32 PCM")
    if type(pcm["frames"]) is not int or pcm["frames"] <= 0 or pcm["samples"] != pcm["frames"] * 2:
        raise ValueError("Invalid PCM sample/frame count")
    if pcm["bytes"] != pcm["samples"] * 4 or pcm["bytes"] != path.stat().st_size or pcm["bytes"] > 2**31:
        raise ValueError("PCM byte count mismatch or capture too large")
    if pcm["nonFiniteSamples"] != 0 or digest(path) != pcm["sha256"]:
        raise ValueError("PCM hash mismatch or nonfinite captured samples")
    decoder = manifest["decoder"]
    if decoder["separateReaderSamples"] != pcm["samples"] or decoder["separateReaderPcmSha256"] != pcm["sha256"]:
        raise ValueError("Separate format-probe decode does not match cached PCM")
    return manifest, path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True, help="Private copy containing the original Coldest Winter sequence, MP3 and FSEQ")
    parser.add_argument("--validate-only", action="store_true")
    parser.add_argument("--compare-program", type=Path, help="Already built vixen-session compare_fseq example")
    parser.add_argument("--out", type=Path, help="Fresh result directory; parent must exist")
    args = parser.parse_args()
    profile = args.profile.resolve()
    manifest, pcm = validate(args.capture, profile / "Media/Coldest Winter Pentatonix.mp3")
    for relative, expected in [("Sequence/Coldest Winter.tim", TIM_SHA256), ("Export/Coldest Winter.fseq", FSEQ_SHA256)]:
        if digest(profile / relative) != expected:
            raise ValueError(f"The diagnostic requires the unchanged {relative}")
    print("Validated capture integrity and matching input files; no Windows runtime inference from this local check.", flush=True)
    if args.validate_only:
        return
    if args.compare_program is None or args.out is None:
        parser.error("Comparison requires --compare-program and --out (or use --validate-only)")
    executable = args.compare_program.resolve(strict=True)
    context = {name: digest(profile / "SystemData" / name) for name in ("SystemConfig.xml", "ModuleStore.xml")}
    output = args.out.resolve()
    output.mkdir()  # Exclusive: never reuse an earlier report directory.
    command = [str(executable), str(profile), "10", "Coldest Winter"]
    env = {k: v for k, v in os.environ.items() if not k.startswith("AUDIO_") and k not in ("FSEQ_DIR", "LIT_RANGE", "SHOW_DETAILS")}
    env.update(AUDIO_RAW_F32=str(pcm), SHOW_DETAILS="1")
    with (output / "comparison.log").open("xb") as log:
        child = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=os.name != "nt")
        try:
            code = child.wait(timeout=600)
        except BaseException:
            if child.poll() is None:
                if os.name == "nt":
                    child.kill()
                else:
                    os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            raise
    report = {
        "captureManifest": str(args.capture.resolve() / "manifest.json"),
        "captureManifestSha256": digest(args.capture / "manifest.json"),
        "pcmSha256": manifest["pcm"]["sha256"], "inputSha256": INPUT_SHA256,
        "command": command, "compareProgramSha256": digest(executable), "exitCode": code,
        "renderContextSha256": context,
        "renderIntervalMs": 10, "exportIntervalMs": 25,
        "note": "No PCM trim/shift/quantization. Read comparison.log for measured differences; exit 0 alone does not establish matching FSEQ values.",
    }
    if any(digest(profile / "SystemData" / name) != checksum for name, checksum in context.items()):
        raise ValueError("Rendering context files changed during comparison; inspect the log, but do not accept this result")
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"Comparison exit {code}; inspect {output / 'comparison.log'} for actual nonrandom difference counts.")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
