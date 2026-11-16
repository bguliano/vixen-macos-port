#!/usr/bin/env python3
"""Prepare a frozen optimized host, or run only in a coordinated foreground slot.

Real WindowServer gestures, passive production values/tooltips, no Core or show.
Preparation compiles and signs only; it never launches the host or input driver.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


def preferences():
    paths = [Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist',
             Path.home()/'Library/Application Support/Vixen/Settings.xml']
    return {str(p): digest(p) if p.exists() else None for p in paths}


def main():
    began = time.monotonic(); end = began+63
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--build', type=Path, default=ROOT/'target/build-release')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-only', action='store_true')
    mode.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args()
    out, build = args.out.resolve(), args.build.resolve()
    assert out.is_relative_to(ROOT/'target') and build.is_relative_to(ROOT/'target')
    app = out/'InlineGestures.app'
    executable = app/'Contents/MacOS/InlineGestures'
    driver = out/'inline-gesture-driver'
    if args.prepare_only:
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        frozen = out/'compiled-inputs'; modules = frozen/'modules'
        modules.mkdir(parents=True)
        copied, originals = {}, {}
        def pin(source, destination):
            destination.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(['/bin/cp','-c',str(source),str(destination)],check=True)
            assert digest(source) == digest(destination)
            originals[str(source)] = copied[str(destination)] = digest(source)
            return destination
        products = build/'swift/out/Products/Release'
        for module in ['VixenKit','VixenBridge']:
            for source in (products/(module+'.swiftmodule')).rglob('*'):
                if source.is_file(): pin(source, modules/(module+'.swiftmodule')/source.relative_to(products/(module+'.swiftmodule')))
        objects = [pin(products/(name+'.o'),frozen/(name+'.o')) for name in ['VixenKit','VixenBridge','vixen_ffiFFI']]
        library = pin(build/'pkg/lib/libvixen_ffi.a',frozen/'libvixen_ffi.a')
        header = pin(build/'pkg/Sources/vixen_ffiFFI/include/vixen_ffiFFI.h',frozen/'include/vixen_ffiFFI.h')
        pin(build/'pkg/Sources/VixenBridge/vixen_ffi.swift',frozen/'vixen_ffi.swift')
        modulemap = frozen/'vixen_ffiFFI.modulemap'
        modulemap.write_text('module vixen_ffiFFI { header "'+str(header)+'" export * }\n')
        copied[str(modulemap)] = digest(modulemap)
        host_source = pin(ROOT/'scripts/inline_gesture_host.swift',out/'host-source.swift')
        driver_source = pin(ROOT/'scripts/inline_gesture_driver.swift',out/'driver-source.swift')
        pin(Path(__file__).resolve(),out/'runner-source.py')
        production = {}
        for name in ['CurveStripView.swift','GradientStripView.swift','KitSupport.swift','ValueDragView.swift']:
            source = ROOT/'app/Sources/VixenKit'/name
            if source.exists():
                saved = pin(source,out/'production-source'/name)
                production[str(source)] = {'sha256':digest(source),'snapshot':str(saved)}
        build_report = pin(ROOT/'target/native-curve-tooltip-build-report.json',out/'source-build-report.json')
        pin(ROOT/'target/native-curve-tooltip-build-inputs.json',out/'source-build-inputs.json')
        reference = pin(ROOT/'target/curve-handle-native-fixed-01/prepared.json',out/'optimized-handler-reference.json')
        pin(ROOT/'target/curve-handle-native-fixed-01/result.json',out/'optimized-handler-reference-result.json')
        reference_inputs = json.loads(reference.read_text())['pinnedInputHashes']
        for item in [*objects,library]:
            expected = [h for p,h in reference_inputs.items() if Path(p).name == item.name]
            assert expected == [digest(item)], 'Optimized dependency differs from verified37-handler input: '+item.name
        command = ['/usr/bin/swiftc','-module-cache-path',str(out/'clang-cache'),'-parse-as-library',
            '-swift-version','6','-default-isolation','MainActor','-target','arm64-apple-macos26.0','-O',
            '-I',str(modules),'-Xcc','-fmodule-map-file='+str(modulemap),'-I',str(header.parent),
            str(host_source),*map(str,objects),str(library),'-lc++','-lcurl']
        for framework in ['AppKit','SwiftUI','AudioToolbox','AudioUnit','CoreAudio','CoreFoundation','CoreGraphics',
                          'CoreText','Foundation','IOKit','Metal','QuartzCore','Security','SystemConfiguration']:
            command += ['-framework',framework]
        command += ['-o',str(executable)]
        driver_command = ['/usr/bin/swiftc','-parse-as-library','-swift-version','6','-default-isolation','MainActor',
            '-module-cache-path',str(out/'driver-clang-cache'),str(driver_source),'-o',str(driver)]
        with (out/'compile.log').open('w') as log:
            for cmd in [command,driver_command]:
                subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        identifier = 'com.vixenlights.verification.inlinegestures.'+uuid.uuid4().hex
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':identifier,
            'CFBundleExecutable':'InlineGestures','CFBundleName':'Private Inline Gesture Verification',
            'CFBundlePackageType':'APPL','LSMinimumSystemVersion':'26.0'}))
        for item in [app,driver]:
            subprocess.run(['/usr/bin/codesign','--force','--sign','-',str(item)],check=True,capture_output=True,timeout=30)
            subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(item)],check=True,capture_output=True,timeout=30)
        manifest = {'stageName':'inline-gestures','executable':str(executable),'driver':str(driver),
            'executableSha256':digest(executable),'driverSha256':digest(driver),'bundleIdentifier':identifier,
            'pinnedInputHashes':copied,'originalInputHashes':originals,'productionHashes':production,
            'sourceReleaseSha256':digest(build/'Vixen.app/Contents/MacOS/Vixen'),
            'sourceBuildReportSha256':digest(build_report),
            'command':[sys.executable,'-B',str(Path(__file__).resolve()),'--out',str(out),'--run-prepared'],
            'ownedExecutables':[str(executable),str(driver)],'timeoutSeconds':75,
            'compileCommand':command,'driverCompileCommand':driver_command,
            'inputDeadlineSeconds':40,'driverCleanupDeadlineSeconds':48,'hostWatchdogSeconds':55,
            'runnerDeadlineSeconds':63,'requestedIdleSeconds':75,'launchPerformed':False,
            'scope':'Three actual WindowServer gestures into optimized native inline views; Shift level25 to40, curve midpoint50/50 to70/75, paired gradient stops.25 to.40. Visible native tooltip and deferred single release commit; no Core/show/handler replacement.'}
        (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
        print(json.dumps({'prepared':str(out/'prepared.json'),'launched':False}));return

    m = json.loads((out/'prepared.json').read_text())
    assert m['executable'] == str(executable) and m['driver'] == str(driver)
    assert digest(executable) == m['executableSha256'] and digest(driver) == m['driverSha256']
    assert all(digest(p) == h for p,h in m['pinnedInputHashes'].items())
    assert digest(__file__) == m['originalInputHashes'][str(Path(__file__).resolve())]
    assert not (out/'events.json').exists() and not (out/'result.json').exists()
    for path in [app,driver]:
        subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(path)],check=True,capture_output=True,timeout=3)
    before = preferences(); process = helper = None
    report = {'passed':False,'processExited':False,'cleanupCompleted':False,'noInputHeld':False,
        'restoredPrevious':False,'inputsUnchanged':False,'appUnchanged':False,'driverUnchanged':False,
        'ownerPreferencesUnchanged':False,'ownerPreferenceHashesBefore':before,'failureKind':None,
        'appExecutable':str(executable),'driverExecutable':str(driver),'scope':m['scope'],'finalizationErrors':[]}
    try:
        assert end-time.monotonic() >= 59, 'Insufficient absolute interval for desktop check and complete native run'
        subprocess.run([str(driver),'--desktop',str(out/'desktop-before.json')],check=True,timeout=min(3,end-time.monotonic()))
        desktop = json.loads((out/'desktop-before.json').read_text())
        assert desktop['accessibilityTrusted'] and desktop['eventPostingPreflight']
        previous = desktop['application']; report['previousApplication'] = previous
        assert previous['pid'] > 1 and previous['executable'] and previous['bundleIdentifier']
        assert 'securityagent' not in previous['bundleIdentifier'].lower()
        env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
        env['VIXEN_SETTINGS_DIR'] = str(out/'private-settings')
        with (out/'host.log').open('w') as log, (out/'driver.log').open('w') as driver_log:
            assert end-time.monotonic() >= 56, 'Insufficient absolute interval; no host launched'
            process = subprocess.Popen([str(executable),str(out/'events.json')],env=env,stdout=log,stderr=subprocess.STDOUT)
            config = dict(m,pid=process.pid,output=str(out/'foreground.json'),events=str(out/'events.json'),previousApplication=previous)
            (out/'config.json').write_text(json.dumps(config,indent=2)+'\n')
            report['hostPID'] = process.pid
            assert end-time.monotonic() >= 56, 'Insufficient absolute interval; no input helper launched'
            helper = subprocess.Popen([str(driver),str(out/'config.json')],stdout=driver_log,stderr=subprocess.STDOUT)
            report['driverPID'] = helper.pid
            (out/'owned-processes.json').write_text(json.dumps({'appPID':process.pid,'appExecutable':str(executable),
                'driverPID':helper.pid,'driverExecutable':str(driver),'bundleIdentifier':m['bundleIdentifier']},indent=2)+'\n')
            helper.wait(timeout=min(50,max(.001,end-time.monotonic()-6)))
            assert helper.returncode == 0
            fg = json.loads((out/'foreground.json').read_text()); report['foreground'] = fg
            assert fg['passed'] and fg['targetExited'] and fg['noInputHeld'] and fg['restoredPrevious'] and fg['normalFinish']
            process.wait(timeout=min(2,max(.001,end-time.monotonic())))
            assert process.returncode == 0
            data = json.loads((out/'events.json').read_text()); report['finalObservation'] = data['current']
            assert data['terminated'] and not data['testMode']
            assert data['current']['commits'] == {'level':1,'point':1,'gradient':1}
            assert data['current']['tooltips'] == []
            report.update(passed=True,normalFinish=True,observations=len(fg['observations']))
    except BaseException as error:
        report['error'] = repr(error);report['failureKind'] = 'fatal'
        raise
    finally:
        for name, child in [('driver',helper),('host',process)]:
            if child is not None and child.poll() is None:
                try:
                    child.terminate()
                    try: child.wait(timeout=min(2,max(.001,end-time.monotonic())))
                    except subprocess.TimeoutExpired:
                        child.kill();child.wait(timeout=min(1,max(.001,end-time.monotonic())))
                except BaseException as error: report['finalizationErrors'].append({'phase':name+'-cleanup','error':repr(error)})
        report['processExited'] = process is None or process.poll() is not None
        report['processExitCode'] = process.returncode if process else None
        report['driverExited'] = helper is None or helper.poll() is not None
        report['driverExitCode'] = helper.returncode if helper else None
        path = out/'foreground.json'
        if path.exists():
            try:
                fg = json.loads(path.read_text());report['foreground'] = fg
                report['noInputHeld'] = fg.get('noInputHeld') is True
                report['restoredPrevious'] = fg.get('restoredPrevious') is True
            except (OSError,ValueError) as error: report['finalizationErrors'].append({'phase':'helper-report','error':repr(error)})
        def check_hashes():
            try:
                report['inputsUnchanged'] = all(digest(p) == h for p,h in m['pinnedInputHashes'].items())
                report['appUnchanged'] = digest(executable) == m['executableSha256']
                report['driverUnchanged'] = digest(driver) == m['driverSha256']
                report['ownerPreferenceHashesAfter'] = preferences()
                report['ownerPreferencesUnchanged'] = report['ownerPreferenceHashesAfter'] == before
            except OSError as error: report['finalizationErrors'].append({'phase':'final-hashes','error':repr(error)})
        check_hashes()
        report['cleanupCompleted'] = report['processExited'] and report['driverExited'] and not report['finalizationErrors']
        report['elapsedSeconds'] = time.monotonic()-began
        required = ['processExited','cleanupCompleted','noInputHeld','restoredPrevious','inputsUnchanged','appUnchanged','driverUnchanged','ownerPreferencesUnchanged']
        if not all(report[key] for key in required) or time.monotonic() > end: report['passed'] = False
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    assert report['passed'],report
    print(json.dumps({'passed':True,'observations':report['observations'],'elapsedSeconds':report['elapsedSeconds']}))


if __name__ == '__main__': main()
