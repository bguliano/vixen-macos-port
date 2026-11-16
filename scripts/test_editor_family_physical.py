#!/usr/bin/env python3
"""Private native Color/Gradient inline editor-to-library gestures; root launches only."""
import argparse,copy,json,os,plistlib,re,shutil,subprocess,time
from pathlib import Path
import xml.etree.ElementTree as ET
import editor_family_contract as contract
from test_editor_library_foreground import sha,tree,save,terminate,ensure_launch_budget
from test_library_family_foreground import preferences
from vixen2_physical_contract import typed_xml
from prepare_private_test_app import code_digest
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'scripts/foreground_editor_family.swift'

def candidate_binding(app,approved_source_sha,canonical_app,build_inputs):
    assert app.is_relative_to(ROOT/'target') and canonical_app.is_relative_to(ROOT/'target') and build_inputs.is_relative_to(ROOT/'target')
    assert re.fullmatch('[0-9a-f]{64}',approved_source_sha),'Explicit approved source hash required'
    clonepath=app.parent/'private-app.json';clone=json.loads(clonepath.read_text());exe=app/'Contents/MacOS/Vixen';canonical_exe=canonical_app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged']
    assert Path(clone['app']).resolve()==app and Path(clone['sourceApp']).resolve()==canonical_app
    assert clone['sourceExecutableSha256']==sha(canonical_exe)==approved_source_sha
    assert clone['executableSha256']==sha(exe) and code_digest(exe)==code_digest(canonical_exe)==clone['executableCodeDigest']
    bundle=plistlib.loads((app/'Contents/Info.plist').read_bytes())['CFBundleIdentifier'];assert bundle==clone['bundleIdentifier'] and bundle.startswith('com.vixenlights.verification.')
    inputs=json.loads(build_inputs.read_text())['sources'];assert len(inputs)==811 and all(sha(ROOT/path)==digest for path,digest in inputs.items()),'Candidate production input drift'
    return dict(app=str(app),canonicalApp=str(canonical_app),approvedSourceSha256=approved_source_sha,buildInputs=str(build_inputs),codeDigest=clone['executableCodeDigest'],bundleIdentifier=bundle)

def validate(out,m):
    assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
    assert m['kind'] in ['color','gradient'] and m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    assert all(sha(p)==h for p,h in m['frozenHashes'].items()),'Frozen input drift'
    assert tree(Path(m['source']))==m['sourceHashes'] and tree(out/'profile')==m['preparedProfileHashes']
    assert contract.family_state(out/'profile',out/'profile/ColorPalette.xml')==m['initialState']
    assert plistlib.loads((Path(m['app']).parents[1]/'Info.plist').read_bytes())['CFBundleIdentifier']==m['bundleIdentifier']
    assert m['controllers']==0 and [m[k] for k in ['inputDeadlineSeconds','cleanupDeadlineSeconds','allInDeadlineSeconds']]==[40,48,75]
    system=ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot();assert all(len(system.find(n))==0 for n in ['Controllers','Patches','Previews'])
    root=typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());assert [e.text for e in root.iter() if 'WebServer' in e.tag and e.tag.endswith('}IsEnabled')]==['false']
    assert contract.prepare_contract(out/'profile',m['kind'],m['openFile'])=={k:m[k] for k in contract.prepare_contract(out/'profile',m['kind'],m['openFile'])}
    if 'candidate' in m:
        c=m['candidate'];assert candidate_binding(Path(c['app']),c['approvedSourceSha256'],Path(c['canonicalApp']),Path(c['buildInputs']))==c
        assert m['sourceExecutableSha256']==c['approvedSourceSha256'] and m['app']==str(Path(c['app'])/'Contents/MacOS/Vixen') and m['bundleIdentifier']==c['bundleIdentifier']

