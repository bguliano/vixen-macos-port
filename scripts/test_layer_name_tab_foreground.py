#!/usr/bin/env python3
"""Focused layer-name draft survival across inactive/active drop registration.

Preparation never launches the app. Runtime needs a coordinated foreground slot.
The external driver finds the actual AX name field and types physical events;
the harness only opens/selects panels, observes, saves, and reopens private data.
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

from test_layer_drag_foreground import layer_xml, hashes, hook, wait

ROOT = Path(__file__).resolve().parents[1]
DRAFT = 'Layer Draft Retained'
INITIAL = ['Zebra Layer', 'Alpha Layer', 'Middle Layer', 'Default']
FINAL = [DRAFT, *INITIAL[1:]]
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


def preferences():
    paths = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist',
             Path.home()/'Library/Application Support/Vixen/Settings.xml']
    return {str(p): digest(p) if p.exists() else None for p in paths}


def main():
    started=time.monotonic(); deadline=started+55
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--helper', type=Path, default=ROOT/'target/foreground-layer-name-tab-v2')
    parser.add_argument('--profile', type=Path, default=ROOT/'target/layer-drag-source')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-only', action='store_true')
    mode.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args()
    app, out, helper, source = (p.resolve() for p in [args.app, args.out, args.helper, args.profile])
    assert all(p.is_relative_to(ROOT/'target') for p in [app, out, helper, source])
    executable = app/'Contents/MacOS/Vixen'
    clone_path = app.parent/'private-app.json'
    clone = json.loads(clone_path.read_text())
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged']
    assert clone['app'] == str(app) and clone['executableSha256'] == digest(executable)
    info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
    assert info['CFBundleIdentifier'] == clone['bundleIdentifier']
    assert info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    if args.prepare_only:
        assert not out.exists()
        fixture = json.loads((source/'fixture.json').read_text())
        assert fixture['controllers'] == 0 and fixture['muted']
        original = Path(fixture['sequence']).resolve()
        assert original.is_relative_to(source) and [v['name'] for v in fixture['layers']] == INITIAL
        out.mkdir()
        shutil.copytree(source, out/'profile')
        (out/'profile/.lock').unlink(missing_ok=True)
        sequence = out/'profile'/original.relative_to(source)
        steps = [dict(action='window', id='administration'), wait(.7),
                 hook('admin.openSequence', str(sequence)), wait(1.5),
                 dict(action='focus', window='Layer Drag Verification'),
                 hook('sequencer.activate', 'Layer Drag Verification'),
                 hook('sequencer.resize', '1280', '800'), hook('sequencer.panel', 'Layers'),
                 hook('sequencer.layers', 'select', '0'), hook('sequencer.layers', 'expand', '0'), wait(.5),
                 dict(action='snapshot', name='name-ready'), wait(5),
                 hook('sequencer.layers', 'state'), dict(action='snapshot', name='name-draft'),
                 hook('sequencer.keyboardState'), hook('sequencer.panel', 'Color Library'), wait(.5),
                 dict(action='snapshot', name='name-away'), hook('sequencer.layers', 'state'),
                 hook('sequencer.panel', 'Layers'), wait(.5),
                 dict(action='snapshot', name='name-returned'), wait(3),
                 hook('sequencer.layers', 'state'), hook('sequencer.keyboardState'),
                 dict(action='snapshot', name='name-committed'),
                 dict(action='menu', path=['Sequence', 'Save']), wait(.5),
                 dict(action='closeWindow'), wait(.5), hook('admin.openSequence', str(sequence)), wait(1),
                 dict(action='focus', window='Layer Drag Verification'),
                 hook('sequencer.activate', 'Layer Drag Verification'), hook('sequencer.panel', 'Layers'), wait(.4),
                 hook('sequencer.layers', 'state'), hook('sequencer.keyboardState'),
                 dict(action='snapshot', name='name-reopened'), dict(action='quit')]
        (out/'steps.json').write_text(json.dumps({'steps':steps}, indent=2)+'\n')
        pins = {str(p):digest(p) for p in [Path(__file__).resolve(), ROOT/'scripts/test_layer_drag_foreground.py',
                ROOT/'scripts/foreground_layer_name_tab.swift', helper, executable, clone_path, out/'steps.json']}
        manifest = {'app':str(app), 'executable':str(executable), 'helper':str(helper),
                    'source':str(source), 'sourceHashes':hashes(source), 'profileHashes':hashes(out/'profile'),
                    'sequence':str(sequence), 'originalXML':layer_xml(original), 'pins':pins,
                    'steps':len(steps)-1, 'inputDeadlineSeconds':30, 'appWaitSeconds':35,
                    'driverWaitSeconds':5, 'cleanupBoundSeconds':10, 'timeoutSeconds':55,
                    'scope':'Real AX-discovered field click and keyboard draft; tab away/back via existing setup hooks; Return without retyping; private save/reopen and full UUID/filter/level preservation.'}
        (out/'prepared.json').write_text(json.dumps(manifest, indent=2)+'\n')
        print(json.dumps({'prepared':str(out/'prepared.json'), 'steps':manifest['steps'], 'launched':False}))
        return
    m = json.loads((out/'prepared.json').read_text())
    assert not (out/'result.json').exists(), 'Refuse to overwrite attempted-run evidence'
    before=preferences(); process=driver=None
    result={'passed':False, 'ownerPreferenceHashesBefore':before, 'scope':m['scope'],
            'processExited':False,'driverExited':False,'cleanupCompleted':False,
            'noInputHeld':False,'restoredPrevious':False,'finalizationErrors':[]}

    environment={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    environment.update(VIXEN_DATA=str(out/'profile'), VIXEN_MUTE='1', VIXEN_UI_TEST=str(out/'steps.json'),
        VIXEN_UI_TEST_OUT=str(out/'ui'), VIXEN_SETTINGS_DIR=str(out/'ui'), VIXEN_UI_TEST_TRACE='1', TMPDIR=str(out/'runtime')+'/')
    try:
        assert m['app']==str(app) and m['helper']==str(helper) and m['source']==str(source)
        assert all(digest(p)==h for p,h in m['pins'].items()), 'Frozen input changed'
        assert hashes(source)==m['sourceHashes'], 'Source fixture changed'
        assert hashes(out/'profile')==m['profileHashes'] and not (out/'ui').exists()
        (out/'ui').mkdir();(out/'runtime').mkdir()
        with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as helper_log:
            assert deadline-time.monotonic() >= 45, 'Insufficient complete native-run interval; no app launched'
            process=subprocess.Popen([str(executable)],env=environment,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned={'appPID':process.pid,'appExecutable':str(executable),'appProcessGroup':process.pid}
            (out/'owned-processes.json').write_text(json.dumps(owned)+'\n')
            assert deadline-time.monotonic() >= 40, 'Insufficient input/cleanup interval; no driver launched'
            driver=subprocess.Popen([str(helper),str(process.pid),str(out),str(executable)],stdout=helper_log,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(driverPID=driver.pid,driverExecutable=str(helper),driverProcessGroup=driver.pid);(out/'owned-processes.json').write_text(json.dumps(owned)+'\n')
            process.wait(timeout=min(35, max(.001, deadline-time.monotonic()-10)))
            driver.wait(timeout=min(5,max(.001,deadline-time.monotonic()-8)))
        result.update(exitCode=process.returncode,driverExitCode=driver.returncode)
        foreground=json.loads((out/'foreground.json').read_text());result['foreground']=foreground
        assert process.returncode==driver.returncode==0 and foreground['passed'],foreground
        assert foreground['targetExited'] and foreground['restoredPrevious'] and foreground['noInputHeld'],foreground
        rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
        states=[r['detail'] for r in rows if r.get('detail','').startswith(('Zebra Layer:','Layer Draft Retained:'))]
        names=lambda text:[v.split(':')[0] for v in text.split(' | ')]
        assert len(states)==6,states # select, expand, draft, away, committed, reopened
        assert [names(v) for v in states[:3]]==[INITIAL]*3,states
        assert names(states[3]) in [INITIAL,FINAL],states
        assert [names(v) for v in states[4:]]==[FINAL,FINAL],states
        transport=[r['detail'] for r in rows if r.get('detail','').startswith('running=')]
        assert len(transport)==3 and all('running=false paused=false' in s for s in transport),transport
        saved=layer_xml(Path(m['sequence'])); expected=json.loads(json.dumps(m['originalXML']))
        expected['layers'][0]['LayerName']=DRAFT
        assert saved==expected,(saved,expected)
        assert (out/'driver-typed.json').exists() and (out/'driver-committed.json').exists()
        result.update(passed=True,steps=len(rows),layerStates=states,transportStates=transport,savedXML=saved,
                      savedStateVerified=True)
    except BaseException as error:
        result['error']=repr(error)
        raise
    finally:
        for label, child in [('app',process),('driver',driver)]:
            try:
                if child is not None and child.poll() is None:
                    try: os.killpg(child.pid,signal.SIGTERM)
                    except ProcessLookupError: pass
                    try: child.wait(timeout=min(2,max(.001,deadline-time.monotonic()-2)))
                    except subprocess.TimeoutExpired:
                        try: os.killpg(child.pid,signal.SIGKILL)
                        except ProcessLookupError: pass
                        child.wait(timeout=min(1,max(.001,deadline-time.monotonic())))
            except BaseException as error:
                result['finalizationErrors'].append(label+': '+repr(error))
        def observed(label, operation):
            try: return operation()
            except BaseException as error:
                result['finalizationErrors'].append(label+': '+repr(error));return False
        foreground={}
        if (out/'foreground.json').is_file():
            try: foreground=json.loads((out/'foreground.json').read_text())
            except BaseException as error: result['finalizationErrors'].append('foreground: '+repr(error))
        result['foreground']=foreground
        result.update(elapsedSeconds=time.monotonic()-started,
            processExited=process is not None and process.poll() is not None,
            driverExited=driver is not None and driver.poll() is not None,
            sourceUnchanged=observed('source',lambda:hashes(source)==m['sourceHashes']),
            ownerPreferencesUnchanged=observed('preferences',lambda:preferences()==before),
            inputsUnchanged=observed('pins',lambda:all(digest(p)==h for p,h in m['pins'].items())),
            appUnchanged=observed('app hash',lambda:digest(executable)==m['pins'][str(executable)]),
            driverUnchanged=observed('driver hash',lambda:digest(helper)==m['pins'][str(helper)]),
            noInputHeld=foreground.get('noInputHeld') is True,
            restoredPrevious=foreground.get('restoredPrevious') is True)
        result['cleanupCompleted']=result['processExited'] and result['driverExited'] and not result['finalizationErrors']
        result['withinDeadline']=time.monotonic()<=deadline
        required=['processExited','driverExited','sourceUnchanged','ownerPreferencesUnchanged','inputsUnchanged',
                  'appUnchanged','driverUnchanged','noInputHeld','restoredPrevious','cleanupCompleted','withinDeadline']
        if not all(result[k] for k in required):result['passed']=False
        (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':result['passed'],'steps':result.get('steps',0),'elapsedSeconds':result['elapsedSeconds']}))


if __name__=='__main__':main()
