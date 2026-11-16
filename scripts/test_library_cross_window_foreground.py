#!/usr/bin/env python3
"""Prepare/run one bounded native cross-window Curve copy and linked rejection.

Preparation never launches Vixen or posts input. --run-prepared requires the
root's coordinated exclusive desktop slot. No drag or mutation hooks are used.
"""
import argparse,copy,hashlib,json,os,plistlib,re,shutil,signal,subprocess,sys,time
from pathlib import Path
import xml.etree.ElementTree as ET
from test_library_family_foreground import preferences
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'scripts/foreground_library_cross_window.swift'
LAYOUT=ROOT/'target/library-cross-window-layout-03'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
local=lambda s:s.rsplit('}',1)[-1]
STAGES=['ready','copied','rejected']

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
    initial=m['initialLibrary'];expected=copy.deepcopy(initial['entries'])
    if name!='ready':expected.append(dict(expected[0],name='Cross Window Curve'))
    assert state['entries']==expected,'Exact library entries differ at '+name
    assert state['otherXML']==initial['otherXML'],'Unrelated module data changed'
    assert [v['name'] for v in state['entries']]==['Curve A','Curve B','Curve C','Curve D']+([] if name=='ready' else ['Cross Window Curve'])
    assert [v['points'][0][1] for v in state['entries']]==[10.,20.,30.,40.]+([] if name=='ready' else [10.])
    if name=='rejected':assert state['moduleStoreSha256']==previous['copied']['moduleStoreSha256'],'Linked rejection modified saved ModuleStore'
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
    assert process.returncode==driver.returncode==0 and fg.get('passed') is True
    for key in ['restoredPrevious','restoredPointer','targetExited','noInputHeld','cleanupCompleted','inputOwnershipMaintained','nativeNameSheetObserved','nameSheetDismissed','linkedDragHadNoNameSheet','inputCompleted']:assert fg.get(key) is True,key
    assert fg['enteredName']=='Cross Window Curve' and fg['mouseDownCount']==fg['mouseUpCount']==2 and fg['keyDownCount']==fg['keyUpCount']==4
    assert fg['heldButtonAfterCleanup'] is False and fg['heldKeyAfterCleanup'] is False
    ids=fg['windowIDs'];assert set(ids)=={'source','destination'} and ids['source']!=ids['destination']
    pointer=[e for e in fg['events'] if 'point' in e];assert len(pointer)==62
    for events,command in [(pointer[:31],False),(pointer[31:],True)]:
        assert all(e['command'] is (command if i else False) for i,e in enumerate(events)) and events[0]['label']=='source' and events[1]['label']=='source' and events[-1]['label']=='destination'
        assert [e['kind'] for e in events]==[5,1]+[6]*28+[2]
        assert all(e['window']==ids[e['label']] for e in events)
    assert set(result['checkpoints'])==set(STAGES) and all(v['whileAlive'] and v['passed'] for v in result['checkpoints'].values())
    rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
    transports=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')];assert len(transports)==4 and all('running=false' in r for r in transports)
    focus=[json.loads(r['detail']) for r in rows if '"captured"' in r.get('detail','')];assert len(focus)==2 and focus[0]['captured']['identity']!=focus[1]['captured']['identity']
    log=(out/'process.log').read_text();started=re.findall(r'library-drag session-start kind=curve index=(\d+) ',log);ended=re.findall(r'library-drag session-ended kind=curve index=(\d+) operation=(\d+) ',log)
    assert started==['0','0'] and ended==[('0','1'),('0','0')],(started,ended)
    drops=re.findall(r'library-drag cell-drop kind=curve index=(\d+) own=(\w+) ',log);assert drops==[('1','false')],drops
    assert 'library-drag down kind=curve index=0 panel=false' in log
    assert library_state(out/'profile')=={k:v for k,v in result['checkpoints']['rejected'].items() if k not in ['passed','whileAlive']}
    expected=m['preparedProfileHashes'];actual=tree(out/'profile')
    for name,digest in expected.items():
        if name!='SystemData/ModuleStore.xml':assert actual.get(name)==digest,'Private fixture changed: '+name
    result.update(steps=len(rows),actualDragSessions=2,nativeSessionEndedOperations=[1,0],exactSavedStateVerified=True,transportStopped=True)

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
    subprocess.run([sys.executable,'-B',str(ROOT/'scripts/prepare_library_cross_window.py'),'--out',str(out),'--app',str(app)],check=True,timeout=10,capture_output=True)
    base=json.loads((out/'prepared.json').read_text());steps=json.loads((out/'layout-steps.json').read_text())['steps'][:-3]
    h=lambda name,*args:dict(action='hook',name=name,path=list(args))
    steps += [dict(action='wait',seconds=24),h('sequencer.activate','Source Library'),h('sequencer.transport','state'),h('sequencer.activate','Destination Library'),h('sequencer.transport','state'),h('timelineAllPlayback'),dict(action='quit')]
    save(out/'steps.json',dict(steps=steps));cal=json.loads(calibration.read_text());cal['bundleIdentifier']=base['privateBundleIdentifier'];save(out/'calibration.json',cal)
    source=Path(base['source']);exe=app/'Contents/MacOS/Vixen';initial=library_state(out/'profile');assert [v['name'] for v in initial['entries']]==['Curve A','Curve B','Curve C','Curve D']
    frozen=[Path(__file__).resolve(),SOURCE,ROOT/'scripts/prepare_library_cross_window.py',ROOT/'scripts/test_library_family_foreground.py',out/'steps.json',out/'calibration.json',helper,exe,app/'Contents/Info.plist',app.parent/'private-app.json',LAYOUT/'prepared.json',LAYOUT/'result.json']+[Path(c['referenceImage']) for c in cal['windows']]
    system=ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot();controllers=len(next(e for e in system if local(e.tag)=='Controllers'));assert controllers==0
    m=dict(app=str(exe),appSha256=sha(exe),sourceExecutableSha256=base['sourceExecutableSha256'],bundleIdentifier=base['privateBundleIdentifier'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),initialLibrary=initial,controllers=controllers,helper=str(helper),helperSha256=sha(helper),steps=len(steps)-1,inputDeadlineSeconds=40,cleanupDeadlineSeconds=48,allInDeadlineSeconds=75,launchPerformed=False,foregroundAccepted=False,scope='Native same-PID distinct-window Curve toolbar copy with actual name and Command-linked rejection; controller-free private fixture; observed layout03, per-point native IDs, exact saved values, bounded owned cleanup; no mutation/drag hooks.',frozenHashes={str(p):sha(p) for p in frozen})
    save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],launched=False)))
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path);p.add_argument('--helper',type=Path);p.add_argument('--calibration',type=Path);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve(),a.calibration.resolve());return 0
    return run(out,json.loads((out/'prepared.json').read_text()))
if __name__=='__main__':sys.exit(main())
