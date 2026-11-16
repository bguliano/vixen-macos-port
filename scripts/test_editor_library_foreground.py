#!/usr/bin/env python3
"""Prepare/run one bounded native inline editor→Curve toolbar copy.

Preparation never launches Vixen or posts input. --run-prepared requires the
root's coordinated exclusive desktop slot. No drag or mutation hooks are used.
"""
import argparse,copy,hashlib,json,os,plistlib,re,shutil,signal,subprocess,sys,time
from pathlib import Path
import xml.etree.ElementTree as ET
from test_library_family_foreground import preferences
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'scripts/foreground_editor_library.swift'
LAYOUT=ROOT/'target/editor-library-layout-01'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
local=lambda s:s.rsplit('}',1)[-1]
STAGES=['ready','copied']

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
    if name=='copied':expected.append(m['expectedEntry'])
    assert name in STAGES and state['entries']==expected,'Exact library entries differ at '+name
    assert state['otherXML']==initial['otherXML'],'Unrelated module data changed'
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
    for key in ['restoredPrevious','restoredPointer','targetExited','noInputHeld','cleanupCompleted','inputOwnershipMaintained','nativeNameSheetObserved','nameSheetDismissed','inputCompleted']:assert fg.get(key) is True,key
    assert fg['enteredName']=='Editor Curve Copy' and fg['mouseDownCount']==fg['mouseUpCount']==1 and fg['keyDownCount']==fg['keyUpCount']==3
    assert not fg['heldButtonAfterCleanup'] and not fg['heldKeyAfterCleanup']
    ids=fg['windowIDs'];assert set(ids)=={'editor'}
    pointer=[e for e in fg['events'] if 'point' in e];assert len(pointer)==31
    assert [e['kind'] for e in pointer]==[5,1]+[6]*28+[2]
    assert all(e['command'] is False and e['label']=='editor' and e['window']==ids['editor'] for e in pointer)
    keyboard=[e for e in fg['events'] if 'key' in e]
    assert [(e['key'],e['down'],e['command']) for e in keyboard]==[(55,True,True),(0,True,True),(0,False,True),(55,False,False),(0,True,False),(0,False,False)]
    assert set(result['checkpoints'])==set(STAGES)
    for name in STAGES:accept_checkpoint(name,result['checkpoints'][name],m,result['checkpoints'])
    rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
    transports=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')];assert len(transports)==2 and all('running=false' in r for r in transports)
    timeline=next(json.loads(r['detail']) for r in rows if '"activeRow"' in r.get('detail',''))
    assert timeline['selected']==[1] and timeline['elements'][0]['type']=='Pulse' and timeline['modified'] is False and timeline['transport']=='stopped'
    log=(out/'process.log').read_text();drops=re.findall(r'library-drag cell-drop kind=curve index=(\d+) own=(\w+) ',log);assert drops==[('1','false')],drops
    assert 'library-drag down kind=curve' not in log,'Source unexpectedly originated from a library cell'
    assert library_state(out/'profile')=={k:v for k,v in result['checkpoints']['copied'].items() if k not in ['passed','whileAlive']}
    actual=tree(out/'profile')
    for name,digest in m['preparedProfileHashes'].items():
        if name!='SystemData/ModuleStore.xml':assert actual.get(name)==digest,'Private fixture changed: '+name
    assert set(actual)-set(m['preparedProfileHashes']) <= {'.lock','logs/Application.Info.log'},'Unexpected private profile output'
    inspection=json.loads(subprocess.check_output([m['inspector'],str(out/'profile'),'--inspect'],timeout=3))
    assert inspection['sourcePoints']==m['fixture']['sourcePoints'] and inspection['sourceCurrent'] is False and inspection['sourceLink']==''
    assert inspection['curves'][-1]==dict(name='Editor Curve Copy',points=m['fixture']['sourcePoints'],link='',current=True)
    result.update(steps=len(rows),actualDragSessions=1,exactSavedStateVerified=True,sourceSequenceUnchanged=True,transportStopped=True,inspection=inspection)

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