def verify(out,m,result,process,driver):
    fg=json.loads((out/'foreground.json').read_text());result['foreground']=fg
    counts=contract.verify_foreground(fg,m)
    assert process.returncode==driver.returncode==0
    previous={}
    for name in contract.stages(m['kind']):
        saved=result['checkpoints'][name];state={k:v for k,v in saved.items() if k not in ['passed','whileAlive']}
        assert contract.accept_checkpoint(name,state,m,previous)==saved
        assert json.loads((out/('checkpoint-'+name+'.json')).read_text())==saved;previous[name]=saved
    assert set(previous)==set(result['checkpoints'])
    final=contract.family_state(out/'profile',out/'ui/ColorPalette.xml')
    assert final=={k:v for k,v in result['checkpoints'][contract.stages(m['kind'])[-1]].items() if k not in ['passed','whileAlive']}
    rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
    transports=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')];assert len(transports)==2 and all('running=false' in r for r in transports)
    timeline=next(json.loads(r['detail']) for r in rows if '"activeRow"' in r.get('detail',''))
    assert timeline['selected']==[1] and timeline['elements'][0]['type']==('Set Level' if m['kind']=='color' else 'Pulse') and timeline['modified'] is False and timeline['transport']=='stopped'
    log=(out/'process.log').read_text();drops=re.findall(r'library-drag cell-drop kind=(\w+) index=(\d+) own=(\w+) ',log)
    assert drops==[(m['kind'],'1','false')]*(2 if m['kind']=='gradient' else 1),drops
    assert 'library-drag down kind=' not in log,'Source unexpectedly originated from a library cell'
    profile=contract.accept_profile(out,m)
    inspection=json.loads(subprocess.check_output([m['inspector'],str(out/'profile'),m['kind'],'--inspect',str(out/'ui/ColorPalette.xml')],timeout=3))
    expected=copy.deepcopy(m['fixture']);expected['sequence']=m['openFile']
    if m['kind']=='color':expected['palette'].insert(1,contract.SOURCE_COLOR)
    else:
        gradient=copy.deepcopy(expected['sourceGradient']);gradient['current']=True
        expected['gradients'].append(dict(name=contract.COPY_NAME,value=gradient))
    assert inspection==expected,'Rust native-file readback differs from exact expected source/library values'
    result.update(steps=len(rows),**counts,exactSavedStateVerified=True,sourceSequenceUnchanged=True,transportStopped=True,inspection=inspection,privateProfileValidation=profile)

