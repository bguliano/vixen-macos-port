#!/usr/bin/env python3
"""Compile private Vixen2 color callback checks; no global UI input."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import xml.etree.ElementTree as ET
from check_color_sessions import ROOT, digest


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--baseline', action='store_true')
    p.add_argument('--prepare-only', action='store_true')
    p.add_argument('--run-prepared', action='store_true')
    a = p.parse_args()
    build, out = a.build.resolve(), a.out.resolve()
    assert all(x.is_relative_to(ROOT/'target') for x in [build, out])
    products = build/'swift/out/Products/Debug'
    source = ROOT/'scripts/check_vixen2_color_sessions.swift'
    production = ROOT/'app/Sources/VixenScheduler/Vixen2ImporterWindow.swift'
    profile = ROOT/'target/library-drag-fixture/profile'
    sample = ROOT/'target/vixen2-ui/sample.vix'
    app = out/'Vixen2ColorSessions.app'
    exe = app/'Contents/MacOS/Vixen2ColorSessions'
    objects = sorted(products.glob('*.o'))
    inputs = [source, Path(__file__).resolve(), ROOT/'scripts/check_color_sessions.py', production,
              *objects, build/'pkg/lib/libvixen_ffi.a', build/'pkg/Sources/VixenBridge/vixen_ffi.swift']
    inputs += sorted(x for x in products.glob('*.swiftmodule/*') if x.is_file())
    if not a.run_prepared:
        assert not out.exists()
        exe.parent.mkdir(parents=True)
        cmd = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library', '-swift-version', '6',
               '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0', '-I', str(products), '-Xcc',
               '-fmodule-map-file='+str(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'),
               '-I', str(build/'pkg/Sources/vixen_ffiFFI/include')]
        if a.baseline: cmd += ['-D', 'VIXEN2_BASELINE']
        cmd += [str(source), *(str(x) for x in objects), '-L', str(build/'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for name in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation', 'CoreGraphics',
                     'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            cmd += ['-framework', name]
        cmd += ['-o', str(exe)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        bundle = 'com.vixenlights.verification.vixen2colors.'+digest(exe)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': bundle, 'CFBundleExecutable': 'Vixen2ColorSessions',
            'CFBundleName': 'Vixen2 Color Verification', 'CFBundlePackageType': 'APPL', 'LSUIElement': True, 'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        compiled = {str(x): digest(x) for x in inputs}; frozen = {}; mapping = {}
        for i, (name, wanted) in enumerate(compiled.items()):
            dst = out/'input-snapshot'/f'{i:03d}-{Path(name).name}'; dst.parent.mkdir(exist_ok=True)
            subprocess.run(['/bin/cp', '-c', name, str(dst)], check=True)
            assert digest(dst) == wanted; frozen[str(dst)] = wanted; mapping[name] = str(dst)
        original = {str(x.relative_to(profile)): digest(x) for x in profile.rglob('*') if x.is_file()}
        shutil.copytree(profile, out/'profile'); (out/'profile/.lock').unlink(missing_ok=True)
        # A generated local leaf is enough for map serialization; the library
        # fixture intentionally has no real channels or controller outputs.
        config = out/'profile/SystemData/SystemConfig.xml'
        tree = ET.parse(config)
        ET.SubElement(tree.getroot().find('Channels'), 'Channel', id='11111111-1111-4111-8111-111111111111', name='Callback Fixture')
        ET.SubElement(tree.getroot().find('Nodes'), 'Node', id='22222222-2222-4222-8222-222222222222',
                      channelId='11111111-1111-4111-8111-111111111111', name='Callback Fixture')
        tree.write(config, encoding='utf-8', xml_declaration=True)
        shutil.copyfile(config, out/'fixture-SystemConfig.xml')
        shutil.copyfile(sample, out/'sample.vix')
        manifest = dict(baseline=a.baseline, compileCommand=cmd, compiledInputHashes=compiled, inputHashes=frozen,
                        inputSnapshotMap=mapping, executableSha256=digest(exe), bundleIdentifier=bundle,
                        sourceProfile=str(profile), sourceProfileHashes=original, sampleSha256=digest(sample),
                        generatedFixtureSha256=digest(config), expectedChecks=18)
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
    if a.prepare_only: print(out/'prepared.json'); return
    m = json.loads((out/'prepared.json').read_text())
    assert m['baseline'] == a.baseline and digest(exe) == m['executableSha256']
    assert all(digest(Path(k)) == v for k, v in m['inputHashes'].items())
    preferences = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist',
                   Path.home()/'Library/Application Support/Vixen/Settings.xml']
    before = {str(x): digest(x) if x.exists() else None for x in preferences}
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_UI_TEST='private-callbacks', VIXEN_UI_TEST_OUT=str(out/'ui'), VIXEN_DATA=str(out/'profile'),
               VIXEN_V2_INPUT=str(out/'sample.vix'), VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process = subprocess.run([str(exe), str(out/'native-result.json')], env=env, stdout=log, stderr=subprocess.STDOUT, timeout=25)
    assert process.returncode == 0, (process.returncode, out/'process.log')
    result = json.loads((out/'native-result.json').read_text())
    assert len(result['checks']) == m['expectedChecks'] and not result['appActive']
    assert result['frontmostBefore'] == result['frontmostAfter'] != result['ownPID']
    failed = [x['name'] for x in result['checks'] if not x['passed']]
    if a.baseline:
        assert {'selection_change_keeps_original_row', 'multiselection_keeps_original_rows', 'empty_selection_does_not_retarget',
                'unchanged_initial_red_commits', 'map_begin_finishes_previous_owner'} <= set(failed), failed
    else:
        assert not failed, failed
        assert result['presentationStates'] and all(not s['panelVisibleAfterBegin'] and s['nativeOwnerActiveAfterOrderOut'] and
            s['alphaRestored'] == 1 for s in result['presentationStates']), result['presentationStates']
        observed = result['orderingObservations']
        assert observed and any(o['mode'] == 0 for o in observed)
        assert all(o['alpha'] == 0 for o in observed if o['mode'] != 0 or o['visible']), observed
    assert before == {str(x): digest(x) if x.exists() else None for x in preferences}
    assert digest(exe) == m['executableSha256'] and all(digest(Path(k)) == v for k,v in m['inputHashes'].items())
    assert {str(x.relative_to(profile)): digest(x) for x in profile.rglob('*') if x.is_file()} == m['sourceProfileHashes']
    assert digest(sample) == digest(out/'sample.vix') == m['sampleSha256']
    # Inspect the actual serialized module store, independently of the FFI's
    # in-memory reopen. Blue is the original named .NET color37.
    store = ET.parse(out/'profile/SystemData/ModuleStore.xml')
    entries = [x for x in store.iter() if any(c.tag.rsplit('}', 1)[-1] == 'Key' and c.text == 'Accepted Color Map' for c in x)]
    assert len(entries) == 1
    rows = [x for x in entries[0].iter() if x.tag.rsplit('}', 1)[-1] == 'ChannelMapping']
    assert len(rows) == 6
    color = next(x for x in rows[0] if x.tag.rsplit('}', 1)[-1] == 'DestinationColor')
    color_fields = {x.tag.rsplit('}', 1)[-1]: x.text for x in color}
    assert color_fields['knownColor'] == '37' and color_fields['state'] == '1'
    result.update(expectedOutcomeObserved=True, baseline=a.baseline, inputsUnchanged=True, ownerPreferencesUnchanged=True,
                  ownerPreferenceHashes=before, executableSha256=m['executableSha256'], savedXMLBlueVerified=True,
                  savedModuleStoreSha256=digest(out/'profile/SystemData/ModuleStore.xml'))
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(checks=len(result['checks']), expectedOutcomeObserved=True, failures=failed)))


if __name__ == '__main__': main()
