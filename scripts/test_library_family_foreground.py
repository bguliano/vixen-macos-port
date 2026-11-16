#!/usr/bin/env python3
"""Prepare or run one actual Color/Gradient library pointer workflow.

--run-prepared requires root-coordinated desktop readiness. No library mutation
or drag callback hook is used. Setup hooks only open/size/select the private UI.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
COLORS = [0xffff0000, 0xff00ff00, 0xff0000ff, 0xffffff00]
STAGES = ['selected-two', 'moved-left', 'moved-end', 'toolbar-copy']
local = lambda tag: tag.rsplit('}', 1)[-1]
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


def tree(path):
    return {str(p.relative_to(path)): digest(p) for p in sorted(path.rglob('*')) if p.is_file()}


def palette(path):
    return [int(next(c.text for c in e if local(c.tag) == 'value')) & 0xffffffff for e in ET.parse(path).getroot()]


def order(out, kind):
    if kind == 'color':
        return ['Color '+chr(65+COLORS.index(c)) for c in palette(out/'ui/ColorPalette.xml')]
    root = ET.parse(out/'profile/SystemData/ModuleStore.xml').getroot()
    library = next(e for e in root.iter() if local(e.tag) == 'ColorGradientLibraryStaticData')
    entries = next(e for e in library if local(e.tag) == '_library')
    return [next(c.text for c in e if local(c.tag) == 'Key') for e in entries]


def preferences():
    paths = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist',
             Path.home()/'Library/Application Support/Vixen/Settings.xml']
    return {str(p): digest(p) if p.exists() else None for p in paths}


def read_foreground_result(out, result):
    """Keep the helper's diagnosis even when it deliberately stops the app early."""
    path = out/'foreground.json'
    try:
        fg = json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise AssertionError('Foreground helper report unavailable or invalid: '+str(error)) from error
    assert isinstance(fg, dict), 'Foreground helper report is not an object'
    result['foreground'] = fg
    assert fg.get('passed') is True, 'Foreground helper failed: '+str(fg.get('error', 'missing error detail'))
    return fg


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--kind', choices=['color', 'gradient'], required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--app', type=Path, default=ROOT/'target/private-app-native-color-well-release/Vixen.app')
    p.add_argument('--calibration', type=Path)
    p.add_argument('--helper', type=Path, default=ROOT/'target/foreground-library-family')
    mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-only', action='store_true')
    mode.add_argument('--run-prepared', action='store_true')
    args = p.parse_args()
    out, app, helper = args.out.resolve(), args.app.resolve(), args.helper.resolve()
    assert all(x.is_relative_to(ROOT/'target') for x in [out, app, helper])
    source = ROOT/'target/library-drag-fixture/profile'
    exe = app/'Contents/MacOS/Vixen'
    inspector = ROOT/'target/build-main/rust/debug/examples/library_drag_fixture'
    if args.prepare_only:
        assert not out.exists() and args.calibration
        calibration = args.calibration.resolve()
        cal = json.loads(calibration.read_text())
        assert cal['kind'] == args.kind and cal['reviewedActualLayout']
        assert digest(cal['referenceImage']) == cal['referenceImageSha256']
        clone = json.loads((app.parent/'private-app.json').read_text())
        info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
        assert clone['passed'] and digest(exe) == clone['executableSha256']
        assert info['CFBundleIdentifier'] == clone['bundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
        assert not any(x.is_symlink() for x in source.rglob('*'))
        system = ET.parse(source/'SystemData/SystemConfig.xml').getroot()
        assert not list(next(e for e in system if local(e.tag) == 'Controllers'))
        assert palette(source/'ColorPalette.xml') == COLORS
        out.mkdir()
        shutil.copytree(source, out/'profile')
        (out/'profile/.lock').unlink(missing_ok=True)
        shutil.copy2(calibration, out/'calibration.json')
        hook = lambda name, *args: {'action': 'hook', 'name': name, 'path': list(args)}
        wait = lambda n: {'action': 'wait', 'seconds': n}
        steps = [{'action': 'window', 'id': 'administration'}, wait(1),
            hook('admin.openSequence', str(out/'profile/Sequence/Library Drag.tim')), wait(2),
            {'action': 'focus', 'window': 'Library Drag'}, hook('sequencer.resize', '1100', '780'),
            hook('sequencer.panel', args.kind.title()+' Library'), hook('sequencer.transport', 'state'), wait(.6),
            {'action': 'menu', 'path': ['View', 'Toolbars', 'Library '+args.kind.title()]}, wait(.6),
            {'action': 'dump', 'name': 'ready-controls'}, {'action': 'menus', 'name': 'ready-menus'},
            {'action': 'snapshot', 'name': 'ready'}, wait(5)]
        for stage in STAGES:
            steps += [{'action': 'dump', 'name': stage}, {'action': 'snapshot', 'name': stage}, wait(1 if stage == STAGES[-1] else 5)]
        steps += [hook('sequencer.state'), hook('sequencer.transport', 'state'), {'action': 'quit'}]
        (out/'steps.json').write_text(json.dumps({'steps': steps}, indent=2)+'\n')
        snapshot = out/'fixture-source'; snapshot.mkdir()
        for path in [Path(__file__).resolve(), ROOT/'scripts/foreground_library_family.swift']:
            shutil.copy2(path, snapshot/path.name)
        manifest = dict(kind=args.kind, app=str(app), appSha256=digest(exe), privateBundleIdentifier=info['CFBundleIdentifier'],
            sourceExecutableSha256=clone['sourceExecutableSha256'], source=str(source), sourceHashes=tree(source),
            preparedProfileHashes=tree(out/'profile'), helper=str(helper), helperSha256=digest(helper),
            helperSourceSha256=digest(ROOT/'scripts/foreground_library_family.swift'), runnerSha256=digest(__file__),
            calibrationSha256=digest(out/'calibration.json'), stepsSha256=digest(out/'steps.json'), steps=len(steps)-1,
            inspector=str(inspector), inspectorSha256=digest(inspector), referenceImageSha256=cal['referenceImageSha256'],
            inputDeadlineSeconds=40, appWaitSeconds=45, driverWaitSeconds=5, ownedCleanupBoundSeconds=14,
            inspectionTimeoutSeconds=3, conservativePerKindSeconds=67, expectedNormalSeconds=30,
            scope='Actual OS multi-selection reorder, empty-list move and toolbar-to-panel copy. Gradient name uses actual focused CG keys and AX acceptance. Color inserts directly. No mutation/drag hook; no reverse panel-to-toolbar or exact10-point boundary claim.')
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
        print(json.dumps(dict(prepared=str(out/'prepared.json'), steps=manifest['steps'], launched=False)))
        return
    m = json.loads((out/'prepared.json').read_text())
    assert m['kind'] == args.kind and m['app'] == str(app) and m['helper'] == str(helper)
    assert digest(exe) == m['appSha256'] and digest(helper) == m['helperSha256']
    assert digest(__file__) == m['runnerSha256'] and digest(ROOT/'scripts/foreground_library_family.swift') == m['helperSourceSha256']
    assert digest(out/'calibration.json') == m['calibrationSha256'] and digest(out/'steps.json') == m['stepsSha256']
    assert tree(out/'profile') == m['preparedProfileHashes'] and not (out/'ui').exists()
    processes = subprocess.check_output(['/bin/ps', '-A', '-o', 'pid=,comm='], text=True, timeout=3)
    assert not any(x.strip().endswith('/Contents/MacOS/Vixen') for x in processes.splitlines())
    before = preferences()
    (out/'ui').mkdir(); (out/'runtime').mkdir()
    shutil.copy2(out/'profile/ColorPalette.xml', out/'ui/ColorPalette.xml')
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_DATA=str(out/'profile'), VIXEN_MUTE='1', VIXEN_UI_TEST=str(out/'steps.json'),
        VIXEN_UI_TEST_OUT=str(out/'ui'), VIXEN_SETTINGS_DIR=str(out/'ui'), VIXEN_UI_TEST_TRACE='1',
        VIXEN_LIBRARY_DRAG_TRACE='1', TMPDIR=str(out/'runtime')+'/')
    started = time.monotonic(); process = driver = None
    result = dict(passed=False, kind=args.kind, appSha256=m['appSha256'], sourceExecutableSha256=m['sourceExecutableSha256'],
                  scope=m['scope'], checkpoints={}, ownerPreferenceHashesBefore=before)
    try:
        with (out/'process.log').open('w') as log, (out/'foreground.log').open('w') as driver_log:
            process = subprocess.Popen([str(exe)], env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            owned = {'appPID':process.pid, 'appExecutable':str(exe), 'appProcessGroup':process.pid}
            (out/'owned-processes.json').write_text(json.dumps(owned)+'\n')
            driver = subprocess.Popen([str(helper), str(process.pid), str(out), str(exe), str(out/'calibration.json')], stdout=driver_log, stderr=subprocess.STDOUT)
            owned.update(driverPID=driver.pid, driverExecutable=str(helper))
            (out/'owned-processes.json').write_text(json.dumps(owned)+'\n')
            deadline = time.monotonic()+45
            while process.poll() is None and time.monotonic() < deadline:
                for stage in ['ready', *STAGES]:
                    if stage not in result['checkpoints'] and (out/'ui'/(stage+'.png')).exists():
                        result['checkpoints'][stage] = dict(order=order(out, args.kind), whileAlive=True)
                        (out/'live-checkpoints.json').write_text(json.dumps(result['checkpoints'], indent=2)+'\n')
                time.sleep(.08)
            assert process.poll() is not None, 'Private app exceeded45 seconds'
            result['exitCode'] = process.returncode
            driver.wait(timeout=5)
            result['driverExitCode'] = driver.returncode
        fg = read_foreground_result(out, result)
        assert fg['targetExited'] and fg['restoredPrevious'] and len(fg['initialPixelSamples']) == 8
        rows = json.loads((out/'ui/result.json').read_text())
        assert process.returncode == driver.returncode == 0 and len(rows) == m['steps'] and all(r['ok'] for r in rows)
        assert not fg['heldButtonAfterCleanup'] and not fg['heldKeyAfterCleanup']
        assert fg['toolbarNamingPromptObserved'] == (args.kind == 'gradient')
        prefix = args.kind.title(); initial = [prefix+' '+letter for letter in 'ABCD']
        expected = {'ready':initial, 'selected-two':initial, 'moved-left':[initial[i] for i in [2,3,0,1]],
                    'moved-end':initial, 'toolbar-copy':[initial[0], 'Imported Gradient' if args.kind == 'gradient' else initial[0], *initial[1:]]}
        assert {k:v['order'] for k,v in result['checkpoints'].items()} == expected
        assert all(v['whileAlive'] for v in result['checkpoints'].values()) and order(out, args.kind) == expected['toolbar-copy']
        transport = [r['detail'] for r in rows if r.get('detail', '').startswith('speed=')]
        assert len(transport) == 2 and all('running=false' in x for x in transport)
        trace = (out/'process.log').read_text()
        assert trace.count('library-drag session-start kind='+args.kind+' ') == 3
        assert trace.count('library-drag session-ended kind='+args.kind+' ') == 3
        if args.kind == 'gradient':
            assert digest(inspector) == m['inspectorSha256']
            saved = json.loads(subprocess.check_output([str(inspector), str(out/'profile'), '--inspect'], timeout=3))
            values = saved['gradients']; result['savedInspection'] = saved
            assert [v['argb'] for v in values] == [COLORS[0], COLORS[0], *COLORS[1:]]
            assert all(v['current'] and not v['link'] for v in values)
        else:
            result['savedPalette'] = palette(out/'ui/ColorPalette.xml')
            assert result['savedPalette'] == [COLORS[0], COLORS[0], *COLORS[1:]]
        lock = out/'profile/.lock'
        if lock.exists():
            assert lock.read_text().startswith(str(process.pid)+'@')
            lock.unlink()
        result.update(passed=True, steps=len(rows), actualDragSessions=3, transportStopped=True,
                      savedStateVerified=True, privateLockRemovedAfterOwnedExit=True)
    except BaseException as error:
        result['error'] = repr(error)
        raise
    finally:
        if process and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try: process.wait(timeout=3)
            except subprocess.TimeoutExpired: os.killpg(process.pid, signal.SIGKILL); process.wait(timeout=3)
        if driver and driver.poll() is None:
            driver.terminate()
            try: driver.wait(timeout=5)
            except subprocess.TimeoutExpired: driver.kill(); driver.wait(timeout=3)
        result.update(elapsedSeconds=time.monotonic()-started, sourceUnchanged=tree(source)==m['sourceHashes'],
            appUnchanged=digest(exe)==m['appSha256'], ownerPreferencesUnchanged=preferences()==before)
        if not all(result[k] for k in ['sourceUnchanged','appUnchanged','ownerPreferencesUnchanged']): result['passed'] = False
        (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    assert result['passed'], result
    print(json.dumps(dict(passed=True, kind=args.kind, steps=result['steps'], elapsedSeconds=result['elapsedSeconds'])))


if __name__ == '__main__':
    main()
