#!/usr/bin/env python3
"""Prepare/run one bounded native Curve library click-collapse and double-click Cancel.

Preparation never launches Vixen or posts input. --run-prepared requires the
root's coordinated exclusive desktop slot. No drag or mutation hooks are used.
"""
import argparse,copy,hashlib,json,os,plistlib,re,shutil,signal,subprocess,sys,time
from pathlib import Path
import xml.etree.ElementTree as ET
from test_library_family_foreground import preferences
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'scripts/foreground_library_clicks.swift'
LAYOUT=ROOT/'target/library-cross-window-layout-03'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
local=lambda s:s.rsplit('}',1)[-1]
STAGES=['ready','unchanged']

def save(path,value):
    temporary=path.with_suffix(path.suffix+'.tmp');temporary.write_text(json.dumps(value,indent=2)+'\n');temporary.replace(path)
def tree(path):return {str(p.relative_to(path)):sha(p) for p in sorted(path.rglob('*')) if p.is_file()}
def xmlvalue(node):return [node.tag,[[key,value] for key,value in sorted(node.attrib.items())],(node.text or '').strip(),[xmlvalue(c) for c in node]]
def library_state(profile):
    root=ET.parse(profile/'SystemData/ModuleStore.xml').getroot();library=next(e for e in root.iter() if local(e.tag)=='CurveLibraryStaticData');entries=next(e for e in library if local(e.tag)=='_library')
    values=[]
    for entry in entries:
        key=next(c.text for c in entry if local(c.tag)=='Key');value=next(c for c in entry if local(c.tag)=='Value')
        values.append(dict(name=key,value=xmlvalue(value),current=next(c.text for c in value if local(c.tag)=='IsCurrentLibraryCurve'),link=next((c.text or '' for c in value if local(c.tag)=='_libraryReferenceName'),''),points=[[float(next(c.text for c in p if local(c.tag)==axis)) for axis in ['X','Y','Z']] for p in value.iter() if local(p.tag)=='PointPair']))
    for module in root.iter():
        if entries in list(module):module.remove(entries);break
    return dict(entries=values,otherXML=xmlvalue(root),moduleStoreSha256=sha(profile/'SystemData/ModuleStore.xml'))
def accept_checkpoint(name,state,m,previous):
    assert name in STAGES and state==m['initialLibrary'],'Selection/editor Cancel changed the exact saved library'
    return dict(state,passed=True,whileAlive=True)
def validate(out,m):
    assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
    assert m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    for path,digest in m['frozenHashes'].items():assert sha(path)==digest,'Frozen input drift: '+path
    assert tree(Path(m['source']))==m['sourceHashes'] and tree(out/'profile')==m['preparedProfileHashes']
    assert library_state(out/'profile')==m['initialLibrary']
    info=plistlib.loads((Path(m['app']).parents[1]/'Info.plist').read_bytes());assert info['CFBundleIdentifier']==m['bundleIdentifier']
    assert m['controllers']==0 and m['inputDeadlineSeconds']==40 and m['allInDeadlineSeconds']==75

def ensure_launch_budget(deadline,seconds=68):assert deadline-time.monotonic()>=seconds,'Full child watchdog and cleanup no longer fit'
def terminate(child,deadline):
    if child is None or child.poll() is not None:return
    try:os.killpg(child.pid,signal.SIGTERM)
    except ProcessLookupError:pass
    try:child.wait(timeout=min(1.5,max(.01,deadline-time.monotonic())))
    except subprocess.TimeoutExpired:
        try:os.killpg(child.pid,signal.SIGKILL)
        except ProcessLookupError:pass
        child.wait(timeout=min(1,max(.01,deadline-time.monotonic())))
