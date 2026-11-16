#!/usr/bin/env python3
"""Compile/run isolated native colour-session regressions from matching debug objects.

The baseline option records expected regressions without claiming a passing app.
No system input is posted. Run only when another foreground test is not active.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    p.add_argument('--baseline', action='store_true')
    p.add_argument('--prepare-only', action='store_true')
    p.add_argument('--run-prepared', action='store_true')
    args = p.parse_args()
    build, out = args.build.resolve(), args.out.resolve()
    assert build.is_relative_to(ROOT/'target') and out.is_relative_to(ROOT/'target')
    products = build/'swift/out/Products/Debug'
    source = ROOT/'scripts/check_color_sessions.swift'
    app = out/'ColorSessions.app'
    executable = app/'Contents/MacOS/ColorSessions'
    inputs = [source, *(products/f'{name}.o' for name in ['VixenKit', 'VixenBridge', 'vixen_ffiFFI']),
              build/'pkg/lib/libvixen_ffi.a', build/'pkg/Sources/VixenBridge/vixen_ffi.swift']
    inputs += [path for path in (products/'VixenKit.swiftmodule').glob('*') if path.is_file()]
    if not args.run_prepared:
        assert not out.exists(), 'Use a fresh output directory'
        executable.parent.mkdir(parents=True)
        cmd = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library',
               '-swift-version', '6', '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0',
               '-I', str(products), '-Xcc', '-fmodule-map-file='+str(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'),
               '-I', str(build/'pkg/Sources/vixen_ffiFFI/include')]
        if not args.baseline:
            cmd += ['-D', 'COLOR_FIXED']
        cmd += [str(source), *(str(products/f'{name}.o') for name in ['VixenKit', 'VixenBridge', 'vixen_ffiFFI']),
                '-L', str(build/'pkg/lib'), '-lvixen_ffi', '-lc++', '-lcurl']
        for framework in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation',
                          'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            cmd += ['-framework', framework]
        cmd += ['-o', str(executable)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        bundle = 'com.vixenlights.verification.colorsessions.' + digest(executable)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': bundle,
            'CFBundleExecutable': 'ColorSessions', 'CFBundleName': 'Vixen Colour Session Verification',
            'CFBundlePackageType': 'APPL', 'LSUIElement': True, 'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        manifest = {'baseline': args.baseline, 'build': str(build), 'inputHashes': {str(path):digest(path) for path in inputs},
                    'executableSha256': digest(executable), 'bundleIdentifier': bundle, 'compileCommand': cmd}
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
    if args.prepare_only:
        print(out/'prepared.json'); return
    manifest = json.loads((out/'prepared.json').read_text())
    assert manifest['baseline'] == args.baseline
    assert digest(executable) == manifest['executableSha256']
    assert all(digest(Path(path)) == expected for path, expected in manifest['inputHashes'].items())
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_UI_TEST='private-callback-checks', VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process = subprocess.run([str(executable), str(out/'native-result.json')], env=env,
                                 stdout=log, stderr=subprocess.STDOUT, timeout=35)
    assert process.returncode == 0, (process.returncode, out/'process.log')
    result = json.loads((out/'native-result.json').read_text())
    assert len(result['checks']) == 13
    failures = [row['name'] for row in result['checks'] if not row['passed']]
    if args.baseline:
        assert {'new_unchanged_white_added_once', 'gradient_pending_then_select_commits_original_stop',
                'gradient_reverted_then_select_clears_preview'} <= set(failures), failures
    else:
        assert result['passed'], failures
    assert digest(executable) == manifest['executableSha256']
    assert all(digest(Path(path)) == expected for path, expected in manifest['inputHashes'].items())
    result.update(expectedOutcomeObserved=True, baseline=args.baseline, executableSha256=manifest['executableSha256'],
                  inputHashes=manifest['inputHashes'], inputsUnchanged=True)
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({'expectedOutcomeObserved':True, 'baseline':args.baseline, 'checks':len(result['checks']), 'failures':failures}))


if __name__ == '__main__':
    main()
