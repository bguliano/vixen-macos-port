#!/usr/bin/env python3
"""Prepare/run a bounded private native-well ownership probe; never global input."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import zipfile
from check_color_sessions import ROOT, digest


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build', type=Path, required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--prepare-only', action='store_true')
    p.add_argument('--run-prepared', action='store_true')
    a = p.parse_args(); build, out = a.build.resolve(), a.out.resolve()
    assert all(x.is_relative_to(ROOT/'target') for x in [build, out])
    products = build/'swift/out/Products/Debug'
    source = ROOT/'scripts/check_prop_color_takeover.swift'
    profile = ROOT/'target/library-drag-fixture/profile'
    sample = ROOT/'target/vixen2-ui/sample.vix'
    prop = ROOT/'target/resume-ui/prop-panels/metadata.prp'
    config = ROOT/'target/vixen2-color-fixed-02/fixture-SystemConfig.xml'
    production = [ROOT/x for x in ['app/Sources/VixenTools/PropStates.swift', 'app/Sources/VixenTools/PropEditorModel.swift',
        'app/Sources/VixenKit/ColorControls.swift', 'app/Sources/VixenScheduler/Vixen2ImporterWindow.swift',
        'crates/vixen-prop/src/states.rs', 'crates/vixen-ffi/src/tools/states.rs']]
    app = out/'PropColorTakeover.app'; exe = app/'Contents/MacOS/PropColorTakeover'
    objects = sorted(products.glob('*.o'))
    inputs = [source, Path(__file__).resolve(), ROOT/'scripts/check_color_sessions.py', *production, *objects,
              build/'pkg/lib/libvixen_ffi.a', build/'pkg/Sources/VixenBridge/vixen_ffi.swift', sample, prop, config]
    inputs += sorted(x for x in products.glob('*.swiftmodule/*') if x.is_file())
    if not a.run_prepared:
        assert not out.exists(); exe.parent.mkdir(parents=True)
        cmd = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library', '-swift-version', '6',
               '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0', '-I', str(products), '-Xcc',
               '-fmodule-map-file='+str(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'),
               '-I', str(build/'pkg/Sources/vixen_ffiFFI/include'), str(source), *(str(x) for x in objects),
               '-L', str(build/'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for name in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation', 'CoreGraphics',
                     'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            cmd += ['-framework', name]
        cmd += ['-o', str(exe)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        bundle = 'com.vixenlights.verification.propcolortakeover.'+digest(exe)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': bundle, 'CFBundleExecutable': 'PropColorTakeover',
            'CFBundleName': 'Prop Color Takeover Probe', 'CFBundlePackageType': 'APPL', 'LSUIElement': True, 'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        compiled = {str(x): digest(x) for x in inputs}; frozen = {}; mapping = {}
        for i, (name, wanted) in enumerate(compiled.items()):
            dst = out/'input-snapshot'/f'{i:03d}-{Path(name).name}'; dst.parent.mkdir(exist_ok=True)
            subprocess.run(['/bin/cp', '-c', name, str(dst)], check=True)
            assert digest(dst) == wanted; frozen[str(dst)] = wanted; mapping[name] = str(dst)
        original = {str(x.relative_to(profile)): digest(x) for x in profile.rglob('*') if x.is_file()}
        shutil.copytree(profile, out/'profile'); (out/'profile/.lock').unlink(missing_ok=True)
        shutil.copyfile(config, out/'profile/SystemData/SystemConfig.xml')
        shutil.copyfile(sample, out/'sample.vix'); shutil.copyfile(prop, out/'fixture.prp')
        manifest = dict(compileCommand=cmd, compiledInputHashes=compiled, inputHashes=frozen, inputSnapshotMap=mapping,
                        productionHashes={str(x): digest(x) for x in production}, executableSha256=digest(exe),
                        bundleIdentifier=bundle, sourceProfile=str(profile), sourceProfileHashes=original)
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
    if a.prepare_only: print(out/'prepared.json'); return
    m = json.loads((out/'prepared.json').read_text())
    assert digest(exe) == m['executableSha256'] and all(digest(Path(k)) == v for k,v in m['inputHashes'].items())
    assert all(digest(Path(k)) == v for k,v in m['productionHashes'].items())
    preferences = [Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist',
                   Path.home()/'Library/Application Support/Vixen/Settings.xml']
    before = {str(x): digest(x) if x.exists() else None for x in preferences}
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_UI_TEST='private-callbacks', VIXEN_UI_TEST_OUT=str(out/'ui'), VIXEN_DATA=str(out/'profile'),
               VIXEN_V2_INPUT=str(out/'sample.vix'), VIXEN_PROP_INPUT=str(out/'fixture.prp'), VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process = subprocess.run([str(exe), str(out/'native-result.json')], env=env, stdout=log, stderr=subprocess.STDOUT, timeout=25)
    assert process.returncode == 0, (process.returncode, out/'process.log')
    result = json.loads((out/'native-result.json').read_text())
    assert not result['appActive'] and result['frontmostBefore'] == result['frontmostAfter'] != result['ownPID']
    assert all(not x['panelVisible'] for x in result['events'])
    assert before == {str(x): digest(x) if x.exists() else None for x in preferences}
    assert all(digest(Path(k)) == v for k,v in m['productionHashes'].items())
    assert digest(exe) == m['executableSha256'] and all(digest(Path(k)) == v for k,v in m['inputHashes'].items())
    assert {str(x.relative_to(profile)): digest(x) for x in profile.rglob('*') if x.is_file()} == m['sourceProfileHashes']
    persisted = {}
    for name in ['prop-after-native-well.prp', 'prop-after-shared-session.prp', 'prop-after-double-click.prp']:
        with zipfile.ZipFile(out/name) as z: doc = json.loads(z.read('prop.json'))
        rows = [i for e in doc['elements'] for d in e['stateDefinitions'] for i in d['items'] if i['id'] == result['originalPropItemID']]
        assert len(rows) == 1
        persisted[name] = dict(sha256=digest(out/name), item=rows[0])
    assert persisted['prop-after-double-click.prp']['item']['color'] == '#FFFF00FF'
    result.update(inputsUnchanged=True, ownerPreferencesUnchanged=True, ownerPreferenceHashes=before,
                  executableSha256=m['executableSha256'], savedProps=persisted,
                  savedModuleStoreSha256=digest(out/'profile/SystemData/ModuleStore.xml'),
                  findingsRequireActiveWell=result['activeWellObservedBeforeReverse'])
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(checks=len(result['checks']), failures=[x['name'] for x in result['checks'] if not x['passed']],
                         activeWell=result['activeWellObservedBeforeReverse'], fanOutObserved=result['fanOutObserved'])))


if __name__ == '__main__': main()