def run(out,m):
    began=time.monotonic();deadline=began+75;process=driver=None;before=None;stages=contract.stages(m["kind"])
    result=dict(passed=False,scope=m['scope'],appSha256=m['appSha256'],sourceExecutableSha256=m['sourceExecutableSha256'],checkpoints={},allInDeadlineSeconds=75)
    try:
        before=preferences();validate(out,m)
        inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=2)
        assert not any(line.strip().endswith('/Contents/MacOS/Vixen') or '/foreground-' in line for line in inventory.splitlines()),'Another app/helper is running'
        (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_SETTINGS_DIR=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',VIXEN_LIBRARY_DRAG_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as dlog:
            ensure_launch_budget(deadline)
            process=subprocess.Popen([m['app'],'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned=dict(appPID=process.pid,appExecutable=m['app'],appProcessGroup=process.pid);save(out/'owned-processes.json',owned)
            ensure_launch_budget(deadline,56)
            driver=subprocess.Popen([m['helper'],str(process.pid),str(out),m['app'],str(out/'calibration.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(driverPID=driver.pid,driverExecutable=m['helper'],driverProcessGroup=driver.pid);save(out/'owned-processes.json',owned)
            end=min(deadline-12,time.monotonic()+54);pending={}
            while process.poll() is None and time.monotonic()<end:
                if driver.poll() is not None:
                    fg=json.loads((out/'foreground.json').read_text()) if (out/'foreground.json').exists() else {}
                    assert fg.get('passed') is True,'Helper exited early: '+str(fg.get('error','no cleanup report'))
                for name in stages:
                    if name not in result['checkpoints'] and (out/('checkpoint-'+name+'.request')).exists():
                        try:
                            request=out/('checkpoint-'+name+'.request');assert request.is_file() and not request.is_symlink() and request.read_text()=='ready','Unexpected checkpoint request'
                            state=contract.accept_checkpoint(name,contract.family_state(out/'profile',out/'ui/ColorPalette.xml'),m,result['checkpoints'])
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
        if driver is not None and driver.poll() is None:
            try:save(out/'abort-request.json',dict(abort=True))
            except Exception as error:errors.append('abort request: '+repr(error))
            try:driver.wait(timeout=min(4,max(.01,deadline-time.monotonic()-5)))
            except subprocess.TimeoutExpired:pass
            except Exception as error:errors.append('helper cleanup wait: '+repr(error))
        for child in [driver,process]:
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

def prepare(out,kind,helper,helper_preparation,app_override=None,approved_source_sha=None,canonical_app=None,build_inputs=None,inspector_override=None):
    assert out.is_relative_to(ROOT/'target') and not out.exists() and kind in ['color','gradient']
    layout_dir=ROOT/('target/editor-family-'+kind+'-layout-01');layout=json.loads((layout_dir/'prepared.json').read_text());raw=json.loads((layout_dir/'result.json').read_text())
    assert raw['passed'] and raw['steps']==22 and raw['foregroundUnchanged'] and raw['sourceUnchanged'] and raw['ownerPreferencesUnchanged']
    inspector=inspector_override or Path(layout['inspector'])
    assert inspector.is_relative_to(ROOT/'target') and sha(inspector)==layout['frozenHashes'][layout['inspector']],'Inspector must be byte-identical to accepted layout oracle'
    assert all(sha(inspector if p==layout['inspector'] else p)==h for p,h in layout['frozenHashes'].items())
    prep=json.loads(helper_preparation.read_text());assert prep['passed'] and prep['strictSignatureVerified'] and prep['sourceSha256']==sha(SOURCE) and prep['helperSha256']==sha(helper)
    assert all(sha(p)==h for p,h in prep['contractPins'].items())
    calibration=next(row for row in prep['calibrations'] if row['kind']==kind);calpath=Path(calibration['path']);assert sha(calpath)==calibration['sha256'];cal=json.loads(calpath.read_text())
    assert cal['kind']==kind and len(cal['windows'])==1 and cal['windows'][0]['referenceImageSha256']==sha(layout_dir/'ui/ready.png')
    app=Path(layout['app']);exe=app/'Contents/MacOS/Vixen';source=Path(layout['source']);assert sha(exe)==layout['appSha256'] and tree(source)==layout['sourceHashes']
    candidate=None;overrides=[app_override,approved_source_sha,canonical_app,build_inputs]
    assert not any(overrides) or all(overrides),'All four candidate binding arguments are required together'
    if all(overrides):
        candidate=candidate_binding(app_override,approved_source_sha,canonical_app,build_inputs);app=app_override;exe=app/'Contents/MacOS/Vixen'
    out.mkdir();shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    sequence=out/'profile/Sequence'/Path(layout['openFile']).name
    steps=json.loads((layout_dir/'layout-steps.json').read_text())['steps']
    for row in steps:
        if row.get('name')=='admin.openSequence':row['path']=[str(sequence)]
    index=next(i for i,row in enumerate(steps) if row.get('action')=='snapshot' and row.get('name')=='ready')
    steps.insert(index+1,dict(action='wait',seconds=24));steps.insert(-1,dict(action='hook',name='sequencer.transport',path=['state']))
    save(out/'steps.json',dict(steps=steps));cal['bundleIdentifier']=candidate['bundleIdentifier'] if candidate else layout['bundleIdentifier'];save(out/'calibration.json',cal)
    pins=[Path(__file__).resolve(),SOURCE,ROOT/'scripts/editor_family_contract.py',ROOT/'scripts/test_editor_family_contract.py',ROOT/'scripts/test_editor_family_runner_controls.py',ROOT/'scripts/test_editor_family_candidate_controls.py',ROOT/'scripts/test_editor_library_foreground.py',ROOT/'scripts/test_library_family_foreground.py',ROOT/'scripts/vixen2_physical_contract.py',helper,helper_preparation,calpath,out/'steps.json',out/'calibration.json',exe,app/'Contents/Info.plist',app.parent/'private-app.json',layout_dir/'prepared.json',layout_dir/'result.json',layout_dir/'ui/ready.png',Path(layout['inspector']),ROOT/'crates/vixen-session/examples/editor_family_fixture.rs']+[Path(p) for p in layout['frozenHashes']]
    pins=[inspector if p==Path(layout['inspector']) else p for p in pins]
    m=dict(app=str(exe),appSha256=sha(exe),sourceExecutableSha256=layout['sourceExecutableSha256'],bundleIdentifier=layout['bundleIdentifier'],source=str(source),sourceHashes=tree(source),preparedProfileHashes=tree(out/'profile'),fixture=layout['fixture'],inspector=str(inspector),controllers=0,helper=str(helper),helperSha256=sha(helper),steps=len(steps)-1,openFile=str(sequence),sourcePoint=cal['sourcePoint'],destinationPoint=cal['destinationPoint'],inputDeadlineSeconds=40,cleanupDeadlineSeconds=48,allInDeadlineSeconds=75,launchPerformed=False,foregroundAccepted=False,scope='Actual native '+kind+' inline editor-to-matching-library toolbar copy'+(' and second naming-sheet Cancel' if kind=='gradient' else '')+'; strict persisted value/order/typed XML, exact source bytes and private cleanup. No real outputs or production edits.',frozenHashes={str(p):sha(p) for p in pins},**contract.prepare_contract(out/'profile',kind,sequence))
    if candidate:
        m.update(candidate=candidate,sourceExecutableSha256=approved_source_sha,bundleIdentifier=candidate['bundleIdentifier'],calibrationSourceExecutableSha256=layout['sourceExecutableSha256'])
        for path in [canonical_app/'Contents/MacOS/Vixen',build_inputs,ROOT/'scripts/prepare_private_test_app.py']:
            m['frozenHashes'][str(path)]=sha(path)
    save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),kind=kind,steps=m['steps'],launched=False)))

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--kind',choices=['color','gradient']);p.add_argument('--helper',type=Path);p.add_argument('--helper-preparation',type=Path);p.add_argument('--app',type=Path);p.add_argument('--approved-source-sha');p.add_argument('--canonical-app',type=Path);p.add_argument('--build-inputs',type=Path);p.add_argument('--inspector',type=Path);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.kind,a.helper.resolve(),a.helper_preparation.resolve(),a.app.resolve() if a.app else None,a.approved_source_sha,a.canonical_app.resolve() if a.canonical_app else None,a.build_inputs.resolve() if a.build_inputs else None,a.inspector.resolve() if a.inspector else None)
    else:raise SystemExit(run(out,json.loads((out/'prepared.json').read_text())))
