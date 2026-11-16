#!/usr/bin/env python3
"""Prepare a normal Element Mapper file-panel check; run only with a coordinated foreground slot."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import sys
import time
import unicodedata
import uuid
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
SOURCE_PROFILE = ROOT/'target/library-drag-fixture/profile'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
SOURCE_IDS = ['10101010-1010-4010-8010-101010101010', '20202020-2020-4020-8020-202020202020']
TARGET_IDS = ['30303030-3030-4030-8030-303030303030', '40404040-4040-4040-8040-404040404040']


def write(path, data):
    path.write_text(json.dumps(data, indent=2)+'\n')


def hashes(folder):
    return {str(p.relative_to(folder)): sha(p) for p in sorted(folder.rglob('*')) if p.is_file()}



def validate_private_profile(profile, initial):
    """Allow only normal shutdown's newly created empty ApplicationData file.

    The production writer is application_data_to_xml(ApplicationData::new()) in
    vixen-io/appdata.rs; AppCore::shutdown saves it even in an empty profile.
    Initial files remain byte-for-byte pinned. No new user data is permitted.
    """
    profile = Path(profile)
    links = [str(p.relative_to(profile)) for p in profile.rglob('*') if p.is_symlink()]
    if links:
        return {'accepted': False, 'strictlyUnchanged': False, 'initialFilesUnchanged': False,
                'reason': 'Profile contains symbolic links', 'symbolicLinks': links}
    current = hashes(profile)
    changed = [p for p,h in initial.items() if current.get(p) != h]
    added = sorted(set(current)-set(initial))
    result = {'accepted': False, 'strictlyUnchanged': current == initial,
              'initialFilesUnchanged': not changed, 'changedOrMissingInitialFiles': changed,
              'createdFiles': {p: current[p] for p in added}, 'allowedCreatedFiles': {}}
    if changed:
        result['reason'] = 'Initial profile file changed or disappeared'; return result
    if not added:
        result['accepted'] = True; return result
    if added != ['VixenApplicationData.xml']:
        result['reason'] = 'Unexpected profile output'; return result
    path = profile/'VixenApplicationData.xml'
    try:
        assert path.stat().st_size <= 4096, 'Oversized application data'
        text = path.read_text(encoding='utf-8-sig')
        assert '<!DOCTYPE' not in text.upper() and '<!ENTITY' not in text.upper(), 'DTD/entities are forbidden'
        body = text.lstrip()
        if body.startswith('<?xml '):
            declaration_end = body.find('?>'); assert declaration_end >= 0
            body = body[declaration_end+2:]
        assert '<!' not in body and '<?' not in body, 'Unexpected XML comments, declarations or processing instructions'
        parser = ET.XMLParser(target=ET.TreeBuilder(insert_comments=True, insert_pis=True))
        root = ET.fromstring(text, parser=parser)
        blank = lambda value: not (value or '').strip()
        assert root.tag == 'VixenApplicationData' and not root.attrib and blank(root.text) and blank(root.tail)
        assert [child.tag for child in root] == ['DataFormatVersion', 'RecentSequences', 'FilterShapePositions', 'FilterSetupFormHighQualityRendering']
        version, recent, positions, hq = list(root)
        assert not version.attrib and (version.text or '').strip() == '3'
        assert not recent.attrib and blank(recent.text) and not positions.attrib and blank(positions.text)
        assert hq.attrib == {'value': 'false'} and blank(hq.text)
        assert all(len(child) == 0 and blank(child.tail) for child in root)
    except (AssertionError, ValueError, ET.ParseError) as error:
        result['reason'] = 'New application data is not the empty production default: '+repr(error); return result
    result.update(accepted=True, allowedCreatedFiles={'VixenApplicationData.xml': current['VixenApplicationData.xml']},
                  reason='Normal shutdown created only the version3 empty production application-data document')
    return result


def prepare(out, build):
    assert not out.exists() and out.is_relative_to(ROOT/'target') and build.is_relative_to(ROOT/'target')
    app = out/'MapperPanels.app'; executable = app/'Contents/MacOS/MapperPanels'
    executable.parent.mkdir(parents=True)
    frozen = out/'compiled-inputs'; modules = frozen/'modules'; modules.mkdir(parents=True)
    pins = {}; originals = {}

    def pin(source, destination):
        destination.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(['/bin/cp', '-c', str(source), str(destination)], check=True)
        assert sha(source) == sha(destination)
        pins[str(destination)] = sha(destination); originals[str(source)] = sha(source)
        return destination

    products = build/'swift/out/Products/Release'
    for module in ['VixenKit', 'VixenBridge']:
        for src in (products/(module+'.swiftmodule')).rglob('*'):
            if src.is_file(): pin(src, modules/(module+'.swiftmodule')/src.relative_to(products/(module+'.swiftmodule')))
    objects = [pin(products/(name+'.o'), frozen/(name+'.o')) for name in ['VixenKit', 'VixenBridge', 'vixen_ffiFFI']]
    library = pin(build/'pkg/lib/libvixen_ffi.a', frozen/'libvixen_ffi.a')
    header = pin(build/'pkg/Sources/vixen_ffiFFI/include/vixen_ffiFFI.h', frozen/'include/vixen_ffiFFI.h')
    pin(build/'pkg/Sources/VixenBridge/vixen_ffi.swift', frozen/'vixen_ffi.swift')
    modulemap = frozen/'vixen_ffiFFI.modulemap'
    modulemap.write_text('module vixen_ffiFFI { header "'+str(header)+'" export * }\n'); pins[str(modulemap)] = sha(modulemap)
    production = [pin(ROOT/'app/Sources/VixenScheduler'/name, out/'production-source'/name)
                  for name in ['ElementMapperSheet.swift', 'SchedulerAlerts.swift']]
    host_source = pin(ROOT/'scripts/mapper_file_panel_host.swift', out/'host-source.swift')
    driver_source = pin(ROOT/'scripts/foreground_mapper_file_panels.swift', out/'driver-source.swift')
    pin(Path(__file__).resolve(), out/'runner-source.py')
    pin(ROOT/'crates/vixen-session/src/element_mapper.rs', out/'source-evidence/element_mapper.rs')
    for source in ['crates/vixen-io/src/appdata.rs', 'crates/vixen-session/src/boot.rs', 'crates/vixen-session/src/app.rs']:
        pin(ROOT/source, out/'source-evidence'/Path(source).name)
    pin(ROOT/'scripts/foreground_native_panels.swift', out/'source-evidence/foreground_native_panels.swift')
    command = ['/usr/bin/swiftc', '-module-cache-path', str(out/'clang-cache'), '-parse-as-library', '-swift-version', '6',
               '-default-isolation', 'MainActor', '-target', 'arm64-apple-macos26.0', '-O', '-module-name', 'MapperPanelProbe',
               '-enable-upcoming-feature', 'InferIsolatedConformances', '-enable-upcoming-feature', 'NonisolatedNonsendingByDefault',
               '-I', str(modules), '-Xcc', '-fmodule-map-file='+str(modulemap), '-I', str(header.parent),
               *map(str, production), str(host_source), *map(str, objects), str(library), '-lc++', '-lcurl']
    for name in ['AppKit', 'SwiftUI', 'AudioToolbox', 'AudioUnit', 'CoreAudio', 'CoreFoundation', 'CoreGraphics', 'CoreText', 'Foundation', 'IOKit', 'Metal', 'QuartzCore', 'Security', 'SystemConfiguration']:
        command += ['-framework', name]
    command += ['-o', str(executable)]
    driver = out/'foreground-mapper-file-panels'
    driver_command = ['/usr/bin/swiftc', '-module-cache-path', str(out/'driver-clang-cache'), str(driver_source), '-o', str(driver)]
    with (out/'compile.log').open('w') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
        subprocess.run(driver_command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
    identifier = 'com.vixenlights.verification.mapperpanels.'+uuid.uuid4().hex
    (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier': identifier, 'CFBundleExecutable': 'MapperPanels',
        'CFBundleName': 'Private Element Mapper Panels', 'CFBundlePackageType': 'APPL', 'LSMinimumSystemVersion': '26.0'}))
    for path in [app, driver]:
        subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', str(path)], check=True, capture_output=True, timeout=20)
        subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(path)], check=True, capture_output=True, timeout=20)
    profile = out/'profile'; (profile/'SystemData').mkdir(parents=True)
    source_profile_hashes = {}
    for name in ['SystemConfig.xml', 'ModuleStore.xml']:
        source = SOURCE_PROFILE/'SystemData'/name
        source_profile_hashes[str(source)] = sha(source)
        shutil.copy2(source, profile/'SystemData'/name)
    config = profile/'SystemData/SystemConfig.xml'; tree = ET.parse(config); root = tree.getroot()
    for tag in ['Channels', 'Nodes', 'Controllers', 'Previews', 'Patches']:
        assert len(root.find(tag)) == 0
    for i, target in enumerate(TARGET_IDS):
        channel = str(uuid.uuid5(uuid.NAMESPACE_URL, 'private-mapper-channel-'+target)); name = 'Destination '+chr(65+i)
        ET.SubElement(root.find('Channels'), 'Channel', id=channel, name=name)
        ET.SubElement(root.find('Nodes'), 'Node', id=target, channelId=channel, name=name)
    tree.write(config, encoding='utf-8', xml_declaration=True)
    inputs = out/'Input maps é'; inputs.mkdir(); destination = out/'Saved maps 雪'; destination.mkdir()
    source_map = inputs/'Input Café.vmap'; saved_map = destination/'Saved Noël.vmap'
    document = {'id': '50505050-5050-4050-8050-505050505050', 'name': source_map.stem, 'isDirty': True,
        'elementMappings': [{'sourceId': source, 'sourceName': 'Source '+chr(65+i), 'targetId': TARGET_IDS[i],
                             'targetName': 'Destination '+chr(65+i), 'isDirty': True} for i, source in enumerate(SOURCE_IDS)]}
    write(source_map, document)
    for name in ['private-settings', 'private-www', 'runtime']: (out/name).mkdir()
    host_config = {'profile': str(profile), 'wwwroot': str(out/'private-www'), 'eventsFile': str(out/'events.json'),
                   'sourceMap': str(source_map), 'sources': [{'id': source, 'name': 'Source '+chr(65+i)} for i, source in enumerate(SOURCE_IDS)]}
    write(out/'host-config.json', host_config)
    pins[str(out/'host-config.json')] = sha(out/'host-config.json'); pins[str(source_map)] = sha(source_map)
    m = {'preparedOnly': True, 'launched': False, 'app': str(app), 'executable': str(executable), 'executableSha256': sha(executable),
         'stageName': 'mapper-file-panels', 'command': [sys.executable, '-B', str(Path(__file__).resolve()), '--out', str(out), '--run-prepared'],
         'ownedExecutables': [str(executable), str(driver)], 'timeoutSeconds': 110, 'diagnosticContinuationAllowed': False,
         'driver': str(driver), 'driverSha256': sha(driver), 'bundleIdentifier': identifier, 'build': str(build),
         'compileCommand': command, 'driverCompileCommand': driver_command, 'pinnedInputHashes': pins, 'originalInputHashes': originals,
         'sourceProfileHashes': source_profile_hashes, 'profile': str(profile), 'initialProfileHashes': hashes(profile),
         'sourceMap': str(source_map), 'sourceMapSha256': sha(source_map), 'sourceDocument': document, 'savedMap': str(saved_map),
         'inputDeadlineSeconds': 90, 'hostWatchdogSeconds': 100, 'totalBoundSeconds': 110, 'expectedSeconds': '30–45',
         'steps': ['Open Map Cancel', 'Save Map As Cancel', 'Save Map As Unicode destination', 'Open original map', 'Open saved map', 'Cancel mapper and normal host shutdown'],
         'scope': 'Exact two unmodified Element Mapper/SchedulerAlerts source files compiled -O with current optimized Kit/Bridge/FFI. Real production presenter, local File menu and native Open/Save panels. Seeded initial private map is fixture setup. No UI_TEST, supplied panel result, filename hook, controller, original show write or physical drag claim.',
         'limitations': ['Prepared only: native File menu and file-panel AX navigation are unexecuted.', 'Not a whole VixenScheduler release object or full application startup test.', 'Cancel proves unchanged map file/editor return; no changed-map discard prompt or physical mapper drag is exercised.']}
    write(out/'prepared.json', m); print(out/'prepared.json')


def validate_outputs(out, m, expected_pid):
    f = json.loads((out/'foreground.json').read_text())
    host = json.loads((out/'events.json').read_text()); events = host['events']
    assert f['pid'] == host['pid'] == expected_pid
    assert f['passed'] and f['targetExited'] and f['noInputHeld'] and f['restoredPrevious']
    names = [e['event'] for e in events]
    assert 'watchdog' not in names and 'startup-error' not in names and names[-2:] == ['shutdown-complete', 'terminated']
    completed = [e for e in events if e['event'] == 'mapper-completed']
    assert len(completed) == 1 and completed[0]['cancelled'] is True and completed[0]['path'] is None
    ready = [e for e in events if e['event'] == 'ready']
    assert len(ready) == 1 and not ready[0]['webRunning'] and not ready[0]['testMode']
    ordered = ['initial', 'open-cancelled', 'save-cancelled', 'saved-live', 'original-reopened', 'saved-reopened']
    assert [row['stage'] for row in f['steps']] == ordered
    snapshots = {e['stage']: e for e in f['steps']}
    normalize = lambda p: unicodedata.normalize('NFC', p)
    for stage, row in snapshots.items():
        wanted = m['savedMap'] if stage in ['saved-live', 'saved-reopened'] else m['sourceMap']
        assert normalize(row['observedMapPath']) == normalize(wanted) and not row['observedModified']
    assert all(not snapshots[s]['savedExists'] for s in ['initial', 'open-cancelled', 'save-cancelled'])
    saved = Path(m['savedMap']); digest = sha(saved)
    assert all(snapshots[s]['savedSha256'] == digest for s in ['saved-live', 'original-reopened', 'saved-reopened'])
    actual = json.loads(saved.read_text()); expected = dict(m['sourceDocument']); expected['name'] = saved.stem
    actual['name'] = normalize(actual['name']); assert actual == expected
    states = [e for e in events if e['event'] == 'mapper-state']
    assert {normalize(e['path']) for e in states} == {normalize(m['sourceMap']), normalize(m['savedMap'])}
    for state in states:
        assert not state['modified'] and state['rows'] == [{k: v for k, v in entry.items() if k != 'isDirty'} for entry in m['sourceDocument']['elementMappings']]
    return dict(savedSha256=digest, savedDocument=actual, acceptedNativePanelWorkflow=True, foreground=f, hostEvents=events)


def run(out):
    start = time.monotonic(); epoch = time.time(); total_end = start+110
    m = json.loads((out/'prepared.json').read_text()); executable = Path(m['executable']); driver = Path(m['driver'])
    assert sha(Path(__file__).resolve()) == m['originalInputHashes'][str(Path(__file__).resolve())], 'Executing runner differs from reviewed preparation'
    assert sha(executable) == m['executableSha256'] and sha(driver) == m['driverSha256']
    assert all(sha(p) == h for p, h in m['pinnedInputHashes'].items()) and hashes(Path(m['profile'])) == m['initialProfileHashes']
    assert not any((out/name).exists() for name in ['events.json', 'result.json', 'foreground.json']) and not Path(m['savedMap']).exists()
    assert time.monotonic() < total_end, 'Budget expired during prepared-input validation'
    desktop = json.loads(subprocess.check_output([str(driver), '--desktop'], text=True, timeout=min(5,total_end-time.monotonic())))
    assert desktop['accessibility'] and desktop['eventPosting'] and not desktop['vixenProcesses']
    previous = desktop['application']; assert previous['pid'] > 0 and 'securityagent' not in previous['bundleIdentifier'].lower()
    prefs = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist', Path.home()/'Library/Application Support/Vixen/Settings.xml']
    before = {str(p): sha(p) for p in prefs}; write(out/'desktop-before.json', desktop)
    environment = {k: v for k, v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    environment.update(VIXEN_MUTE='1', VIXEN_SETTINGS_DIR=str(out/'private-settings'), TMPDIR=str(out/'runtime')+'/')
    process = helper = None; report = {'passed': False, 'scope': m['scope'], 'forcedCleanup': False,
                                     'finalizationErrors': []}
    def record_owned(owned):
        temporary = out/'.owned-processes.json.tmp'
        write(temporary, owned)
        os.replace(temporary, out/'owned-processes.json')
    try:
        # Setup/hashing is charged to the same absolute110s budget. Never launch
        # with less than a complete90s input allowance and14s cleanup reserve.
        assert total_end-time.monotonic() >= 104, 'Insufficient full input/cleanup budget; no app launch'
        input_end = time.monotonic()+90
        with (out/'process.log').open('w') as log, (out/'driver.log').open('w') as driver_log:
            process = subprocess.Popen([str(executable), str(out/'host-config.json')], env=environment, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            report['hostPID'] = process.pid
            owned = {'appPID': process.pid, 'appExecutable': str(executable), 'appProcessGroup': process.pid,
                     'bundleIdentifier': m['bundleIdentifier']}
            record_owned(owned)
            config = {**m, 'pid': process.pid, 'out': str(out), 'previousApplication': previous,
                      'inputDeadlineEpoch': epoch+(input_end-start), 'cleanupDeadlineEpoch': epoch+108}
            write(out/'run-config.json', config)
            assert time.monotonic() < input_end, 'Input deadline expired before driver launch'
            helper = subprocess.Popen([str(driver), str(out/'run-config.json')], stdout=driver_log, stderr=subprocess.STDOUT, start_new_session=True)
            report['driverPID'] = helper.pid
            owned.update(driverPID=helper.pid, driverExecutable=str(driver), driverProcessGroup=helper.pid)
            record_owned(owned)
            assert helper.wait(timeout=max(.001, min(input_end+4,total_end-10)-time.monotonic())) == 0, 'Native driver stopped; inspect foreground/failure AX report'
            assert process.wait(timeout=max(.001, min(5,total_end-8-time.monotonic()))) == 0
        report.update(passed=True, **validate_outputs(out, m, process.pid))
    except BaseException as error:
        report['error'] = repr(error)
        raise
    finally:
        for label, child in [('helper', helper), ('host', process)]:
            if child is not None and child.poll() is None:
                report['forcedCleanup'] = True
                try:
                    os.killpg(child.pid, signal.SIGTERM)
                    try: child.wait(timeout=max(.001,min(2,total_end-time.monotonic())))
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait(timeout=max(.001,min(1,total_end-time.monotonic())))
                except BaseException as error:
                    report['finalizationErrors'].append({'phase': label+'-cleanup', 'error': repr(error)})
        def final_hash(path):
            try: return sha(path)
            except OSError as error:
                report['finalizationErrors'].append({'phase': 'final-hash', 'path': str(path), 'error': repr(error)})
                return None
        try: profile_validation = validate_private_profile(Path(m['profile']), m['initialProfileHashes'])
        except OSError as error:
            profile_validation = {'accepted': False, 'strictlyUnchanged': False, 'reason': repr(error)}
            report['finalizationErrors'].append({'phase': 'private-profile-hashes', 'error': repr(error)})
        report.update(hostExited=process is None or process.poll() is not None,
                      helperExited=helper is None or helper.poll() is not None, hostExitCode=process.returncode if process else None,
                      helperExitCode=helper.returncode if helper else None, ownerPreferencesBefore=before,
                      ownerPreferencesAfter={str(p): final_hash(p) for p in prefs}, inputsUnchanged=all(final_hash(p) == h for p, h in m['pinnedInputHashes'].items()),
                      sourceProfileUnchanged=all(final_hash(p) == h for p, h in m['sourceProfileHashes'].items()),
                      privateProfileUnchanged=profile_validation['strictlyUnchanged'],
                      privateProfileValid=profile_validation['accepted'], privateProfileValidation=profile_validation,
                      appUnchanged=final_hash(executable) == m['executableSha256'], driverUnchanged=final_hash(driver) == m['driverSha256'])
        report['ownerPreferencesUnchanged'] = report['ownerPreferencesBefore'] == report['ownerPreferencesAfter']
        try:
            foreground = json.loads((out/'foreground.json').read_text()) if (out/'foreground.json').is_file() else {}
            assert isinstance(foreground, dict)
        except (OSError, ValueError, AssertionError) as error:
            foreground = {}; report.setdefault('error', repr(error)); report['passed'] = False
            report['finalizationErrors'].append({'phase': 'foreground-report', 'error': repr(error)})
        report['processExited'] = report['hostExited']
        report['cleanupCompleted'] = report['hostExited'] and report['helperExited'] and not report['forcedCleanup'] and not report['finalizationErrors'] and 'error' not in report
        report['noInputHeld'] = foreground.get('noInputHeld') is True
        report['restoredPrevious'] = foreground.get('restoredPrevious') is True
        report['elapsedSeconds'] = time.monotonic()-start
        report['withinTotalBudget'] = report['elapsedSeconds'] <= 110
        report['passed'] = report['passed'] and not report['forcedCleanup'] and all(report[k] for k in [
            'ownerPreferencesUnchanged', 'inputsUnchanged', 'sourceProfileUnchanged', 'privateProfileValid',
            'appUnchanged', 'driverUnchanged', 'processExited', 'cleanupCompleted', 'noInputHeld',
            'restoredPrevious', 'withinTotalBudget'])
        write(out/'result.json', report)
    assert report['passed'], report


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--build', type=Path, default=ROOT/'target/build-release')
    modes = parser.add_mutually_exclusive_group(required=True); modes.add_argument('--prepare-only', action='store_true'); modes.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args(); out = args.out.resolve(); assert out.is_relative_to(ROOT/'target')
    if args.prepare_only: prepare(out, args.build.resolve())
    else: run(out)


if __name__ == '__main__': main()
