#!/usr/bin/env python3
"""Compile or run a private production SwiftUI ColorPicker ownership fixture.

Preparation freezes matching debug objects before linking. Run only in an
assigned native slot; it creates hidden AppKit views, never global input.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def preferences():
    files = [Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist']
    folder = Path.home()/'Library/Application Support/Vixen'
    if folder.exists():
        files += sorted(p for p in folder.rglob('*') if p.is_file())
    return {str(p): digest(p) if p.is_file() else None for p in files}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--production-snapshot', type=Path)
    parser.add_argument('--baseline', action='store_true')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-only', action='store_true')
    mode.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args()
    build, out = args.build.resolve(), args.out.resolve()
    assert all(p.is_relative_to(ROOT/'target') for p in [build, out])
    app = out/'SwiftUIColorTakeover.app'
    executable = app/'Contents/MacOS/SwiftUIColorTakeover'
    source = ROOT/'scripts/check_swiftui_color_takeover.swift'
    if args.prepare_only:
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        products = build/'swift/out/Products/Debug'
        originals = sorted(set([*products.glob('*.o'), *products.glob('*.swiftmodule/*'),
            build/'pkg/lib/libvixen_ffi.a', *(build/'pkg/Sources').rglob('*'),
            *(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps').glob('*')]))
        inputs = out/'link-inputs'
        copied = {}
        for path in (p for p in originals if p.is_file()):
            relative = path.relative_to(build)
            destination = inputs/relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            before = digest(path)
            subprocess.run(['/bin/cp', '-c', str(path), str(destination)], check=True)
            assert digest(destination) == before == digest(path)
            copied[str(relative)] = before
        production_root = args.production_snapshot.resolve() if args.production_snapshot else ROOT
        production = {}
        for relative in ['app/Sources/VixenKit/ColorControls.swift', 'app/Sources/VixenDialogs/Common.swift']:
            path = production_root/relative
            destination = out/'production-snapshot'/relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(['/bin/cp', '-c', str(path), str(destination)], check=True)
            production[relative] = digest(destination)
        # Clang's generated map embeds its old build-root path. Use the exact
        # frozen header under this output directory, with a separate map.
        modulemap = out/'vixen_ffiFFI.modulemap'
        modulemap.write_text('module vixen_ffiFFI {\numbrella header "'+str(inputs/'pkg/Sources/vixen_ffiFFI/include/vixen_ffiFFI.h')+'"\nexport *\n}\n')
        products = inputs/'swift/out/Products/Debug'
        objects = sorted(products.glob('*.o'))
        cmd = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library',
               '-swift-version', '6', '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0',
               '-I', str(products), '-Xcc', '-fmodule-map-file='+str(modulemap),
               '-I', str(inputs/'pkg/Sources/vixen_ffiFFI/include'), str(source), *map(str, objects),
               '-L', str(inputs/'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for name in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation',
                     'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            cmd += ['-framework', name]
        cmd += ['-o', str(executable)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        bundle = 'com.vixenlights.verification.swiftuicolor.'+uuid.uuid4().hex
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': bundle,
            'CFBundleExecutable': executable.name, 'CFBundleName': 'SwiftUI Color Ownership Probe',
            'CFBundlePackageType': 'APPL', 'LSUIElement': True, 'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        frozen = {str(inputs/p): value for p,value in copied.items()}
        frozen.update({str(out/'production-snapshot'/p): value for p,value in production.items()})
        frozen[str(modulemap)] = digest(modulemap)
        frozen[str(source)] = digest(source)
        frozen[str(Path(__file__).resolve())] = digest(__file__)
        prepared = dict(baseline=args.baseline, sourceBuild=str(build), productionHashes=production,
                        inputHashes=frozen, compileCommand=cmd, executableSha256=digest(executable),
                        bundleIdentifier=bundle, objectCount=len(objects), launched=False)
        (out/'prepared.json').write_text(json.dumps(prepared, indent=2)+'\n')
        print(json.dumps(dict(prepared=str(out/'prepared.json'), launched=False, objectCount=len(objects))))
        return
    prepared = json.loads((out/'prepared.json').read_text())
    assert args.baseline == prepared['baseline']
    assert digest(executable) == prepared['executableSha256']
    assert all(digest(p) == h for p,h in prepared['inputHashes'].items())
    assert not (out/'native-result.json').exists(), 'Use a fresh preparation for each native run'
    before = preferences()
    for directory in ['ui', 'profile']:
        (out/directory).mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_UI_TEST='private-callbacks', VIXEN_UI_TEST_OUT=str(out/'ui'),
               VIXEN_SETTINGS_DIR=str(out/'ui'), VIXEN_DATA=str(out/'profile'), VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process = subprocess.Popen([str(executable), str(out/'native-result.json')], env=env,
                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = process.wait(timeout=25)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=3)
            raise AssertionError('Owned SwiftUI color fixture exceeded 25 seconds')
    assert code == 0, (code, out/'native-result.json', out/'process.log')
    result = json.loads((out/'native-result.json').read_text())
    assert not result['appActive'] and result['allWindowsHidden']
    assert result['frontmostBefore'] == result['frontmostAfter'] != result['ownPID']
    assert all(not event['panelVisible'] for event in result['events'])
    assert before == preferences()
    assert all(digest(p) == h for p,h in prepared['inputHashes'].items()) and digest(executable) == prepared['executableSha256']
    failures = [check['name'] for check in result['checks'] if not check['passed']]
    required = ['compiled_picker_produced_native_well', 'actual_swiftui_well_activation_succeeded',
                'actual_panel_change_reaches_picker_binding', 'reverse_route_started_with_real_active_picker']
    assert all(next(c['passed'] for c in result['checks'] if c['name'] == name) for name in required), result
    result.update(passed=not failures, baseline=args.baseline, expectedOutcomeObserved=(bool(failures) if args.baseline else not failures),
                  ownerPreferencesUnchanged=True, ownerPreferenceHashes=before, inputsUnchanged=True,
                  executableSha256=prepared['executableSha256'], bundleIdentifier=prepared['bundleIdentifier'], exitCode=code)
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(passed=result['passed'], baseline=args.baseline, checks=len(result['checks']), failures=failures,
                         wellClass=result['wellClass'], expectedOutcomeObserved=result['expectedOutcomeObserved'])))
    assert result['expectedOutcomeObserved'], failures


if __name__ == '__main__':
    main()
