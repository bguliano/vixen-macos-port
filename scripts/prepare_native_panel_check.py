#!/usr/bin/env python3
"""Prepare a normal-app file-panel check with isolated Cocoa and Rust settings.

Never launches the app. The foreground runner must independently check desktop
ownership, use only these paths, and stop for unexpected dialogs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import xml.etree.ElementTree as ET

from prepare_private_test_app import code_digest

ROOT = Path(__file__).resolve().parents[1]
local = lambda name: name.split('}')[-1]
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--private-app', type=Path, required=True)
    parser.add_argument('--generator', type=Path, default=ROOT/'target/build-main/rust/debug/examples/native_panel_fixture')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    app, out = args.private_app.resolve(), args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    manifest = json.loads((app.parent/'private-app.json').read_text())
    info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
    executable = app/'Contents/MacOS'/info['CFBundleExecutable']
    assert info['CFBundleIdentifier'] == manifest['bundleIdentifier']
    assert info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    assert digest(executable) == manifest['executableSha256']
    assert code_digest(executable) == manifest['executableCodeDigest']
    assert b'VIXEN_SETTINGS_DIR' in executable.read_bytes(), 'This app lacks normal-mode settings isolation'
    out.mkdir(parents=True)
    profile = out/'profile'
    subprocess.run([str(args.generator.resolve()), str(profile)], check=True, capture_output=True, timeout=30)
    system = ET.parse(profile/'SystemData/SystemConfig.xml').getroot()
    assert len(system.find('Controllers')) == 0
    store = ET.parse(profile/'SystemData/ModuleStore.xml').getroot()
    web = [n for n in store.iter() if local(n.tag) == 'IsEnabled' and 'WebServer' in n.tag]
    assert len(web) == 1 and (web[0].text or '').lower() == 'false'
    fixture = json.loads((profile/'fixture.json').read_text())
    original = Path(fixture['sequence'])
    assert original.resolve().is_relative_to(profile) and original.is_file()
    settings = out/'settings'; settings.mkdir()
    destination = out/'Saved sequences é'; destination.mkdir()
    output = destination/'Native Saved Café.tim'
    assert not output.exists()
    source_hashes = {str(p.relative_to(profile)):digest(p) for p in profile.rglob('*') if p.is_file()}
    prepared = {
        'prepared': True, 'launched': False, 'app': str(app), 'appManifest': manifest,
        'profile': str(profile), 'settings': str(settings), 'openFile': str(original),
        'saveFile': str(output), 'originalSequenceSha256': digest(original),
        'initialProfileHashes': source_hashes, 'effectInstanceId': fixture['effectInstanceId'], 'effectSessionId': fixture['effectSessionId'],
        'environment': {'VIXEN_DATA':str(profile), 'VIXEN_SETTINGS_DIR':str(settings), 'VIXEN_MUTE':'1'},
        'arguments': ['no_controllers', 'no_execution'],
        'scope': 'Normal Open/Save panels, no VIXEN_UI_TEST or scripted SaveAs substitution. Unique Cocoa identity and private Rust settings; web disabled in private fixture.',
    }
    (out/'prepared.json').write_text(json.dumps(prepared, indent=2)+'\n')
    print(out/'prepared.json')


if __name__ == '__main__':
    main()