def verify(out,m,result,process,driver):
    fg=json.loads((out/'foreground.json').read_text());result['foreground']=fg
    assert fg.get('passed') is True,'Helper failed: '+str(fg.get('error','no successful cleanup report'))
    assert process.returncode==driver.returncode==0,f'Native process exits: app={process.returncode}, helper={driver.returncode}'
    for key in ['restoredPrevious','restoredPointer','targetExited','noInputHeld','cleanupCompleted','inputOwnershipMaintained','exactCurveCEditorObserved','editorCancelDismissed','selectionCollapseOnReleaseObserved','inputCompleted']:assert fg.get(key) is True,key
    assert fg['mouseDownCount']==fg['mouseUpCount']==5 and fg['keyDownCount']==fg['keyUpCount']==1
    assert not fg['heldButtonAfterCleanup'] and not fg['heldKeyAfterCleanup']
    ids=fg['windowIDs'];assert set(ids)=={'editor'}
    pointer=[e for e in fg['events'] if 'point' in e];assert len(pointer)==14
    assert [e['kind'] for e in pointer]==[5,1,2]*3+[5,1,2,1,2]
    assert all(e['label']=='editor' and e['window']==ids['editor'] for e in pointer)
    assert [e['clickCount'] for e in pointer]==[1]*12+[2]*2
    assert [e['command'] for e in pointer]==[False]*3+[True]*3+[False]*8
    keyboard=[e for e in fg['events'] if 'key' in e]
    assert [(e['key'],e['down'],e['command']) for e in keyboard]==[(55,True,True),(55,False,False)]
    expected=[('initial',False,False,False),('selected-C',True,True,False),('selected-CD',False,True,False),('C-pressed-still-multiple',False,True,True),('C-released-collapsed',True,True,False),('after-editor-cancel',True,True,False)]
    states=[(v['stage'],v['editEnabled'],v['deleteEnabled'],v['mouseHeld']) for v in fg['observations'] if 'editEnabled' in v]
    assert states==expected,states
    assert set(result['checkpoints'])==set(STAGES)
    for name in STAGES:accept_checkpoint(name,{k:v for k,v in result['checkpoints'][name].items() if k not in ['passed','whileAlive']},m,result['checkpoints'])
    rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
    transports=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')];assert len(transports)==2 and all('running=false' in r for r in transports)
    log=(out/'process.log').read_text();assert 'library-drag session-start' not in log and 'library-drag cell-drop' not in log
    assert re.findall(r'library-drag down kind=curve index=(\d+) panel=(\w+) ',log)==[('2','true'),('3','true'),('2','true'),('2','true'),('2','true')]
    assert library_state(out/'profile')==m['initialLibrary']
    actual=tree(out/'profile')
    assert all(actual.get(k)==v for k,v in m['preparedProfileHashes'].items())
    assert set(actual)-set(m['preparedProfileHashes']) <= {'.lock','logs/Application.Info.log'}
    result.update(steps=len(rows),physicalClicks=5,exactSelectionStateTransitions=states,exactEditor='Curve C',cancelPreservesAllData=True,transportStopped=True)

