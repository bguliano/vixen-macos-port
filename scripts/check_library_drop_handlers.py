#!/usr/bin/env python3
"""Link an isolated native handler check against pinned production Swift objects.

No app rebuild, foreground input, show profile, or copied acceptance logic.
Coordinate the short background AppKit process before --run-prepared.
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
    parser.add_argument('--build', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--run-prepared', action='store_true')
    parser.add_argument('--expect-reentry-gap', action='store_true')
    parser.add_argument('--expect-routing-gap', action='store_true')
    parser.add_argument('--expect-cross-model-gap', action='store_true')
    parser.add_argument('--has-boundary', action='store_true', help='Wire the production viewport reference added by the re-entry fix.')
    parser.add_argument('--production-source', type=Path, default=ROOT / 'app/Sources/VixenSequencer/LibraryPanels.swift')
    parser.add_argument('--pin-inputs', action='store_true', help='Freeze linked inputs before a shared-cache rebuild; does not launch.')
    args = parser.parse_args()
    build, out = args.build.resolve(), args.out.resolve()
    assert build.is_relative_to(ROOT / 'target') and out.is_relative_to(ROOT / 'target')
    source = out / 'probe.swift'
    app = out / 'LibraryDropHandlers.app'
    executable = app / 'Contents/MacOS/LibraryDropHandlers'
    if args.pin_inputs:
        manifest_path = out / 'prepared.json'
        manifest = json.loads(manifest_path.read_text())
        assert digest(executable) == manifest['executableSha256']
        originals = manifest['inputHashes']
        assert all(digest(Path(path)) == expected for path, expected in originals.items())
        snapshot = out / 'input-snapshot'
        snapshot.mkdir()
        pinned = {}
        mapping = {}
        for index, (path, expected) in enumerate(originals.items()):
            target = snapshot / f'{index:03d}-{Path(path).name}'
            subprocess.run(['/bin/cp', '-c', path, str(target)], check=True, timeout=30)
            assert digest(target) == expected
            pinned[str(target)] = expected
            mapping[path] = str(target)
        manifest.update(originalInputHashes=originals, inputHashes=pinned, pinnedInputs=mapping)
        manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
        print(manifest_path)
        return
    if not args.run_prepared:
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / 'scripts/check_library_drop_handlers.swift', source)
        products = build / 'swift/out/Products/Debug'
        objects = sorted(products.glob('*.o'))
        assert any(path.name == 'VixenSequencer.o' for path in objects)
        inputs = [source, *objects, build / 'pkg/lib/libvixen_ffi.a']
        inputs += [path for path in products.rglob('*') if path.is_file() and '.swiftmodule' in str(path)]
        modulemap = build / 'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'
        inputs += [modulemap]
        inputs += list((build / 'pkg/Sources/vixen_ffiFFI/include').glob('*'))
        command = ['/usr/bin/swiftc', '-module-cache-path', str(out / 'clang-cache'), '-parse-as-library',
                   '-swift-version', '6', '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0',
                   '-I', str(products), '-Xcc', '-fmodule-map-file=' + str(modulemap),
                   '-I', str(build / 'pkg/Sources/vixen_ffiFFI/include'), str(source),
                   *(str(path) for path in objects), '-L', str(build / 'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for framework in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation',
                          'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            command += ['-framework', framework]
        if args.has_boundary:
            command += ['-D', 'LIBRARY_REENTRY_BOUNDARY']
        command += ['-o', str(executable)]
        with (out / 'compile.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        identifier = 'com.vixenlights.verification.libraryhandlers.' + digest(executable)[:16]
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': identifier, 'CFBundleExecutable': 'LibraryDropHandlers',
            'CFBundleName': 'Vixen Library Handler Verification', 'CFBundlePackageType': 'APPL',
            'LSUIElement': True, 'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True, capture_output=True, timeout=30)
        shutil.copyfile(args.production_source, out / 'LibraryPanels.source.swift')
        manifest = {'inputHashes': {str(path): digest(path) for path in inputs},
                    'productionSourceSha256': digest(out / 'LibraryPanels.source.swift'),
                    'executableSha256': digest(executable), 'bundleIdentifier': identifier,
                    'compileCommand': command, 'hasBoundary': args.has_boundary, 'launched': False}
        (out / 'prepared.json').write_text(json.dumps(manifest, indent=2) + '\n')
    if args.prepare_only:
        print(out / 'prepared.json')
        return
    manifest = json.loads((out / 'prepared.json').read_text())
    assert manifest['hasBoundary'] == args.has_boundary
    assert digest(executable) == manifest['executableSha256']
    assert all(digest(Path(path)) == value for path, value in manifest['inputHashes'].items())
    assert not (out / 'native-result.json').exists()
    environment = {key: value for key, value in os.environ.items() if not key.startswith(('VIXEN_', 'DYLD_'))}
    with (out / 'process.log').open('w') as log:
        result = subprocess.run([str(executable), str(out / 'native-result.json')], env=environment,
                                stdout=log, stderr=subprocess.STDOUT, timeout=30)
    assert result.returncode == 0
    report = json.loads((out / 'native-result.json').read_text())
    expected = set()
    if args.expect_reentry_gap:
        expected.update({'whole-panel-reentry-cell-uses-new-entry', 'whole-panel-reentry-empty-uses-new-entry'})
    if args.expect_routing_gap:
        expected.update('panel-to-toolbar-' + name + '-route' for name in
                        ['color', 'curve', 'gradient', 'linked-curve', 'linked-gradient'])
    if args.expect_cross_model_gap:
        expected.update('cross-model-toolbar-' + name + '-route' for name in
                        ['color', 'curve', 'gradient', 'linked-curve', 'linked-gradient'])
    assert set(report['failures']) == expected, report['failures']
    assert all(digest(Path(path)) == value for path, value in manifest['inputHashes'].items())
    report.update(executableSha256=manifest['executableSha256'], inputsUnchanged=True,
                  expectedKnownGapObserved=bool(expected),
                  expectedRoutingGapObserved=args.expect_routing_gap,
                  expectedCrossModelGapObserved=args.expect_cross_model_gap)
    (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'checks': len(report['checks']), 'failures': report['failures'],
                      'expectedKnownGapObserved': bool(expected)}))


if __name__ == '__main__':
    main()
