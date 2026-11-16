#!/usr/bin/env python3
"""Private library drag preparation/acceptance; foreground mode requires exclusive desktop readiness.

The input helper sends actual WindowServer CG events. No drag/drop callback or
library mutation hook is used. Layout-only is an offscreen observation run.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import signal
import subprocess
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
local = lambda tag: tag.split('}')[-1]


def curve_order(profile):
    root = ET.parse(profile/'SystemData/ModuleStore.xml').getroot()
    library = next(n for n in root.iter() if local(n.tag) == 'CurveLibraryStaticData')
    entries = next(n for n in library if local(n.tag) == '_library')
    return [next(n.text for n in entry if local(n.tag) == 'Key') for entry in entries]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--app', type=Path, default=ROOT/'target/private-app-library-drag/Vixen.app')
    p.add_argument('--profile', type=Path, default=ROOT/'target/library-drag-fixture/profile')
    p.add_argument('--helper', type=Path, default=ROOT/'target/foreground-library-drag')
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--prepare-only', action='store_true')
    p.add_argument('--run-prepared', action='store_true')
    p.add_argument('--check-only', action='store_true')
    p.add_argument('--layout-only', action='store_true')
    p.add_argument('--edge-cases', action='store_true')
    p.add_argument('--panel-to-toolbar', action='store_true')
    p.add_argument('--stage-wait', type=int, choices=[5, 7], default=7)
    args = p.parse_args()
    assert not (args.layout_only and args.edge_cases)
    assert not (args.panel_to_toolbar and (args.layout_only or args.edge_cases))
    stages = (['reentry-five', 'boundary-ten', 'boundary-eleven'] if args.edge_cases else
              ['selected-two', 'moved-left', 'moved-end', 'toolbar-copy'])
    app, source, out = args.app.resolve(), args.profile.resolve(), args.out.resolve()
    executable = app/'Contents/MacOS/Vixen'
    info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
    clone = json.loads((app.parent/'private-app.json').read_text())
    assert info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    assert clone['passed'] and clone['bundleIdentifier'] == info['CFBundleIdentifier']
    assert digest(executable) == clone['executableSha256']
    assert source.is_relative_to(ROOT/'target') and out.is_relative_to(ROOT/'target')
    assert not any(path.is_symlink() for path in source.rglob('*'))
    helper = args.helper.resolve()
    if args.run_prepared or args.check_only:
        manifest = json.loads((out/'prepared.json').read_text())
        assert manifest['appSha256'] == digest(executable)
        assert manifest['layoutOnly'] == args.layout_only
        assert manifest.get('edgeCases', False) == args.edge_cases
        assert manifest.get('panelToToolbar', False) == args.panel_to_toolbar
        if not args.layout_only:
            assert manifest['helperSha256'] == digest(helper)
    else:
        assert not out.exists()
        out.mkdir(parents=True)
        hashes = {str(path.relative_to(source)): digest(path) for path in source.rglob('*') if path.is_file()}
        shutil.copytree(source, out/'profile')
        (out/'profile/.lock').unlink(missing_ok=True)
        h = lambda name, *path: {'action':'hook', 'name':name, 'path':list(path)}
        wait = lambda seconds=.4: {'action':'wait', 'seconds':seconds}
        steps = [{'action':'window','id':'administration'}, wait(1),
                 h('admin.openSequence', str(out/'profile/Sequence/Library Drag.tim')), wait(2),
                 {'action':'focus','window':'Library Drag'}, h('sequencer.resize','1100','780'),
                 h('sequencer.panel','Curve Library'), h('sequencer.transport','state'), wait(.6),
                 {'action':'menu','path':['View','Toolbars','Library Curve']}, wait(.6),
                 {'action':'dump','name':'ready-controls'}, {'action':'menus','name':'ready-menus'},
                 {'action':'snapshot','name':'ready'}]
        if not args.layout_only:
            steps += [wait(args.stage_wait)]
            for name in stages:
                steps += [{'action':'dump','name':name}, {'action':'snapshot','name':name}, wait(args.stage_wait if name != stages[-1] else 1)]
            steps += [h('sequencer.state'), h('sequencer.transport','state')]
        steps += [{'action':'quit'}]
        (out/'steps.json').write_text(json.dumps({'steps':steps},indent=2)+'\n')
        manifest = {'appSha256':digest(executable),'privateBundleIdentifier':info['CFBundleIdentifier'],
                    'sourceExecutableSha256':clone['sourceExecutableSha256'],'source':str(source),'sourceHashes':hashes,
                    'layoutOnly':args.layout_only,'steps':len(steps)-1,'stageWaitSeconds':args.stage_wait,
                    'edgeCases':args.edge_cases,
                    'panelToToolbar':args.panel_to_toolbar,
                    'initialOrder':['Curve A','Curve B','Curve C','Curve D'],
                    'scope':'Private profile and Cocoa defaults; actual CG drag routing, no mutation hooks; 10-point boundary not yet claimed'}
        if args.edge_cases:
            manifest['scope'] = 'Actual CG list exit/re-entry followed by +5/+10/+11 point drops; native callback coordinates and live persistence must agree; no drag callback or mutation hooks.'
        if not args.layout_only:
            manifest['helperSha256'] = digest(helper)
        (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
    if args.prepare_only:
        print(out/'prepared.json')
        return
    profile, ui = out/'profile', out/'ui'
    report = {'passed':False,'layoutOnly':args.layout_only,'appSha256':digest(executable),
              'scope':manifest['scope'],'checkpoints':{}}
    process = driver = None
    if args.check_only:
        report = json.loads((out/'result.json').read_text())
    else:
        assert not ui.exists()
        ui.mkdir(); (out/'runtime').mkdir()
        shutil.copyfile(profile/'ColorPalette.xml', ui/'ColorPalette.xml')
        env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
        env.update(VIXEN_DATA=str(profile),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),
                   VIXEN_UI_TEST_OUT=str(ui),VIXEN_UI_TEST_TRACE='1',VIXEN_LIBRARY_DRAG_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        try:
            with (out/'process.log').open('w') as log, (out/'foreground.log').open('w') as driver_log:
                process = subprocess.Popen([str(executable)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                if not args.layout_only:
                    assert helper.is_file()
                    command = [str(helper),str(process.pid),str(out),str(executable)] + (['edges'] if args.edge_cases else [])
                    if args.panel_to_toolbar:
                        command.append('panel-to-toolbar')
                    driver = subprocess.Popen(command,stdout=driver_log,stderr=subprocess.STDOUT)
                deadline = time.monotonic()+75
                while process.poll() is None and time.monotonic()<deadline:
                    for name in ['ready', *stages]:
                        if name not in report['checkpoints'] and (ui/(name+'.png')).exists():
                            report['checkpoints'][name] = {'order':curve_order(profile),'whileAlive':True,
                                'moduleStoreSha256':digest(profile/'SystemData/ModuleStore.xml')}
                            (out/'live-checkpoints.json').write_text(json.dumps(report['checkpoints'],indent=2)+'\n')
                    time.sleep(.1)
                assert process.poll() is not None, 'private app deadline'
                report['exitCode'] = process.returncode
                if driver: driver.wait(timeout=8)
        except BaseException as error:
            report['error'] = repr(error)
            raise
        finally:
            if process and process.poll() is None:
                os.killpg(process.pid,signal.SIGTERM)
                try: process.wait(timeout=3)
                except subprocess.TimeoutExpired: os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
            if driver and driver.poll() is None:
                driver.terminate()
                try: driver.wait(timeout=5)
                except subprocess.TimeoutExpired: driver.kill();driver.wait(timeout=3)
            (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    try:
        if not (ui/'result.json').exists():
            foreground_path=out/'foreground.json'
            if foreground_path.exists():
                report['foreground']=json.loads(foreground_path.read_text())
            raise AssertionError('Native harness did not finish: '+str(report.get('foreground',{}).get('error',report.get('error','inspect process.log'))))
        rows = json.loads((ui/'result.json').read_text())
        report['steps'] = len(rows)
        assert report['exitCode'] == 0 and len(rows)==manifest['steps'] and all(row['ok'] for row in rows)
        transport = [row['detail'] for row in rows if row.get('detail','').startswith('speed=')]
        assert len(transport) == (1 if args.layout_only else 2) and all('running=false' in item for item in transport)
        report['transportStopped'] = True
        assert {str(path.relative_to(source)):digest(path) for path in source.rglob('*') if path.is_file()}==manifest['sourceHashes']
        assert digest(executable)==manifest['appSha256']
        report['sourceUnchanged'] = True
        if args.layout_only:
            report.update(recorded=True, passed=True, acceptance=False)
        else:
            report['foreground'] = json.loads((out/'foreground.json').read_text())
            assert report['foreground']['passed'] and report['foreground']['targetExited'] and report['foreground']['restoredPrevious']
            c = report['checkpoints']
            report['positiveReorderRoutingObserved'] = any(c[name]['order'] != manifest['initialOrder'] for name in stages)
            report['savedInspection'] = json.loads(subprocess.check_output([
                str(ROOT/'target/build-main/rust/debug/examples/library_drag_fixture'), str(profile), '--inspect']))
            assert report['positiveReorderRoutingObserved'], 'No positive system drag routing was observed'
            if args.edge_cases:
                assert c['reentry-five']['order'] == manifest['initialOrder'], c['reentry-five']
                assert c['boundary-ten']['order'] == manifest['initialOrder'], c['boundary-ten']
                assert c['boundary-eleven']['order'] == ['Curve B','Curve C','Curve D','Curve A'], c['boundary-eleven']
                point = re.compile(r'location=\(([-\d.]+), ([-\d.]+)\)')
                native = []
                first_cell = entry = None
                for line in (out/'process.log').read_text().splitlines():
                    if 'library-drag session-start ' in line:
                        first_cell = entry = None
                    if 'library-drag cell-enter kind=curve' in line and first_cell is None:
                        first_cell = tuple(map(float, point.search(line).groups()))
                    if 'library-drag empty-enter kind=curve' in line:
                        entry = tuple(map(float, point.search(line).groups()))
                    if 'library-drag empty-drop kind=curve' in line:
                        drop = tuple(map(float, point.search(line).groups()))
                        assert entry is not None and first_cell is not None, line
                        native.append({'firstCellEntry':first_cell, 'reentry':entry, 'drop':drop,
                                       'farEnough':'farEnough=true' in line, 'own':'own=true' in line})
                assert len(native) == 3, native
                for state, delta in zip(native, [5, 10, 11]):
                    assert abs(state['drop'][0]-state['reentry'][0]-delta) < .05, state
                    assert abs(state['drop'][1]-state['reentry'][1]) < .05, state
                    assert abs(state['firstCellEntry'][1]-state['reentry'][1]) > 10, state
                    assert state['own'] and state['farEnough'] == (delta == 11), state
                report['nativeEdgeEvidence'] = native
                values = report['savedInspection']['curves']
                assert [value['at50'] for value in values] == [20.0,30.0,40.0,10.0]
                assert all(value['current'] and not value['link'] for value in values)
                assert all(value['whileAlive'] for value in c.values())
                report.update(passed=True, acceptance=True)
                return
            assert c['selected-two']['order']==manifest['initialOrder']
            assert c['moved-left']['order']==['Curve C','Curve D','Curve A','Curve B'], c['moved-left']
            assert c['moved-end']['order']==manifest['initialOrder'], c['moved-end']
            expected_order = (['Curve A','Curve B','Curve C','Curve D','Imported Curve'] if args.panel_to_toolbar else
                              ['Curve A','Imported Curve','Curve B','Curve C','Curve D'])
            assert c['toolbar-copy']['order']==expected_order, c['toolbar-copy']
            assert report['foreground']['toolbarNamingPromptObserved']
            assert report['foreground'].get('panelToToolbar', False) == args.panel_to_toolbar
            values = report['savedInspection']['curves']
            assert [value['at50'] for value in values] == ([10.0,20.0,30.0,40.0,10.0] if args.panel_to_toolbar else [10.0,10.0,20.0,30.0,40.0])
            assert all(value['current'] and not value['link'] for value in values)
            assert all(value['whileAlive'] for value in c.values())
            report.update(passed=True, acceptance=True)
    except BaseException as error:
        report['error'] = repr(error)
        raise
    finally:
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':report['passed'],'steps':report['steps'],'result':str(out/'result.json')}))


if __name__=='__main__': main()
