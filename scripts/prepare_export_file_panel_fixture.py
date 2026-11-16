#!/usr/bin/env python3
"""Prepare private normal Export-panel files; never launch Vixen."""
import argparse
import json
from pathlib import Path
import shutil
import xml.etree.ElementTree as ET
from export_file_panel_contract import sha

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "target/native-panels-quit-menu-fixture/profile"


def prepare(out):
    out = out.resolve()
    assert out.is_relative_to(ROOT / "target") and not out.exists()
    profile = out / "profile"
    for name in ["SystemData", "Sequence", "Media", "Module Data Files", "Export"]:
        (profile / name).mkdir(parents=True, exist_ok=True)
    source_inputs = {}
    for name in ["SystemData/SystemConfig.xml", "SystemData/ModuleStore.xml", "Sequence/Panel Source Café.tim"]:
        source, dest = SOURCE / name, profile / name
        shutil.copy2(source, dest)
        source_inputs[str(source)] = sha(source)
        assert sha(dest) == sha(source)
    system = ET.parse(profile / "SystemData/SystemConfig.xml").getroot()
    assert all(len(system.find(name)) == 0 for name in ["Controllers", "Patches", "Previews"])
    modules = ET.parse(profile / "SystemData/ModuleStore.xml").getroot()
    web = [x for x in modules.iter() if x.tag.endswith("}IsEnabled") and "WebServer" in x.tag]
    assert len(web) == 1 and web[0].text == "false"
    sequence = profile / "Sequence/Panel Source Café.tim"
    seq = ET.parse(sequence).getroot()
    assert [x.text for x in seq.iter() if x.tag == "Length"] == ["PT3S"]
    assert not any((x.text or "").strip() for x in seq.iter() if x.tag.rsplit("}", 1)[-1] in ["FileName", "Glediator_FileName"])
    for name in ["settings", "runtime", "Exported files é"]:
        (out / name).mkdir()
    output = out / "Exported files é"
    sentinel = output / "Preserve on Cancel.fseq"
    sentinel.write_bytes(b"private existing file; Save cancellation must preserve every byte\n")
    pins = [Path(__file__).resolve(), ROOT / "scripts/export_file_panel_contract.py"]
    m = dict(preparedOnly=True, launched=False, profile=str(profile),
             openFile=str(sequence), openFileSha256=sha(sequence), sequenceTitle="Panel Source Café",
             exportFile=str(output / "Export Panel Café.fseq"),
             defaultExportFile=str(profile / "Export" / "Panel Source Café.fseq"),
             cancelSentinel=str(sentinel), cancelSentinelSha256=sha(sentinel),
             expectedFrames=60, expectedStepMs=50, expectedPatchedChannels=0,
             sourceInputs=source_inputs,
             profileHashes={str(p.relative_to(profile)): sha(p) for p in sorted(profile.rglob("*")) if p.is_file()},
             requiredEnvironment=dict(VIXEN_DATA=str(profile), VIXEN_SETTINGS_DIR=str(out / "settings"), VIXEN_MUTE="1", TMPDIR=str(out / "runtime") + "/"),
             requiredArguments=["no_controllers", "no_execution"],
             forbiddenEnvironment=["VIXEN_UI_TEST", "VIXEN_TEST_FILE"],
             pins={str(p): sha(p) for p in pins},
             scope="Normal private app; actual sequence Open, Export Start and native Save Cancel, Unicode destination Save, then untouched default-name Save; both FSEQ files structurally verified, sheet Close and Quit menu. No playback or hardware claim.")
    (out / "prepared.json").write_text(json.dumps(m, indent=2) + "\n")
    return m


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    m = prepare(args.out)
    print(json.dumps(dict(preparedOnly=True, launched=False, profile=m["profile"], output=m["exportFile"])))
