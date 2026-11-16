#!/usr/bin/env python3
"""Actual system Color/Font panels in an owned host with no Core or show.

Prepare freely; --run-prepared requires the coordinated foreground slot.
Uses only observed native AX controls, real session callbacks, and normal closes.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


class RGBReadinessError(AssertionError):
    """Observed panel stayed owned, but did not settle within its local readiness budget."""


def failure_kind(error, now, deadline, ownership, color_write_attempted):
    if isinstance(error, subprocess.TimeoutExpired) or now >= deadline: return 'timeout'
    if isinstance(error, RGBReadinessError) and ownership and not color_write_attempted: return 'diagnostic'
    return 'fatal'


def rgb_field_ready(snapshot, title, expected):
    """A settled, closed RGB picker with the exact existing value, never a stale AX tree."""
    rows = snapshot.get('rows', [])
    assert snapshot.get('foregroundOwned') is True, 'Panel lost foreground ownership'
    assert all(r.get('window') == title for r in rows), 'Foreign panel in snapshot'
    popups = [r for r in rows if r.get('role') == 'AXPopUpButton']
    fields = [r for r in rows if r.get('role') == 'AXTextField' and r.get('value') == expected]
    assert len(popups) <= 1 and len(fields) <= 1, 'Ambiguous RGB controls'
    return (bool(rows) and all(r.get('role') for r in rows) and
            not any(r.get('role') in ('AXMenu', 'AXMenuItem') for r in rows) and
            len(popups) == 1 and popups[0].get('value') == 'RGB Sliders' and
            len(fields) == 1 and 'AXConfirm' in fields[0].get('actions', []))


def prepare_rgb_field(call, title, expected, deadline, *, prefix='color', clock=time.monotonic, pause=time.sleep):
    """Reacquire native controls after mode changes; at most five seconds inside the outer budget."""
    assert len(expected) == 6 and all(c in '0123456789ABCDEF' for c in expected)
    end = min(deadline, clock()+5)
    sequence = 0

    def invoke(label, action, selector):
        nonlocal sequence
        if clock() >= end: raise RGBReadinessError('RGB picker readiness deadline')
        sequence += 1
        return call(f'{prefix}-rgb-{expected}-{label}-{sequence}', action, selector, until=end)

    def snapshot(label):
        value = invoke(label, 'dump', {'window': title})
        # Also rejects ownership changes/ambiguous matches on incomplete snapshots.
        rgb_field_ready(value, title, expected)
        return value

    def poll(label, predicate):
        while clock() < end:
            value = snapshot(label)
            if predicate(value): return value
            pause(min(.1, max(0, end-clock())))
        raise RGBReadinessError('RGB picker did not reach '+label+' before deadline')

    observed = poll('initial', lambda s: any(r.get('role') == 'AXPopUpButton' or
        (r.get('role') == 'AXButton' and r.get('description') == 'Color Sliders') for r in s['rows']))
    if rgb_field_ready(observed, title, expected): return observed
    popups = [r for r in observed['rows'] if r.get('role') == 'AXPopUpButton']
    if not popups:
        buttons = [r for r in observed['rows'] if r.get('role') == 'AXButton' and r.get('description') == 'Color Sliders']
        assert len(buttons) == 1, 'Observed Color Sliders button unavailable'
        invoke('sliders', 'press', {'window': title, 'role': 'AXButton', 'description': 'Color Sliders'})
        observed = poll('popup', lambda s: len([r for r in s['rows'] if r.get('role') == 'AXPopUpButton' and r.get('value')]) == 1)
        popups = [r for r in observed['rows'] if r.get('role') == 'AXPopUpButton']
    if not popups[0].get('value'):
        observed = poll('popup-value', lambda s: len([r for r in s['rows'] if r.get('role') == 'AXPopUpButton' and r.get('value')]) == 1)
        popups = [r for r in observed['rows'] if r.get('role') == 'AXPopUpButton']
    if popups[0].get('value') != 'RGB Sliders':
        assert popups[0].get('value') in ('Grayscale Slider', 'CMYK Sliders', 'HSB Sliders'), 'Unrecognized slider mode'
        invoke('menu', 'press', {'window': title, 'role': 'AXPopUpButton', 'value': popups[0]['value']})
        menu = poll('rgb-menu', lambda s: len([r for r in s['rows'] if r.get('role') == 'AXMenuItem' and r.get('title') == 'RGB Sliders']) == 1)
        rgb = [r for r in menu['rows'] if r.get('role') == 'AXMenuItem' and r.get('title') == 'RGB Sliders']
        assert 'AXPress' in rgb[0].get('actions', []), 'RGB menu item cannot be pressed'
        invoke('select', 'press', {'window': title, 'role': 'AXMenuItem', 'title': 'RGB Sliders'})

    cancelled_tracking = False
    while clock() < end:
        observed = snapshot('settle')
        if rgb_field_ready(observed, title, expected): return observed
        popups = [r for r in observed['rows'] if r.get('role') == 'AXPopUpButton']
        rgb = [r for r in observed['rows'] if r.get('role') == 'AXMenuItem' and r.get('title') == 'RGB Sliders']
        # AXPress can select the mode while its tracking menu remains open. Dismiss only
        # that observed owned menu, once; no Escape key or repeat selection/color edit.
        if not cancelled_tracking and len(popups) == 1 and popups[0].get('value') == 'RGB Sliders' and len(rgb) == 1 and 'AXCancel' in rgb[0].get('actions', []):
            invoke('dismiss-menu', 'cancel', {'window': title, 'role': 'AXMenuItem', 'title': 'RGB Sliders'})
            cancelled_tracking = True
        pause(min(.1, max(0, end-clock())))
    raise RGBReadinessError('RGB hexadecimal field unavailable after bounded fresh-state polling')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--build', type=Path, default=ROOT/'target/build-main')
    parser.add_argument('--configuration', choices=['debug', 'release'], default='debug')
    parser.add_argument('--ownership', action='store_true')
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args()
    out = args.out.resolve()
    assert out.is_relative_to(ROOT/'target')
    app = out/'SystemPanels.app'
    executable = app/'Contents/MacOS/SystemPanels'
    driver = out/'system-panel-driver'
    manifest_path = out/'prepared.json'
    if not args.run_prepared:
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        build = args.build.resolve()
        assert build.is_relative_to(ROOT/'target')
        products = build/'swift/out/Products'/args.configuration.title()
        frozen = out/'compiled-inputs'; frozen.mkdir()
        modules = frozen/'modules'; modules.mkdir()
        copied = {}; originals = {}
        def pin(source, destination):
            destination.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(['/bin/cp', '-c', str(source), str(destination)], check=True)
            assert digest(source) == digest(destination)
            copied[str(destination)] = digest(destination)
            originals[str(source)] = digest(source)
            return destination
        for module in ['VixenKit', 'VixenBridge']:
            for source in (products/(module+'.swiftmodule')).rglob('*'):
                if source.is_file(): pin(source, modules/(module+'.swiftmodule')/source.relative_to(products/(module+'.swiftmodule')))
        objects = [pin(products/(name+'.o'), frozen/(name+'.o')) for name in ['VixenKit', 'VixenBridge', 'vixen_ffiFFI']]
        library = pin(build/'pkg/lib/libvixen_ffi.a', frozen/'libvixen_ffi.a')
        header = pin(build/'pkg/Sources/vixen_ffiFFI/include/vixen_ffiFFI.h', frozen/'include/vixen_ffiFFI.h')
        pin(build/'pkg/Sources/VixenBridge/vixen_ffi.swift', frozen/'vixen_ffi.swift')
        modulemap = frozen/'vixen_ffiFFI.modulemap'
        modulemap.write_text('module vixen_ffiFFI { header "'+str(header)+'" export * }\n')
        copied[str(modulemap)] = digest(modulemap)
        host_source = pin(ROOT/'scripts/system_panel_host.swift', out/'host-source.swift')
        driver_source = pin(ROOT/'scripts/system_panel_driver.swift', out/'driver-source.swift')
        pin(ROOT/'scripts/check_system_panels.py', out/'runner-source.py')
        production = {}
        for name in ['ColorControls.swift', 'FontPicker.swift']:
            source = ROOT/'app/Sources/VixenKit'/name
            destination = pin(source, out/'production-source'/name)
            production[str(source)] = {'sha256': digest(source), 'snapshot': str(destination)}
        command = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library',
                   '-swift-version', '6', '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0',
                   '-I', str(modules), '-Xcc', '-fmodule-map-file='+str(modulemap), '-I', str(header.parent),
                   str(host_source), *map(str, objects), str(library), '-lc++', '-lcurl']
        for framework in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation',
                          'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
            command += ['-framework', framework]
        command += ['-o', str(executable)]
        if args.configuration == 'release': command += ['-O']
        with (out/'compile.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
            subprocess.run(['/usr/bin/swiftc', '-module-cache-path', str(out/'driver-clang-cache'), str(driver_source), '-o', str(driver)],
                           stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        identifier = 'com.vixenlights.verification.systempanels.'+uuid.uuid4().hex
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': identifier,
            'CFBundleExecutable': 'SystemPanels', 'CFBundleName': 'Private System Panel Verification',
            'CFBundlePackageType': 'APPL', 'LSMinimumSystemVersion': '26.0'}))
        for path in [app, driver]:
            subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(path)], check=True, capture_output=True, timeout=30)
            subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(path)], check=True, capture_output=True, timeout=30)
        manifest = {'bundleIdentifier': identifier, 'executable': str(executable), 'executableSha256': digest(executable),
                    'driver': str(driver), 'driverSha256': digest(driver), 'compileCommand': command,
                    'pinnedInputHashes': copied, 'originalInputHashes': originals, 'productionHashes': production,
                    'build': str(build), 'configuration': args.configuration,
                    'hostSourceSha256': digest(host_source), 'driverSourceSha256': digest(driver_source),
                    'ownership': args.ownership, 'watchdogSeconds': 45, 'inputDeadlineSeconds': 30,
                    'cleanupBoundSeconds': 14, 'requestedIdleSeconds': 60,
                    'noCoreOrShowOpened': True, 'scope': __doc__}
        manifest_path.write_text(json.dumps(manifest, indent=2)+'\n')
    if args.prepare_only:
        print(manifest_path); return
    manifest = json.loads(manifest_path.read_text())
    assert digest(executable) == manifest['executableSha256'] and digest(driver) == manifest['driverSha256']
    assert manifest['ownership'] == args.ownership
    assert all(digest(Path(p)) == h for p, h in manifest['pinnedInputHashes'].items())
    owner_paths = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist', Path.home()/'Library/Application Support/Vixen/Settings.xml']
    owner_before = {str(p): digest(p) for p in owner_paths}
    assert not (out/'events.json').exists()
    config = dict(manifest)
    config_path = out/'config.json'
    actions = []
    deadline = time.monotonic()+30
    process = None
    phase = 'startup'
    report = {'passed': False, 'appSha256': manifest['executableSha256'], 'actions': actions,
              'hostPID': None, 'driverPIDs': [], 'driverProcesses': [],
              'appExecutable': str(executable), 'driverExecutable': str(driver),
              'bundleIdentifier': manifest['bundleIdentifier'], 'processExited': True,
              'processExitCode': None, 'cleanupCompleted': False, 'noInputHeld': True,
              'inputMethod': 'AX only; no CG posts or global key events',
              'inputOwnershipMaintained': True, 'firstColorWriteAttempted': False,
              'failurePhase': None, 'failureKind': None, 'firstError': None, 'finalizationErrors': []}

    def call(name, action, selector=None, value=None, *, until=None):
        assert action == 'restore' or time.monotonic() < deadline, 'outer deadline'
        call_end = time.monotonic()+8 if action == 'restore' else min(deadline, until if until is not None else deadline, time.monotonic()+8)
        assert time.monotonic() < call_end, 'action deadline'
        output = out/(name+'.ax.json')
        config['output'] = str(output)
        config['actionDeadlineEpoch'] = time.time()+max(0, call_end-time.monotonic())
        config_path.write_text(json.dumps(config, indent=2)+'\n')
        cmd = [str(driver), str(config_path), action]
        if selector is not None: cmd.append(json.dumps(selector))
        if value is not None: cmd.append(value)
        began = time.monotonic()
        child = None
        helper = {'name': name, 'executable': str(driver), 'pid': None, 'exited': False, 'exitCode': None}
        report['driverProcesses'].append(helper)
        try:
            with (out/(name+'.driver.log')).open('w') as log:
                child = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT)
                helper['pid'] = child.pid; report['driverPIDs'].append(child.pid)
                child.wait(timeout=max(.001, call_end-time.monotonic()))
            assert child.returncode == 0, (name, child.returncode)
        except BaseException:
            report['inputOwnershipMaintained'] = False
            raise
        finally:
            if child is not None and child.poll() is None:
                child.kill(); child.wait(timeout=.5)
            helper.update(exited=child is not None and child.poll() is not None,
                          exitCode=child.returncode if child is not None else None)
        result = json.loads(output.read_text())
        normal_finish = (name == 'finish' and action == 'press' and
            selector == {'window': 'Private System Panel Verification', 'role': 'AXButton', 'title': 'Finish Check'} and
            result.get('ownedHostTerminated') is True and process is not None and process.poll() == 0)
        if action not in ('desktop', 'restore') and result.get('foregroundOwned') is not True and not normal_finish:
            report['inputOwnershipMaintained'] = False
            raise AssertionError('Driver did not retain input ownership')
        actions.append({'name': name, 'elapsedSeconds': time.monotonic()-began, 'result': result})
        return result

    def events():
        return json.loads((out/'events.json').read_text())['events'] if (out/'events.json').exists() else []

    def wait_event(event, predicate=lambda event: True):
        end = min(deadline, time.monotonic()+5)
        while time.monotonic() < end:
            matching = [e for e in events() if e['event'] == event and predicate(e)]
            if matching: return matching[-1]
            assert process.poll() is None, 'host exited before '+event
            time.sleep(.05)
        raise AssertionError('Missing event '+event)

    host_title = 'Private System Panel Verification'
    color_title = 'Private Color Check'
    host_button = lambda title: {'window': host_title, 'role': 'AXButton', 'title': title}
    close = lambda title: {'window': title, 'role': 'AXButton', 'subrole': 'AXCloseButton'}
    try:
        previous = call('desktop-before', 'desktop')['application']
        assert 'securityagent' not in previous['bundleIdentifier'].lower()
        config['previousApplication'] = previous
        environment = {k: v for k, v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
        (out/'runtime').mkdir()
        environment['TMPDIR'] = str(out/'runtime')+'/'
        with (out/'process.log').open('w') as log:
            process = subprocess.Popen([str(executable), str(out/'events.json')], env=environment,
                                       stdout=log, stderr=subprocess.STDOUT)
            config['pid'] = process.pid
            report['hostPID'] = process.pid
            wait_event('ready')
            call('activate', 'activate')
            call('open-color', 'press', host_button('Open Color'))
            wait_event('color-opened')
            phase = 'rgb_control_readiness'
            prepare_rgb_field(call, color_title, '0000FF', deadline, prefix='initial')
            phase = 'color_write'; report['firstColorWriteAttempted'] = True
            call('color-red', 'set-rgb', {'window': color_title, 'role': 'AXTextField', 'value': '0000FF'}, 'FF0000')
            live = wait_event('color-live', lambda event: event['argb'] == 0xffff0000)
            assert live['argb'] == 0xffff0000, live
            expected_colors = [0xffff0000]
            if args.ownership:
                phase = 'native_color_ownership'
                initial_owner = wait_event('shared-owner-ready', lambda event: event['session'] == 1)
                assert initial_owner['panelVisible'] and initial_owner['panelAlpha'] == 1 and not initial_owner['testMode']
                assert not initial_owner['nativeWellActive'] and initial_owner['nativeWellColor'] == 0xff00ff00
                call('native-takeover', 'press', host_button('Use Native Well'))
                native = wait_event('native-owner-ready')
                assert native['active'] and native['argb'] == 0xff00ff00 and native['panelColor'] == 0xff00ff00, native
                assert [e['argb'] for e in events() if e['event'] == 'color-committed'] == [0xffff0000]
                native_title = native['panelTitle']
                prepare_rgb_field(call, native_title, '00FF00', deadline, prefix='native')
                call('native-blue', 'set-rgb', {'window': native_title, 'role': 'AXTextField', 'value': '00FF00'}, '0000FF')
                changed_well = wait_event('native-color-changed', lambda event: event['argb'] == 0xff0000ff)
                assert changed_well['active']
                call('shared-retakeover', 'press', host_button('Open Color'))
                shared = wait_event('shared-owner-ready', lambda event: event['session'] == 2)
                assert not shared['nativeWellActive'] and shared['nativeWellColor'] == 0xff0000ff, shared
                assert shared['panelVisible'] and shared['panelAlpha'] == 1 and not shared['testMode'], shared
                prepare_rgb_field(call, color_title, '0000FF', deadline, prefix='shared')
                call('shared-cyan', 'set-rgb', {'window': color_title, 'role': 'AXTextField', 'value': '0000FF'}, '00FFFF')
                wait_event('color-live', lambda event: event['session'] == 2 and event['argb'] == 0xff00ffff)
                expected_colors += [0xff00ffff]
            phase = 'color_close'
            call('close-color', 'press', close(color_title))
            wait_event('color-window-closed')
            assert [e['argb'] for e in events() if e['event'] == 'color-committed'] == expected_colors
            phase = 'font_input'
            call('open-font', 'press', host_button('Open Font'))
            initial = wait_event('font-initial-selection')
            assert initial['family'] == 'Arial' and initial['size'] == 12, initial
            report['initialFontSelection'] = initial
            font = call('font-open', 'dump', {'window': 'Fonts'})
            size = [r for r in font['rows'] if r['role'] == 'AXComboBox' and r['description'] == 'Font Size']
            assert len(size) == 1
            report['initialFontAXValue'] = size[0]['value']
            changed = call('font-eighteen', 'set', {'window': 'Fonts', 'role': 'AXComboBox', 'description': 'Font Size'}, '18')
            assert float(changed['confirmedValue']) == 18
            call('close-font', 'press', close('Fonts'))
            wait_event('font-window-closed')
            fonts = [e for e in events() if e['event'] == 'font-committed']
            assert len(fonts) == 1 and fonts[0]['name'] == 'Arial' and fonts[0]['size'] == 18 and not fonts[0]['bold'] and not fonts[0]['italic'], fonts
            phase = 'normal_finish'
            call('finish', 'press', host_button('Finish Check'))
            process.wait(timeout=max(.1, min(8, deadline-time.monotonic())))
            assert process.returncode == 0
        final = events()
        names = [e['event'] for e in final]
        assert 'watchdog' not in names and names[-2:] == ['finished', 'terminated'], names
        assert names.index('font-committed') < names.index('finished')
        assert names.index('font-window-closed') < names.index('finished')
        assert [e['argb'] for e in final if e['event'] == 'color-committed'] == expected_colors
        if args.ownership:
            final_well = [e for e in final if e['event'] == 'native-well-final']
            assert len(final_well) == 1 and not final_well[0]['active'] and final_well[0]['argb'] == 0xff0000ff
            assert all(e['argb'] != 0xff00ffff for e in final if e['event'] == 'native-color-changed')
        report.update(passed=True, normalCloseVerified=True, noWatchdog=True, exitCode=process.returncode,
                      events=final, ownershipVerified=args.ownership,
                      scope='Actual native ColorPanel RGB input and FontPanel size input via AXConfirm; normal close commits exactly once before explicit host finish; no test-set hooks. Optional native-well handoff uses an explicit host button invoking real activate(true), not a physical well-click claim.')
    except BaseException as error:
        report['error'] = repr(error)
        report['firstError'] = repr(error); report['failurePhase'] = phase
        report['failureKind'] = failure_kind(error, time.monotonic(), deadline,
            report['inputOwnershipMaintained'], report['firstColorWriteAttempted'])
        raise
    finally:
        if process and process.poll() is None:
            try:
                process.terminate()
                try: process.wait(timeout=3)
                except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=3)
            except BaseException as error:
                report['finalizationErrors'].append({'phase': 'owned_host_cleanup', 'error': repr(error)})
                report['passed'] = False
        report['processExited'] = process is None or process.poll() is not None
        report['processExitCode'] = process.returncode if process else None
        if process:
            try:
                restoration = call('desktop-restored', 'restore')
                report['restoredPrevious'] = restoration['restoredPrevious']
                assert report['restoredPrevious']
            except BaseException as error:
                report['restorationError'] = repr(error); report['passed'] = False
                report['finalizationErrors'].append({'phase': 'foreground_restoration', 'error': repr(error)})
        report['elapsedSeconds'] = 30-(deadline-time.monotonic())
        report['ownerPreferenceHashesBefore'] = owner_before
        def final_hash(path):
            try: return digest(path)
            except OSError as error:
                report['finalizationErrors'].append({'phase': 'final_hash', 'path': str(path), 'error': repr(error)})
                return None
        report['ownerPreferenceHashesAfter'] = {str(p): final_hash(p) for p in owner_paths}
        report['ownerPreferencesUnchanged'] = report['ownerPreferenceHashesAfter'] == owner_before
        report['inputsUnchanged'] = all(final_hash(Path(p)) == h for p, h in manifest['pinnedInputHashes'].items())
        report['appUnchanged'] = final_hash(executable) == manifest['executableSha256']
        report['driverUnchanged'] = final_hash(driver) == manifest['driverSha256']
        report['cleanupCompleted'] = report['processExited'] and all(p['exited'] for p in report['driverProcesses']) and not report['finalizationErrors']
        report['normalNonTestPath'] = not any(e.get('testMode', False) for e in events())
        if not all(report[k] for k in ['ownerPreferencesUnchanged', 'inputsUnchanged', 'appUnchanged', 'driverUnchanged', 'cleanupCompleted']): report['passed'] = False
        if report['finalizationErrors'] and report['failurePhase'] is None:
            report['failurePhase'] = 'cleanup'; report['failureKind'] = 'fatal'
        (out/'owned-processes.json').write_text(json.dumps({'appPID': report['hostPID'], 'appExecutable': str(executable),
            'bundleIdentifier': manifest['bundleIdentifier'], 'driverPIDs': report['driverPIDs'], 'driverExecutable': str(driver)}, indent=2)+'\n')
        (out/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    assert report['passed'], report
    print(json.dumps({k: report[k] for k in ['passed', 'normalCloseVerified', 'elapsedSeconds', 'restoredPrevious']}))


if __name__ == '__main__':
    main()
