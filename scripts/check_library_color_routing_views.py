#!/usr/bin/env python3
"""Read-only Color/Gradient/Curve native hit-test and drag-type differential.

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
    app = out/'LibraryRoutingViews.app'
    executable = app/'Contents/MacOS/LibraryRoutingViews'
    if not (args.run_prepared or args.check_only):
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        source = out/'probe.swift'
        shutil.copyfile(ROOT/'scripts/check_library_color_routing_views.swift', source)
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
        identifier = 'com.vixenlights.verification.libraryroutingviews.'+digest(executable)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': identifier,
            'CFBundleExecutable': 'LibraryRoutingViews', 'CFBundlePackageType': 'APPL', 'LSUIElement': True,
            'LSMinimumSystemVersion': '26.0'}))
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(app)], check=True, capture_output=True, timeout=30)
        subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True, capture_output=True, timeout=30)
        source_pins = {str(p):digest(p) for p in [ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift', ROOT/'app/Sources/VixenSequencer/LayersFindPanels.swift', ROOT/'app/Sources/VixenSequencer/SequencerContent.swift', ROOT/'app/Sources/VixenKit/KitSupport.swift', ROOT/'target/native-curve-tooltip-build-report.json']}
        shutil.copytree(profile, out/'profile')
        (out/'profile/.lock').unlink(missing_ok=True)
        manifest = {'compileCommand': command, 'inputHashesAtLink': hashes,
                    'executableSha256': digest(executable), 'bundleIdentifier': identifier, 'productionSourcePins':source_pins, 'scope':'Current compiled debug source/module; no production mutation or global input',
                    'profileHashes': {str(p.relative_to(profile)): digest(p) for p in profile.rglob('*') if p.is_file()}}
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
    if args.prepare_only:
        print(out/'prepared.json'); return
    manifest = json.loads((out/'prepared.json').read_text())
    assert digest(executable) == manifest['executableSha256']
    assert all(digest(Path(p))==h for p,h in manifest['inputHashesAtLink'].items())
    assert all(digest(Path(p))==h for p,h in manifest['productionSourcePins'].items())
    if not args.check_only:
        assert not (out/'result.json').exists()
        previous = subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=3).strip()
        assert previous.startswith('ASN:'), 'Foreground identity unavailable; no probe launched'
        (out/'foreground-before.json').write_text(json.dumps({'asn':previous})+'\n')
        before = {str(p):digest(p) if p.exists() else None for p in [Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist',Path.home()/'Library/Application Support/Vixen/Settings.xml']}
        environment = {k: v for k, v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
        (out/'runtime').mkdir();(out/'settings').mkdir()
        shutil.copy2(profile/'ColorPalette.xml',out/'settings/ColorPalette.xml')
        environment.update(VIXEN_DATA=str(out/'profile'), VIXEN_MUTE='1', VIXEN_SETTINGS_DIR=str(out/'settings'), TMPDIR=str(out/'runtime')+'/')
        process = None
        try:
            with (out/'process.log').open('w') as log:
                process = subprocess.Popen([str(executable), str(out/'result.json')], env=environment, stdout=log, stderr=subprocess.STDOUT)
                process.wait(timeout=15)
            assert process.returncode == 0, process.returncode
        finally:
            if process and process.poll() is None:
                process.terminate()
                try:process.wait(timeout=3)
                except subprocess.TimeoutExpired:process.kill();process.wait(timeout=3)
            current = subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=3).strip()
            final = {'foregroundASNBefore':previous,'foregroundASNAfter':current,'foregroundUnchanged':current==previous,'ownedProcessExited':process is not None and process.poll() is not None,'exitCode':process.returncode if process else None,'ownerPreferencesUnchanged':all((digest(Path(p)) if Path(p).exists() else None)==h for p,h in before.items()),'inputsUnchanged':all(digest(Path(p))==h for p,h in manifest['inputHashesAtLink'].items()),'productionUnchanged':all(digest(Path(p))==h for p,h in manifest['productionSourcePins'].items()),'appUnchanged':digest(executable)==manifest['executableSha256']}
            (out/'lifecycle.json').write_text(json.dumps(final,indent=2)+'\n')
            if not (out/'result.json').exists():
                (out/'result.json').write_text(json.dumps({'passed':False,'error':'Observer exited without completing report; inspect process.log and safety.json','lifecycle':final},indent=2)+'\n')
            assert all(final[k] for k in ['foregroundUnchanged','ownedProcessExited','ownerPreferencesUnchanged','inputsUnchanged','productionUnchanged','appUnchanged'])
    result = json.loads((out/'result.json').read_text())
    assert result['foregroundUnchanged']
    assert all(digest(profile/p) == expected for p, expected in manifest['profileHashes'].items())
    assert len(result['observations'])==5 and all(len(o['cellHitTests'])==(0 if o['panel']=='layers' else 4) for o in result['observations'])
    assert result['prohibitedActivation'] and all(o['windowAlpha']==0 and not o['windowKey'] for o in result['observations'])
    assert all(x['productionRoundTripMatches'] for x in result['pasteboards'])
    registrations=[]
    for observation in result['observations']:
        destinations=[v for v in observation['otherDestinationAncestry']
                      if {'public.data','public.item'}.issubset(v['view']['registeredTypes'])]
        expected=1 if observation['panel']=='layers' or args.expect_inactive_registered else 0
        assert len(destinations)==expected, (observation['panel'],len(destinations),expected)
        if destinations:
            destination=destinations[0]
            assert all(point['covered'] for point in destination['coversRecordedColorPoints'])
            transparent=any(view['alpha']==0 for view in destination['ancestry'])
            assert transparent==(observation['panel']!='layers')
        registrations.append({'panel':observation['panel'],'genericDataDestinations':len(destinations)})
    native_color=next(t for t in result['pasteboardTypeConformance'] if t['identifier']=='com.apple.cocoa.pasteboard.color')
    gradient=next(t for t in result['pasteboardTypeConformance'] if t['identifier']=='com.vixen.gradient')
    assert native_color['conformsToData'] and native_color['conformsToItem']
    assert not gradient['conformsToData'] and not gradient['conformsToItem']
    validation={'passed':True,'panelStates':5,'cellHitTests':16,'foregroundUnchanged':True,
                'expectedInactiveRegistrations':args.expect_inactive_registered,
                'registrations':registrations,'productAcceptance':False}
    (out/'routing-validation.json').write_text(json.dumps(validation,indent=2)+'\n')
    print(json.dumps(validation))

if __name__ == '__main__':main()
