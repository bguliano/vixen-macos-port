#!/usr/bin/env python3
"""Observe actual retained dock native views in a private offscreen host.

No WindowServer input or drag-handler calls. Coordinate the native app slot.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--run-prepared', action='store_true')
    parser.add_argument('--check-only', action='store_true')
    parser.add_argument('--expect-inactive-registered', action='store_true')
    args = parser.parse_args()
    build, out = args.build.resolve(), args.out.resolve()
    assert build.is_relative_to(ROOT/'target') and out.is_relative_to(ROOT/'target')
    profile = ROOT/'target/library-drag-fixture/profile'
    app = out/'LibraryDockVisibility.app'
    executable = app/'Contents/MacOS/LibraryDockVisibility'
    if not (args.run_prepared or args.check_only):
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        source = out/'probe.swift'
        shutil.copyfile(ROOT/'scripts/check_library_dock_visibility.swift', source)
        products = build/'swift/out/Products/Debug'
        objects = sorted(products.glob('*.o'))
        modulemap = build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'
        inputs = [source, *objects, modulemap, build/'pkg/lib/libvixen_ffi.a']
        inputs += [p for p in products.rglob('*') if p.is_file() and '.swiftmodule' in str(p)]
        inputs += list((build/'pkg/Sources/vixen_ffiFFI/include').glob('*'))
        hashes = {str(p): digest(p) for p in inputs}
        command = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library',
                   '-swift-version', '6', '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0',
                   '-I', str(products), '-Xcc', '-fmodule-map-file='+str(modulemap),
                   '-I', str(build/'pkg/Sources/vixen_ffiFFI/include'), str(source),
                   *(str(p) for p in objects), '-L', str(build/'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for name in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation',
                     'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            command += ['-framework', name]
        command += ['-o', str(executable)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        assert all(digest(Path(p)) == expected for p, expected in hashes.items())
        identifier = 'com.vixenlights.verification.librarydock.'+digest(executable)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': identifier,
            'CFBundleExecutable': 'LibraryDockVisibility', 'CFBundlePackageType': 'APPL', 'LSUIElement': True,
            'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True, capture_output=True, timeout=30)
        shutil.copytree(profile, out/'profile')
        (out/'profile/.lock').unlink(missing_ok=True)
        manifest = {'compileCommand': command, 'inputHashesAtLink': hashes,
                    'executableSha256': digest(executable), 'bundleIdentifier': identifier,
                    'profileHashes': {str(p.relative_to(profile)): digest(p) for p in profile.rglob('*') if p.is_file()}}
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
    if args.prepare_only:
        print(out/'prepared.json'); return
    manifest = json.loads((out/'prepared.json').read_text())
    assert digest(executable) == manifest['executableSha256']
    if not args.check_only:
        assert not (out/'result.json').exists()
        environment = {k: v for k, v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
        (out/'runtime').mkdir()
        environment.update(VIXEN_DATA=str(out/'profile'), VIXEN_MUTE='1', TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log:
            process = subprocess.run([str(executable), str(out/'result.json')], env=environment,
                                     stdout=log, stderr=subprocess.STDOUT, timeout=30)
        assert process.returncode == 0, process.returncode
    result = json.loads((out/'result.json').read_text())
    assert result['foregroundUnchanged']
    assert all(digest(profile/p) == expected for p, expected in manifest['profileHashes'].items())
    expected = {'Pulse': {'CurveStripNSView', 'GradientStripNSView'}, 'Set Level': {'ColorSwatchNSView'}}
    checks = []
    for observation in result['observations']:
        views = [v for v in observation['views'] if v['class'] in expected[observation['effect']]]
        assert {v['class'] for v in views} == expected[observation['effect']]
        assert len(views) == len(expected[observation['effect']])
        active = observation['selectedPanel'] == 'effectEditor' or args.expect_inactive_registered
        for view in views:
            assert view['hidden'] == (not active) and view['registered'] == (3 if active else 0), observation
            checks.append({'effect': observation['effect'], 'panel': observation['selectedPanel'], **view})
        cells = [v for v in observation['views'] if v['class'] == 'LibraryCellView']
        assert cells and all('viewportBoundsInWindow' in v for v in cells)
    assert len(checks) == 9
    verification = {'checks': checks, 'foregroundUnchanged': True, 'sourceUnchanged': True,
                    'productPass': not args.expect_inactive_registered,
                    'expectedKnownGapObserved': args.expect_inactive_registered,
                    'scope': result['scope'], 'executableSha256': manifest['executableSha256']}
    (out/'verification.json').write_text(json.dumps(verification, indent=2)+'\n')
    print(json.dumps({'nativeVisibilityChecks': len(checks), 'productPass': verification['productPass'],
                      'foregroundUnchanged': True}))


if __name__ == '__main__':
    main()
