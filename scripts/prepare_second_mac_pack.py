#!/usr/bin/env python3
"""Stage the audited app and private fixtures for another Apple-silicon Mac. No app launch."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import stat

from check_app_bundle import audit
from prepare_private_test_app import code_digest

REPO = Path(__file__).resolve().parents[1]
SCRIPTS = ["run_second_mac_pack.py", "prepare_private_test_app.py", "check_app_bundle.py", "test_bundle_relocation.py",
           "test_native_launch_helper.py", "test_legacy_media_open.py", "test_editor_ownership.py",
           "test_profile_lock.py", "test_video_metadata_ui.py", "test_video_cache_ui.py",
           "video_fixture_paths.py", "read_ui_snapshot.swift"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def candidate_evidence(app, validation_path):
    """Require the complete reviewed build record, never a native-success alias."""
    assert validation_path.is_relative_to(REPO / "target")
    validation = json.loads(validation_path.read_text())
    assert validation["passed"] and validation["promoted"] is False
    assert validation["trackedInputCount"] == 811
    source_path = Path(validation["sourceManifest"])
    assert source_path.is_relative_to(REPO / "target")
    assert digest(source_path) == validation["sourceManifestSha256"]
    sources = json.loads(source_path.read_text())["sources"]
    assert len(sources) == 811
    release = validation["builds"]["release"]
    assert app == Path(release["app"]).resolve()
    expected = release["executableSha256"]
    assert digest(app / "Contents/MacOS/Vixen") == expected
    records = {"candidate-build-validation.json": validation_path,
               "candidate-build-inputs.json": source_path}
    for kind in ("release", "debug"):
        build = validation["builds"][kind]
        audit_path = Path(build["audit"])
        assert audit_path.is_relative_to(REPO / "target")
        # Newer build records retain the terminal result with the full command.
        terminal_exit = build.get("terminalExitCode", build.get("command", {}).get("exitCode"))
        assert terminal_exit == 0 and digest(audit_path) == build["auditSha256"]
        checked = json.loads(audit_path.read_text())
        assert checked["ok"] and checked["errors"] == [] and Path(checked["bundle"]) == Path(build["app"])
        assert digest(Path(build["app"]) / "Contents/MacOS/Vixen") == build["executableSha256"]
        records[f"candidate-{kind}-audit.json"] = audit_path
    clone = validation["private"]
    assert clone["passed"] and clone["signatureVerified"] and clone["sourceUnchanged"]
    assert Path(clone["sourceApp"]) == app and clone["sourceExecutableSha256"] == expected
    private = Path(clone["app"])
    assert private.is_relative_to(REPO / "target")
    assert json.loads((private.parent / "private-app.json").read_text()) == clone
    assert digest(private / "Contents/MacOS/Vixen") == clone["executableSha256"]
    assert code_digest(app / "Contents/MacOS/Vixen") == code_digest(private / "Contents/MacOS/Vixen") == clone["executableCodeDigest"]
    for name, expected_hash in sources.items():
        source = (REPO / name).resolve()
        assert source.is_relative_to(REPO) and digest(source) == expected_hash, f"Candidate source drift: {name}"
    pending = ["Physical cursor and Prop workflows", "Preview linker and Image Maps gestures",
               "Native package file-panel workflow", "Actual macOS 26 verification"]
    evidence = {"buildStatus": "candidate", "promoted": False,
                "sourceCheckpoint": "Audited candidate build; current physical and macOS 26 acceptance remain pending. No historical native result is attributed to this executable.",
                "currentNativeCheckpoint": {"passed": False, "status": "pending", "appSha256": expected},
                "pendingVerification": pending,
                "candidateValidation": {"sha256": digest(validation_path), "sourceManifestSha256": digest(source_path),
                                        "trackedInputs": 811, "codeDigest": clone["executableCodeDigest"]}}
    return expected, evidence, records


def reusable_support(pack):
    manifest_path = pack / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    assert digest(pack / "scripts/read_ui_snapshot.swift") == manifest["files"]["scripts/read_ui_snapshot.swift"] == digest(REPO / "scripts/read_ui_snapshot.swift")
    names = {name: value for name, value in manifest["files"].items()
             if name == "helpers/read-ui-snapshot" or name.startswith("fixtures/")}
    actual = {path.relative_to(pack).as_posix() for path in (pack / "fixtures").rglob("*") if path.is_file()}
    assert set(names) == actual | {"helpers/read-ui-snapshot"}
    for name, expected in names.items():
        path = pack / name
        assert not path.is_symlink() and path.resolve().is_relative_to(pack) and digest(path) == expected
    assert names["helpers/read-ui-snapshot"] == manifest["ocr"]["sha256"]
    for name in ("playback", "metadata"):
        profile = pack / "fixtures" / name
        fixture = json.loads((profile / "fixture.json").read_text())
        assert fixture["controllers"] == 0 and fixture["muted"] and fixture["portable"]
        assert fixture["profile"] == "." and fixture["metadata"] == (name == "metadata")
        for relative in [fixture["sequence"], fixture["video"], *fixture["choices"]]:
            assert not Path(relative).is_absolute() and (profile / relative).resolve().is_relative_to(profile)
            assert (profile / relative).is_file()
    return {"pack": str(pack), "manifestSha256": digest(manifest_path), "files": names}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=REPO / "target/build-release/Vixen.app")
    parser.add_argument("--generator", type=Path, default=REPO / "target/build-main/rust/debug/examples/video_cache_fixture")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--candidate-validation", type=Path, help="Explicit unpromoted candidate build-validation.json; never uses accepted native evidence")
    parser.add_argument("--reuse-support", type=Path, help="Reuse pinned OCR helper and portable media fixtures from an existing pack")
    args = parser.parse_args()
    app, out = args.app.resolve(), args.out.resolve()
    assert not out.exists() and out.is_relative_to(REPO / "target"), "Use a fresh staging directory under target"
    records = {}
    if args.candidate_validation:
        expected, evidence, records = candidate_evidence(app, args.candidate_validation.resolve())
    else:
        checkpoint = json.loads((REPO / "target/completion-checkpoint.json").read_text())
        expected = checkpoint["bundleExecutableSha256"]["release"]
        current_native = checkpoint["currentNativeCheckpoint"]
        assert current_native["passed"] and current_native["appSha256"] == expected, "Current native checkpoint differs from accepted release"
        assert digest(app / "Contents/MacOS/Vixen") == expected, "App differs from audited release checkpoint"
        native_baseline_sha = checkpoint.get("nativeCheckpointAppSha256", expected)
        evidence = {"buildStatus": "accepted-release", "promoted": True,
                    "sourceCheckpoint": f"{checkpoint['workspace']['passed']} workspace; current native evidence on app SHA256 {expected} is scoped in currentNativeCheckpoint; historical native baseline {checkpoint['nativeStrictSteps']} steps on app SHA256 {native_baseline_sha}",
                    "currentNativeCheckpoint": current_native,
                    "nativeBaseline": {"historical": True, "steps": checkpoint["nativeStrictSteps"], "appSha256": native_baseline_sha,
                                       "scope": "Historical baseline only; current release evidence is recorded separately in currentNativeCheckpoint."}}
    reused = reusable_support(args.reuse_support.resolve()) if args.reuse_support else None
    out.mkdir(parents=True)
    shutil.copytree(app, out / "Vixen.app")
    assert digest(out / "Vixen.app/Contents/MacOS/Vixen") == expected
    shutil.copytree(REPO / "app/Resources", out / "app/Resources")
    shutil.copyfile(REPO / "Cargo.toml", out / "Cargo.toml")
    (out / "scripts").mkdir()
    for name in SCRIPTS:
        shutil.copyfile(REPO / "scripts" / name, out / "scripts" / name)
    audio = out / "crates/vixen-audio/tests/assets"
    audio.mkdir(parents=True)
    shutil.copyfile(REPO / "crates/vixen-audio/tests/assets/tone.wma", audio / "tone.wma")
    checked = audit(out / "Vixen.app", out / "app/Resources", "arm64")
    assert checked["ok"], checked["errors"]
    (out / "bundle-audit.json").write_text(json.dumps(checked, indent=2) + "\n")
    for name, source in records.items():
        shutil.copy2(source, out / name)

    helper = out / "helpers/read-ui-snapshot"
    helper.parent.mkdir()
    build_cache = out / ".swift-cache"
    if reused:
        shutil.copy2(Path(reused["pack"]) / "helpers/read-ui-snapshot", helper)
    else:
        subprocess.run(["/usr/bin/xcrun", "swiftc", "-target", "arm64-apple-macos26.0",
                        "-module-cache-path", str(build_cache), str(REPO / "scripts/read_ui_snapshot.swift"),
                        "-o", str(helper)], check=True, timeout=90)
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", str(helper)], check=True)
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", str(helper)], check=True)
    arch = subprocess.check_output(["/usr/bin/lipo", "-archs", str(helper)], text=True).strip()
    loads = subprocess.check_output(["/usr/bin/otool", "-l", str(helper)], text=True)
    versions = re.findall(r"cmd LC_BUILD_VERSION\s+cmdsize \d+\s+platform (\d+)\s+minos ([\d.]+)", loads)
    assert arch == "arm64" and versions == [("1", "26.0")], (arch, versions)
    dependencies = [line.strip().split(" (compatibility version")[0]
                    for line in subprocess.check_output(["/usr/bin/otool", "-L", str(helper)], text=True).splitlines()[1:]]
    assert all(path.startswith(("/usr/lib/", "/System/Library/")) for path in dependencies), dependencies
    if build_cache.exists():
        shutil.rmtree(build_cache)

    fixtures = out / "fixtures"
    if reused:
        shutil.copytree(Path(reused["pack"]) / "fixtures", fixtures)
        assert reusable_support(Path(reused["pack"])) == reused
        assert all(digest(out / name) == value for name, value in reused["files"].items())
    else:
        fixtures.mkdir()
    for name, metadata in ([] if reused else [("playback", False), ("metadata", True)]):
        profile = fixtures / name
        command = [str(args.generator.resolve()), str(profile)] + (["--metadata"] if metadata else [])
        subprocess.run(command, cwd=REPO, env={**os.environ, "VIXEN_MUTE": "1"},
                       check=True, stdout=subprocess.DEVNULL, timeout=30)
        fixture = json.loads((profile / "fixture.json").read_text())
        assert fixture["controllers"] == 0 and fixture["muted"]
        for key in ("sequence", "video"):
            fixture[key] = Path(fixture[key]).resolve().relative_to(profile).as_posix()
        fixture["choices"] = [Path(path).resolve().relative_to(profile).as_posix() for path in fixture["choices"]]
        fixture.update(portable=True, profile=".", source=fixture["video"])
        del fixture["cache"]
        (profile / "fixture.json").write_text(json.dumps(fixture, indent=2) + "\n")
        if not metadata:
            sequence = profile / fixture["sequence"]
            text = sequence.read_text()
            text, changed = re.subn(r'(<Length(?: xmlns="")?>)PT2S(</Length>)', r'\g<1>PT60S\2', text, count=1)
            assert changed == 1, "Expected the generated two-second sequence length"
            sequence.write_text(text)
    shutil.copyfile(REPO / "docs/SECOND_MAC_CHECKS.md", out / "README.md")
    if args.candidate_validation:
        readme = out / "README.md"
        readme.write_text("# Unpromoted candidate verification\n\nThis pack contains candidate " + expected + ". It is not the accepted release. Physical cursor, Prop, Preview linker, Image Maps and native package file-panel acceptance are pending; macOS 26 has not yet been verified for this executable. A successful run establishes only the workflows and actual OS recorded in its result.\n\n" + readme.read_text())
        assert candidate_evidence(app, args.candidate_validation.resolve()) == (expected, evidence, records)
    assert digest(app / "Contents/MacOS/Vixen") == expected, "Source bundle changed during staging"
    files = {path.relative_to(out).as_posix(): digest(path) for path in sorted(out.rglob("*")) if path.is_file()}
    manifest = {"format": 1, "releaseSha256": expected, **evidence,
                "pythonMinimum": "3.11", "requiredArchitecture": "arm64", "minimumOSUnderTest": "26",
                "ocr": {"architecture": arch, "minimumOS": "26.0", "dependencies": dependencies, "sha256": digest(helper)},
                "supportReuse": reused,
                "fileModes": {name: stat.S_IMODE((out / name).stat().st_mode) for name in files},
                "files": files}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"staged": str(out), "files": len(files), "releaseSha256": expected}))


if __name__ == "__main__":
    main()