def expected_entry(initial,fixture):
    entry=copy.deepcopy(initial['entries'][0]);entry['name']='Editor Curve Copy'
    points=next(v for v in entry['value'][3] if local(v[0])=='Points');template=copy.deepcopy(points[3][0]);points[3]=[]
    for x,y in fixture['sourcePoints']:
        point=copy.deepcopy(template)
        for axis in point[3]:
            if local(axis[0]) in ['X','Y','Z']:axis[2]=format({'X':x,'Y':y,'Z':0}[local(axis[0])],'.15g')
        points[3].append(point)
    entry['points']=[[*p,0.] for p in fixture['sourcePoints']]
    return entry

def prepare(out,app,helper,calibration):
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    layout=json.loads((LAYOUT/'prepared.json').read_text());proof=json.loads((LAYOUT/'layout-readiness.json').read_text())
    assert proof['passed'] is True and proof['acceptance'] is False and proof['rawResultSha256']==sha(LAYOUT/'result.json')
    assert proof['imageSha256']==sha(LAYOUT/'ui/ready.png')
    source=Path(layout['source']);assert tree(source)==layout['sourceHashes']
    clone=json.loads((app.parent/'private-app.json').read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and sha(exe)==clone['executableSha256']==layout['appSha256']
    assert clone['bundleIdentifier']==layout['bundleIdentifier']
    out.mkdir();shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    steps=json.loads((LAYOUT/'layout-steps.json').read_text())['steps']
    for row in steps:
        if row.get('name')=='admin.openSequence':row['path']=[str(out/'profile/Sequence/Editor Library.tim')]
    snapshot=next(i for i,row in enumerate(steps) if row.get('action')=='snapshot' and row.get('name')=='ready')
    steps.insert(snapshot+1,dict(action='wait',seconds=24));steps.insert(-1,dict(action='hook',name='sequencer.transport',path=['state']))
    save(out/'steps.json',dict(steps=steps));cal=json.loads(calibration.read_text());cal['bundleIdentifier']=layout['bundleIdentifier'];save(out/'calibration.json',cal)
    assert len(cal['windows'])==1 and cal['sourcePoint']==[970.,299.] and cal['destinationPoint']==[37.75,81.]
    assert set(cal['windows'][0]['scaleReferences'])=={'1','2'}
    observed=ROOT/'target/editor-library-physical-v3'
    observed_prepared=json.loads((observed/'prepared.json').read_text())
    observed_result=json.loads((observed/'result.json').read_text())
    assert observed_prepared['appSha256']==layout['appSha256'] and observed_prepared['sourceHashes']==layout['sourceHashes']
    assert observed_result['inputsUnchanged'] is True and observed_result['ownerPreferencesUnchanged'] is True
    assert observed_result['foreground']['events']==[] and observed_result['foreground']['pointerInputPosted'] is False
    initial=library_state(out/'profile');fixture=layout['fixture'];expected=expected_entry(initial,fixture)
    frozen=[Path(__file__).resolve(),SOURCE,ROOT/'scripts/test_library_family_foreground.py',out/'steps.json',out/'calibration.json',helper,exe,app/'Contents/Info.plist',app.parent/'private-app.json',LAYOUT/'prepared.json',LAYOUT/'result.json',LAYOUT/'layout-readiness.json',LAYOUT/'ui/ready.png',Path(layout['inspector']),ROOT/'crates/vixen-session/examples/editor_library_fixture.rs']
    frozen += [observed/'prepared.json',observed/'result.json',observed/'foreground.json',observed/'ui/ready.png']
    m=dict(app=str(exe),appSha256=sha(exe),sourceExecutableSha256=layout['sourceExecutableSha256'],bundleIdentifier=layout['bundleIdentifier'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),initialLibrary=initial,expectedEntry=expected,fixture=fixture,inspector=layout['inspector'],controllers=0,helper=str(helper),helperSha256=sha(helper),steps=len(steps)-1,inputDeadlineSeconds=40,cleanupDeadlineSeconds=48,allInDeadlineSeconds=75,launchPerformed=False,foregroundAccepted=False,scope='Actual native unmodified inline Pulse Curve→same-window Curve toolbar copy, native name sheet, exact saved three-point value and unchanged source sequence. Private controller-free d129 fixture, no drag/mutation hooks.',frozenHashes={str(p):sha(p) for p in frozen})
    save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],launched=False)))
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path);p.add_argument('--helper',type=Path);p.add_argument('--calibration',type=Path);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve(),a.calibration.resolve());return 0
    return run(out,json.loads((out/'prepared.json').read_text()))
if __name__=='__main__':sys.exit(main())
