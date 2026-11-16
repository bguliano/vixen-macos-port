#!/usr/bin/env python3
"""Prepare bounded Color/Gradient inline editor layouts; root alone runs native layouts.

No global input, color/drop/mutation hooks, or production edits. Layout captures
only establish geometry; separate physical gesture acceptance is still required.
"""
import argparse,json,os,plistlib,shutil,subprocess,time
from pathlib import Path
from test_editor_library_foreground import sha,tree,save,terminate
from test_library_family_foreground import preferences
from vixen2_physical_contract import typed_xml
ROOT=Path(__file__).resolve().parents[1]

def front():
    s=subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=2).strip();assert s.startswith('ASN:');return s

def prepare(out,kind,app,source,inspector):
    assert kind in ['color','gradient'] and not out.exists()
    assert all(p.is_relative_to(ROOT/'target') for p in [out,app,source,inspector])
    assert not any(p.is_symlink() for p in source.rglob('*'))
    clone=json.loads((app.parent/'private-app.json').read_text());exe=app/'Contents/MacOS/Vixen';info=plistlib.loads((app/'Contents/Info.plist').read_bytes())
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged'] and clone['executableSha256']==sha(exe)
    assert clone['sourceExecutableSha256']=='945941c664fc2796c32c7783b2eb8137f502dab0e6587a2ba0680746f2700865'
    assert info['CFBundleIdentifier']==clone['bundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    fixture=json.loads(subprocess.check_output([str(inspector),str(source),kind,'--inspect'],timeout=3))
    assert fixture['kind']==kind and fixture['controllers']==0 and fixture['effects']==1 and fixture['sourceColor']==0xff1177cb
    assert fixture['palette']==[0xffff0000,0xff00ff00,0xff0000ff,0xffffff00]
    assert fixture['sourceGradient']['link']=='' and fixture['sourceGradient']['current'] is False and len(fixture['sourceGradient']['colors'])==len(fixture['sourceGradient']['alphas'])==3
    for p in list(source.rglob('*.xml'))+list(source.rglob('*.tim')):typed_xml(p.read_bytes())
    out.mkdir();shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    title=fixture['title'];sequence=out/'profile/Sequence'/f'{title}.tim'
    hook=lambda n,*a:dict(action='hook',name=n,path=list(a));wait=lambda t:dict(action='wait',seconds=t)
    steps=[dict(action='window',id='administration'),wait(1),hook('admin.openSequence',str(sequence)),wait(2),hook('sequencer.activate',title),dict(action='focus',window=title),hook('sequencer.resize','1100','780'),wait(.6),dict(action='menu',path=['Edit','Select All Elements']),hook('sequencer.panel','Effect Editor'),wait(.6),hook('sequencer.transport','state'),dict(action='menus',name='before-toolbar'),dict(action='menu',path=['View','Toolbars','Library '+kind.title()]),wait(.6),hook('sequencer.focusState'),dict(action='dump',name='ready-controls'),dict(action='menus',name='ready-menus'),dict(action='snapshot',name='ready'),hook('timelineState'),hook('sequencer.state'),hook('timelineAllPlayback'),dict(action='quit')]
    save(out/'layout-steps.json',dict(steps=steps))
    pins=[Path(__file__).resolve(),ROOT/'scripts/test_editor_library_foreground.py',ROOT/'scripts/test_library_family_foreground.py',ROOT/'scripts/vixen2_physical_contract.py',ROOT/'crates/vixen-session/examples/editor_family_fixture.rs',inspector,exe,app/'Contents/Info.plist',app.parent/'private-app.json',out/'layout-steps.json',ROOT/'app/Sources/VixenKit/ColorControls.swift',ROOT/'app/Sources/VixenKit/GradientStripView.swift',ROOT/'app/Sources/VixenEffectEditor/RowEditors.swift',ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift']
    m=dict(prepared=True,launched=False,physicalAccepted=False,kind=kind,app=str(app),appSha256=sha(exe),sourceExecutableSha256=clone['sourceExecutableSha256'],bundleIdentifier=clone['bundleIdentifier'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),inspector=str(inspector),fixture=fixture,title=title,openFile=str(sequence),layoutSteps=len(steps)-1,limits=dict(allInSeconds=40,appSeconds=28,cleanupReserve=6),frozenHashes={str(p):sha(p) for p in pins},scope='Private current whole-app '+kind+' editor-to-library layout only. Actual menu opens toolbar; no global input/drag or mutation hooks. No physical acceptance.',plannedPhysicalCase='Color source0xff1177cb copied into palette at toolbarB; original sequence unchanged.' if kind=='color' else 'Three XYZ/alpha point source copied via native Gradient name sheet, then second name sheet Cancel; exact saved gradient and byte-preserving Cancel, original sequence unchanged.')
    save(out/'prepared.json',m);print(json.dumps(dict(prepared=str(out/'prepared.json'),kind=kind,launched=False)))

def validate(out,m):
    assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json'])
    assert all(sha(p)==v for p,v in m['frozenHashes'].items())
    assert tree(Path(m['source']))==m['sourceHashes'] and tree(out/'profile')==m['preparedProfileHashes']
    for p in list((out/'profile').rglob('*.xml'))+list((out/'profile').rglob('*.tim')):typed_xml(p.read_bytes())
    assert m['kind'] in ['color','gradient'] and m['layoutSteps']==22 and m['limits']==dict(allInSeconds=40,appSeconds=28,cleanupReserve=6)
    info=plistlib.loads((Path(m['app'])/'Contents/Info.plist').read_bytes());assert info['CFBundleIdentifier']==m['bundleIdentifier']

def run(out):
    began=time.monotonic();deadline=began+40;process=None;before=previous=None;m=json.loads((out/'prepared.json').read_text());exe=Path(m['app'])/'Contents/MacOS/Vixen'
    result=dict(passed=False,physicalAccepted=False,inputPosted=False,scope=m['scope'],kind=m['kind'],finalizationErrors=[])
    try:
        validate(out,m);before=preferences();previous=front()
        ps=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=2)
        assert not any(x.strip().endswith('/Contents/MacOS/Vixen') or '/foreground-' in x for x in ps.splitlines()),'Another native verifier owns desktop'
        (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_SETTINGS_DIR=str(out/'ui'),VIXEN_UI_TEST=str(out/'layout-steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log:
            assert deadline-time.monotonic()>=34,'Full child budget and cleanup unavailable'
            process=subprocess.Popen([str(exe),'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-process.json',dict(pid=process.pid,processGroup=process.pid,executable=str(exe)))
            process.wait(timeout=min(28,deadline-time.monotonic()-6))
        result['exitCode']=process.returncode;rows=json.loads((out/'ui/result.json').read_text());assert process.returncode==0 and len(rows)==m['layoutSteps'] and all(r['ok'] for r in rows)
        states=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')];assert len(states)==1 and 'running=false' in states[0]
        timeline=next(json.loads(r['detail']) for r in rows if '"activeRow"' in r.get('detail',''));assert timeline['selected']==[1] and timeline['modified'] is False and timeline['transport']=='stopped'
        assert timeline['elements'][0]['type']==('Set Level' if m['kind']=='color' else 'Pulse')
        for name in ['ready.png','ready-controls.txt','ready-menus.txt','before-toolbar.txt']:assert (out/'ui'/name).is_file()
        inspection=json.loads(subprocess.check_output([m['inspector'],str(out/'profile'),m['kind'],'--inspect',str(out/'ui/ColorPalette.xml')],timeout=3));expected=dict(m['fixture'],sequence=m['openFile']);assert inspection==expected
        result.update(passed=True,steps=len(rows),transportStopped=True,sourceValuesUnchanged=True,inspection=inspection,retainedFiles={str(p):sha(p) for p in (out/'ui').iterdir() if p.is_file()})
    except BaseException as error:result['error']=repr(error)
    finally:
        try:terminate(process,deadline)
        except BaseException as error:result['finalizationErrors'].append(repr(error))
        result['ownedProcessExited']=process is not None and process.poll() is not None
        def profile_same():
            actual=tree(out/'profile');return all(actual.get(k)==v for k,v in m['preparedProfileHashes'].items()) and set(actual)-set(m['preparedProfileHashes'])<={'.lock','logs/Application.Info.log'}
        for key,check in [('sourceUnchanged',lambda:tree(Path(m['source']))==m['sourceHashes']),('appUnchanged',lambda:sha(exe)==m['appSha256']),('profileUnchanged',profile_same),('ownerPreferencesUnchanged',lambda:before is not None and preferences()==before),('foregroundUnchanged',lambda:previous is not None and front()==previous),('frozenInputsUnchanged',lambda:all(sha(p)==v for p,v in m['frozenHashes'].items()))]:
            try:result[key]=check()
            except BaseException as error:result[key]=False;result['finalizationErrors'].append(key+': '+repr(error))
        result['elapsedSeconds']=time.monotonic()-began;result['passed']=result['passed'] and all(result[k] for k in ['ownedProcessExited','sourceUnchanged','appUnchanged','profileUnchanged','ownerPreferencesUnchanged','foregroundUnchanged','frozenInputsUnchanged']) and not result['finalizationErrors'] and result['elapsedSeconds']<40
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','kind','steps','error','elapsedSeconds']}));return 0 if result['passed'] else 1
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--kind',choices=['color','gradient']);p.add_argument('--app',type=Path,default=ROOT/'target/private-app-vixen2-drop-location/Vixen.app');p.add_argument('--source',type=Path);p.add_argument('--inspector',type=Path,default=ROOT/'target/build-main/rust/debug/examples/editor_family_fixture');g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-layout',action='store_true');a=p.parse_args()
    if a.prepare_only:prepare(a.out.resolve(),a.kind,a.app.resolve(),a.source.resolve(),a.inspector.resolve())
    else:raise SystemExit(run(a.out.resolve()))
