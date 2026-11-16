#!/usr/bin/env python3
"""Prepare or observe one private editor→library layout; never posts global input.

--run-layout requires root coordination of the native app slot. Actual physical
driver preparation follows retained current geometry; this is not acceptance.
"""
import argparse, hashlib, json, os, plistlib, shutil, signal, subprocess, time
from pathlib import Path
import xml.etree.ElementTree as ET
from test_library_cross_window_foreground import library_state, tree, sha, save, terminate
from test_library_family_foreground import preferences

ROOT=Path(__file__).resolve().parents[1]
def front():
    value=subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=2).strip()
    assert value.startswith('ASN:')
    return value
def prepare(out,app,source,inspector):
    assert not out.exists() and all(p.is_relative_to(ROOT/'target') for p in [out,app,source,inspector])
    assert not any(p.is_symlink() for p in source.rglob('*'))
    clone=json.loads((app.parent/'private-app.json').read_text());exe=app/'Contents/MacOS/Vixen'
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes())
    assert clone['passed'] and clone['executableSha256']==sha(exe)
    assert info['CFBundleIdentifier']==clone['bundleIdentifier'] and clone['bundleIdentifier'].startswith('com.vixenlights.verification.')
    fixture=json.loads(subprocess.check_output([str(inspector),str(source),'--inspect'],timeout=3))
    assert fixture['controllers']==0 and fixture['effects']==1 and fixture['sourcePoints']==[[0.,13.],[25.,77.],[100.,37.]]
    assert fixture['sourceLink']=='' and fixture['sourceCurrent'] is False
    out.mkdir();shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    hook=lambda name,*args:dict(action='hook',name=name,path=list(args))
    wait=lambda seconds:dict(action='wait',seconds=seconds)
    steps=[dict(action='window',id='administration'),wait(1),hook('admin.openSequence',str(out/'profile/Sequence/Editor Library.tim')),
        wait(2),hook('sequencer.activate','Editor Library'),dict(action='focus',window='Editor Library'),
        hook('sequencer.resize','1100','780'),wait(.6),dict(action='menu',path=['Edit','Select All Elements']),
        hook('sequencer.panel','Effect Editor'),wait(.6),hook('sequencer.transport','state'),
        dict(action='menus',name='before-toolbar'),dict(action='menu',path=['View','Toolbars','Library Curve']),wait(.6),
        hook('sequencer.focusState'),dict(action='dump',name='ready-controls'),dict(action='menus',name='ready-menus'),
        dict(action='snapshot',name='ready'),hook('timelineState'),hook('sequencer.state'),hook('timelineAllPlayback'),dict(action='quit')]
    save(out/'layout-steps.json',dict(steps=steps))
    pins=[Path(__file__).resolve(),ROOT/'scripts/test_library_cross_window_foreground.py',ROOT/'scripts/test_library_family_foreground.py',
        ROOT/'crates/vixen-session/examples/editor_library_fixture.rs',inspector,exe,app/'Contents/Info.plist',app.parent/'private-app.json',out/'layout-steps.json',
        ROOT/'app/Sources/VixenKit/CurveStripView.swift',ROOT/'app/Sources/VixenKit/KitSupport.swift',
        ROOT/'app/Sources/VixenEffectEditor/RowEditors.swift',ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift']
    manifest=dict(prepared=True,foregroundReady=False,launched=False,app=str(app),appSha256=sha(exe),bundleIdentifier=clone['bundleIdentifier'],
        sourceExecutableSha256=clone['sourceExecutableSha256'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),
        inspector=str(inspector),fixture=fixture,initialLibrary=library_state(out/'profile'),layoutSteps=len(steps)-1,
        frozenHashes={str(p):sha(p) for p in pins},scope='Current private app layout of selected Pulse inline Curve, with unique three-point curve and existing four-entry Curve toolbar. No global input or drag/drop hooks.',
        plannedPhysicalCase='Native inline editor Curve drag onto toolbar Curve B, unmodified, native name Editor Curve Copy; exact saved current/unlinked original points and original four entries preserved. Sequence must remain byte-identical.',
        limits=dict(allInSeconds=40,appSeconds=28,cleanupReserve=6),expectedAXLabel='Curve, 3 points')
    save(out/'prepared.json',manifest)
    assert tree(source)==manifest['sourceHashes']
    print(json.dumps(dict(prepared=str(out/'prepared.json'),launched=False,foregroundReady=False,steps=manifest['layoutSteps'])))
def run(out):
    began=time.monotonic();end=began+40;process=None;before=None;previous=None
    m=json.loads((out/'prepared.json').read_text());exe=Path(m['app'])/'Contents/MacOS/Vixen'
    r=dict(passed=False,acceptance=False,inputPosted=False,scope=m['scope'],appSha256=m['appSha256'],sourceExecutableSha256=m['sourceExecutableSha256'],finalizationErrors=[])
    try:
        assert out.is_relative_to(ROOT/'target') and not any((out/p).exists() for p in ['ui','runtime','result.json'])
        assert all(sha(p)==h for p,h in m['frozenHashes'].items())
        assert tree(Path(m['source']))==m['sourceHashes'] and tree(out/'profile')==m['preparedProfileHashes']
        before=preferences();previous=front()
        processes=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=2)
        assert not any(line.strip().endswith('/Contents/MacOS/Vixen') for line in processes.splitlines())
        (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
        env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_SETTINGS_DIR=str(out/'ui'),
            VIXEN_UI_TEST=str(out/'layout-steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log:
            assert end-time.monotonic()>=34,'Full child watchdog and cleanup no longer fit'
            process=subprocess.Popen([str(exe)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-process.json',dict(pid=process.pid,executable=str(exe),processGroup=process.pid))
            process.wait(timeout=min(28,end-time.monotonic()-6))
        r['exitCode']=process.returncode
        rows=json.loads((out/'ui/result.json').read_text());r['steps']=len(rows)
        assert process.returncode==0 and len(rows)==m['layoutSteps'] and all(row['ok'] for row in rows)
        states=[row['detail'] for row in rows if row.get('detail','').startswith('speed=')]
        assert len(states)==1 and 'running=false' in states[0]
        controls=(out/'ui/ready-controls.txt').read_text()
        assert m['expectedAXLabel'] in controls,'Expected unique Pulse Curve strip is not present'
        assert controls.count(m['expectedAXLabel'])==1,'Ambiguous Curve strip'
        r.update(passed=True,transportStopped=True,retainedFiles={str(p):sha(p) for p in [out/'ui/ready.png',out/'ui/ready-controls.txt',out/'ui/ready-menus.txt',out/'ui/before-toolbar.txt']})
    except BaseException as error:r['error']=repr(error)
    finally:
        try:terminate(process,end)
        except BaseException as error:r['finalizationErrors'].append(repr(error))
        r['ownedProcessExited']=process is not None and process.poll() is not None
        for key,check in [('sourceUnchanged',lambda:tree(Path(m['source']))==m['sourceHashes']),
            ('appUnchanged',lambda:sha(exe)==m['appSha256']),('profileUnchanged',lambda:tree(out/'profile')==m['preparedProfileHashes']),
            ('ownerPreferencesUnchanged',lambda:before is not None and preferences()==before),('foregroundUnchanged',lambda:previous is not None and front()==previous)]:
            try:r[key]=check()
            except BaseException as error:r[key]=False;r['finalizationErrors'].append(key+': '+repr(error))
        r['passed']=r['passed'] and all(r[k] for k in ['ownedProcessExited','sourceUnchanged','appUnchanged','profileUnchanged','ownerPreferencesUnchanged','foregroundUnchanged']) and not r['finalizationErrors']
        r['elapsedSeconds']=time.monotonic()-began;save(out/'result.json',r)
    print(json.dumps(r));return 0 if r['passed'] else 1
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--app',type=Path,default=ROOT/'target/private-app-raster-mapper-integration/Vixen.app')
    p.add_argument('--source',type=Path,default=ROOT/'target/editor-library-fixture-v2/profile')
    p.add_argument('--inspector',type=Path,default=ROOT/'target/build-main/rust/debug/examples/editor_library_fixture')
    mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-layout',action='store_true')
    a=p.parse_args()
    if a.prepare_only:prepare(a.out.resolve(),a.app.resolve(),a.source.resolve(),a.inspector.resolve());return 0
    return run(a.out.resolve())
if __name__=='__main__':raise SystemExit(main())