def run(out,m):
    began=time.monotonic();deadline=began+75;process=driver=None;before=None
    result=dict(passed=False,scope=m['scope'],appSha256=m['appSha256'],sourceExecutableSha256=m['sourceExecutableSha256'],checkpoints={},allInDeadlineSeconds=75)
    try:
        before=preferences();validate(out,m)
        inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=2)
        assert not any(line.strip().endswith('/Contents/MacOS/Vixen') or line.strip().endswith(m['helper']) for line in inventory.splitlines()),'Another app/helper is running'
        (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_SETTINGS_DIR=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',VIXEN_LIBRARY_DRAG_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as dlog:
            ensure_launch_budget(deadline)
            process=subprocess.Popen([m['app']],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned=dict(appPID=process.pid,appExecutable=m['app'],appProcessGroup=process.pid);save(out/'owned-processes.json',owned)
            ensure_launch_budget(deadline,56)
            driver=subprocess.Popen([m['helper'],str(process.pid),str(out),m['app'],str(out/'calibration.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(driverPID=driver.pid,driverExecutable=m['helper'],driverProcessGroup=driver.pid);save(out/'owned-processes.json',owned)
            end=min(deadline-12,time.monotonic()+54);pending={}
            while process.poll() is None and time.monotonic()<end:
                if driver.poll() is not None:
                    fg=json.loads((out/'foreground.json').read_text()) if (out/'foreground.json').exists() else {}
                    assert fg.get('passed') is True,'Helper exited early: '+str(fg.get('error','no cleanup report'))
                for name in STAGES:
                    if name not in result['checkpoints'] and (out/('checkpoint-'+name+'.request')).exists():
                        try:state=accept_checkpoint(name,library_state(out/'profile'),m,result['checkpoints'])
                        except (AssertionError,ET.ParseError,OSError) as error:
                            first=pending.setdefault(name,time.monotonic())
                            result.setdefault('checkpointWaits',{})[name]=dict(lastError=repr(error),elapsedSeconds=time.monotonic()-first)
                            if time.monotonic()-first>=1.25:raise AssertionError('Saved checkpoint did not settle: '+name+': '+repr(error)) from error
                            continue
                        result['checkpoints'][name]=state;save(out/('checkpoint-'+name+'.json'),state)
                time.sleep(.05)
            assert process.poll() is not None,'Private app watchdog expired'
            driver.wait(timeout=min(4,max(.01,deadline-time.monotonic()-6)))
        result.update(exitCode=process.returncode,driverExitCode=driver.returncode)
        verify(out,m,result,process,driver);result['passed']=True
    except Exception as error:result['error']=repr(error)
    finally:
        errors=[]
        for child in [process,driver]:
            try:terminate(child,deadline)
            except Exception as error:errors.append(repr(error))
        if errors:result['cleanupErrors']=errors
        try:fg=json.loads((out/'foreground.json').read_text()) if (out/'foreground.json').exists() else {}
        except Exception as error:fg={};result['foregroundReadError']=repr(error)
        result['foreground']=fg
        result.update(processExited=process is not None and process.poll() is not None,driverExited=driver is not None and driver.poll() is not None,cleanupCompleted=not errors and fg.get('cleanupCompleted') is True,noInputHeld=fg.get('noInputHeld') is True,restoredPrevious=fg.get('restoredPrevious') is True,ownerPreferenceHashesBefore=before)
        try:result['ownerPreferenceHashesAfter']=preferences();result['ownerPreferencesUnchanged']=before is not None and before==result['ownerPreferenceHashesAfter']
        except Exception as error:result['ownerPreferencesUnchanged']=False;result['preferenceReadError']=repr(error)
        try:result['inputsUnchanged']=all(sha(path)==digest for path,digest in m['frozenHashes'].items()) and tree(Path(m['source']))==m['sourceHashes']
        except Exception as error:result['inputsUnchanged']=False;result['inputReadError']=repr(error)
        result['elapsedSeconds']=time.monotonic()-began;result['withinDeadline']=time.monotonic()<=deadline
        result['passed']=result['passed'] and all(result[k] for k in ['processExited','driverExited','cleanupCompleted','noInputHeld','restoredPrevious','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline'])
        save(out/'result.json',result)
    print(json.dumps({k:result[k] for k in ['passed','elapsedSeconds']},indent=2));return 0 if result['passed'] else 1

def prepare(out,app,helper,calibration):
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    layout=json.loads((LAYOUT/'prepared.json').read_text());proof=json.loads((LAYOUT/'result.json').read_text())
    assert proof['passed'] is True and proof['foregroundUnchanged'] is True
    source=Path(layout['source']);assert tree(source)==layout['sourceHashes']
    clone=json.loads((app.parent/'private-app.json').read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and sha(exe)==clone['executableSha256']==layout['appSha256']
    assert clone['bundleIdentifier']==layout['privateBundleIdentifier']
    out.mkdir();shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    shutil.copy2(source/'Sequence/Library Drag.tim',out/'profile/Sequence/Source Library.tim')
    h=lambda name,*args:dict(action='hook',name=name,path=list(args));w=lambda n:dict(action='wait',seconds=n)
    steps=[dict(action='window',id='administration'),w(1),h('admin.openSequence',str(out/'profile/Sequence/Source Library.tim')),w(2),
        h('sequencer.activate','Source Library'),dict(action='focus',window='Source Library'),h('sequencer.resize','1100','780'),
        h('sequencer.panel','Curve Library'),w(.6),h('sequencer.transport','state'),dict(action='menu',path=['View','Toolbars','Library Curve']),w(.6),
        h('sequencer.focusState'),dict(action='dump',name='ready-controls'),dict(action='menus',name='ready-menus'),dict(action='snapshot',name='ready'),
        w(24),h('sequencer.transport','state'),h('sequencer.state'),h('timelineAllPlayback'),dict(action='quit')]
    save(out/'steps.json',dict(steps=steps));cal=json.loads(calibration.read_text());cal['bundleIdentifier']=clone['bundleIdentifier'];save(out/'calibration.json',cal)
    assert len(cal['windows'])==1 and cal['pointC']==[1001.25,200.] and cal['pointD']==[1064.25,200.]
    assert set(cal['windows'][0]['scaleReferences'])=={'1','2'}
    older=ROOT/'target/library-cross-window-current-02'
    old_layout=json.loads((older/'prepared.json').read_text())
    assert old_layout['reviewedSourcePins']==layout['reviewedSourcePins'] and old_layout['sourceHashes']==layout['sourceHashes']
    assert len(ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot().find('Controllers'))==0
    initial=library_state(out/'profile')
    frozen=[Path(__file__).resolve(),SOURCE,ROOT/'scripts/test_library_family_foreground.py',out/'steps.json',out/'calibration.json',helper,exe,app/'Contents/Info.plist',app.parent/'private-app.json',LAYOUT/'prepared.json',LAYOUT/'result.json',LAYOUT/'ui/source-ready.png',ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift',ROOT/'app/Sources/VixenKit/CurveEditorDialog.swift']
    frozen += [older/'prepared.json',older/'ui/source-ready.png']
    m=dict(app=str(exe),appSha256=sha(exe),sourceExecutableSha256=layout['sourceExecutableSha256'],bundleIdentifier=clone['bundleIdentifier'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),initialLibrary=initial,controllers=0,helper=str(helper),helperSha256=sha(helper),steps=len(steps)-1,inputDeadlineSeconds=40,cleanupDeadlineSeconds=48,allInDeadlineSeconds=75,launchPerformed=False,foregroundAccepted=False,scope='Actual Curve library C/Command-D multi-selection, C press retains selection until release, real two-click editor open for Curve C, native Cancel. Exact original library and sequence bytes preserved; no mutation hooks or actual drags.',frozenHashes={str(p):sha(p) for p in frozen})
    save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],launched=False)))
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path);p.add_argument('--helper',type=Path);p.add_argument('--calibration',type=Path);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve(),a.calibration.resolve());return 0
    return run(out,json.loads((out/'prepared.json').read_text()))
if __name__=='__main__':sys.exit(main())
