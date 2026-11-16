#!/usr/bin/env python3
"""Prepare/run one private normal sequence Export Cancel and Unicode FSEQ save."""
import argparse,re,json,os,plistlib,shutil,signal,subprocess,time,unicodedata
from pathlib import Path
import xml.etree.ElementTree as ET
import test_vixen2_physical as common
import prepare_export_file_panel_fixture as fixture
import export_file_panel_contract as contract
from prepare_private_test_app import code_digest
ROOT=Path(__file__).resolve().parents[1]
RELEASE='945941c664fc2796c32c7783b2eb8137f502dab0e6587a2ba0680746f2700865'
CHECKPOINTS=['cancelled','exported','default-exported']
sha=common.sha;tree=common.tree;save=common.save;owner_preferences=common.owner_preferences
REFERENCE=ROOT/'target/collection-picker-settings-reference.json'
nfc=lambda value:unicodedata.normalize('NFC',value)

def xml_root(path):
    assert path.is_file() and not path.is_symlink() and path.stat().st_size<65536
    text=path.read_text(encoding='utf-8-sig');body=text.lstrip()
    if body.startswith('<?xml '):
        end=body.find('?>');assert end>=0;body=body[end+2:]
    assert '<!' not in body and '<?' not in body
    return ET.fromstring(text)

def prepare(out,app,helper,approved_source=RELEASE,canonical_app=None,build_inputs=None):
    assert re.fullmatch("[0-9a-f]{64}",approved_source),"Invalid approved source SHA"
    if approved_source!=RELEASE:assert canonical_app is not None and build_inputs is not None,"New release requires explicit canonical app and build inputs"
    assert out.is_relative_to(ROOT/'target') and not out.exists();out.mkdir(parents=True)
    manifest=app.parent/'private-app.json';clone=json.loads(manifest.read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged'] and clone['sourceExecutableSha256']==approved_source
    canonical_app=(canonical_app or Path(clone['sourceApp'])).resolve()
    assert canonical_app==Path(clone['sourceApp']).resolve() and sha(canonical_app/'Contents/MacOS/Vixen')==approved_source
    build_inputs=(build_inputs or ROOT/'target/vixen2-drop-integration/build-inputs.json').resolve()
    source_pins=json.loads(build_inputs.read_text())['sources'];assert len(source_pins)==808 and all(sha(ROOT/p)==h for p,h in source_pins.items())
    assert sha(exe)==clone['executableSha256'] and code_digest(exe)==clone['executableCodeDigest']
    assert plistlib.loads((app/'Contents/Info.plist').read_bytes())['CFBundleIdentifier']==clone['bundleIdentifier']
    f=fixture.prepare(out/'fixture');fm=out/'fixture/prepared.json';reference=json.loads(REFERENCE.read_text())
    files=[Path(__file__).resolve(),ROOT/'scripts/foreground_sequence_export.swift',ROOT/'scripts/test_sequence_export_controls.py',ROOT/'scripts/test_vixen2_physical.py',ROOT/'scripts/prepare_private_test_app.py',helper,manifest,exe,app/'Contents/Info.plist',Path(clone['sourceApp'])/'Contents/MacOS/Vixen',fm,REFERENCE,Path(reference['nativeAcceptance'])]+[Path(p) for p in list(f['pins'])+list(reference['files'])+list(reference['sourcePins'])]
    m=dict(preparedOnly=True,launched=False,workflow='sequence-export',app=str(app),executable=str(exe),helper=str(helper),bundleIdentifier=clone['bundleIdentifier'],sourceApp=clone['sourceApp'],sourceExecutableSha256=approved_source,approvedSourceExecutableSha256=approved_source,approvedCanonicalApp=str(canonical_app),approvedBuildInputs=str(build_inputs),privateExecutableSha256=sha(exe),codeDigest=clone['executableCodeDigest'],fixtureManifest=str(fm),sourceInputs=f['sourceInputs'],profile=f['profile'],profileHashes=f['profileHashes'],settings=str(out/'fixture/settings'),openFile=f['openFile'],openFileSha256=f['openFileSha256'],sequenceTitle=f['sequenceTitle']+' – '+Path(f['profile']).name,exportFile=f['exportFile'],defaultExportFile=f['defaultExportFile'],cancelSentinel=f['cancelSentinel'],environment=f['requiredEnvironment'],arguments=f['requiredArguments'],checkpoints=CHECKPOINTS,inputSeconds=90,cleanupSeconds=105,outerSeconds=125,pins={str(p):sha(p) for p in files},scope=f['scope']+' Native save filename/directory readback selects existing sentinel then Cancel; second actual save creates the Unicode FSEQ; third saves the untouched offered default into private Export. All three offered names require exactly one suffix.14 key pairs/zero pointer events. Exact previous desktop restored.')
    m['pins'][str(build_inputs)]=sha(build_inputs);m['pins'][str(canonical_app/'Contents/MacOS/Vixen')]=sha(canonical_app/'Contents/MacOS/Vixen')
    save(out/'prepared.json',m);validate_prepared(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),launched=False)))

def validate_prepared(out,m):
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists() and not (out/'runtime').exists()
    assert m['workflow']=='sequence-export' and [m[k] for k in ['inputSeconds','cleanupSeconds','outerSeconds']]==[90,105,125] and m['checkpoints']==CHECKPOINTS
    assert all(sha(p)==h for p,h in m['pins'].items()) and all(sha(p)==h for p,h in m['sourceInputs'].items())
    assert tree(Path(m['profile']))==m['profileHashes'] and not list(Path(m['settings']).iterdir())
    approved=m['approvedSourceExecutableSha256'];assert re.fullmatch('[0-9a-f]{64}',approved)
    assert m['sourceExecutableSha256']==approved and Path(m['sourceApp']).resolve()==Path(m['approvedCanonicalApp'])
    assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==approved and sha(m['executable'])==m['privateExecutableSha256']
    inputs=json.loads(Path(m['approvedBuildInputs']).read_text())['sources'];assert len(inputs)==808 and all(sha(ROOT/p)==h for p,h in inputs.items())
    assert m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    f=json.loads(Path(m['fixtureManifest']).read_text());assert m['environment']==f['requiredEnvironment'] and m['arguments']==['no_controllers','no_execution']
    assert m['sequenceTitle']==f['sequenceTitle']+' – '+Path(f['profile']).name
    common.assert_services(Path(m['profile']));contract.verify(Path(m['fixtureManifest']),'cancelled')

def checkpoint_state(out,m,name):
    assert name in CHECKPOINTS and json.loads((out/('checkpoint-'+name+'.request')).read_text())==dict(checkpoint=name)
    proof=contract.verify(Path(m['fixtureManifest']),name)
    current=tree(Path(m['profile']));assert all(current.get(p)==h for p,h in m['profileHashes'].items()),'Existing private profile changed'
    common.assert_services(Path(m['profile']))
    return dict(checkpoint=name,**proof)

def service_checkpoints(out,m,completed,pending,deadline):
    for name in CHECKPOINTS:
        request=out/('checkpoint-'+name+'.request');response=out/('checkpoint-'+name+'.json')
        if not request.exists() or name in completed:continue
        assert request.is_file() and not request.is_symlink() and request.stat().st_size<4096 and json.loads(request.read_text())==dict(checkpoint=name),'Unexpected checkpoint request'
        if name=='exported':assert 'cancelled' in completed,'Export checkpoint before Cancel proof'
        if name=='default-exported':assert 'exported' in completed,'Default export checkpoint before explicit export proof'
        until=pending.setdefault(name,min(deadline-8,time.monotonic()+2.5))
        try:
            state=checkpoint_state(out,m,name)
            if name=='default-exported':assert state['fseq']['sha256']==completed['exported']['fseq']['sha256'],'Explicit export changed during default save'
        except (AssertionError,OSError,ValueError,ET.ParseError) as error:
            if time.monotonic()>=until:
                save(response,dict(passed=False,checkpoint=name,error=repr(error)));raise AssertionError('Persisted '+name+' checkpoint failed: '+repr(error))
            continue
        assert time.monotonic()<until,'Checkpoint deadline expired';save(response,state);completed[name]=state

def accept_observations(fg,m):
    assert fg.get("pendingKeyUpAcknowledgements")==[],"Posted keyUPs remain unacknowledged"
    assert fg.get('passed') is True and fg.get('sequenceExportWorkflow') is True and fg.get('normalQuitMenu') is True
    for key in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','inputCompleted']:assert fg.get(key) is True,key
    assert not fg['cleanupErrors'] and fg['physicalAfterRestoration']==dict(buttons=[],keys=[],modifiers=0)
    assert fg['mouseDowns']==fg['mouseUps']==0 and fg['keyDowns']==fg['keyUps']==14
    goto=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False)]];tab=[dict(key=48,down=True),dict(key=48,down=False)]
    assert fg['events']==goto+goto+tab+goto+tab,'Unexpected native key sequence or pointer events'
    obs=fg['observations']
    for stage in ['initial-export-idle','cancelled-export-idle','completed-export-idle','default-completed-export-idle']:
        rows=[o for o in obs if o.get('stage')==stage];assert rows==[dict(stage=stage,startEnabled=True,stopEnabled=False,closeEnabled=True)]
    for stage in ['cancel','save','default']:
        rows=[o for o in obs if o.get('stage')==stage+'-offered-default'];assert len(rows)==1 and set(rows[0])=={'stage','filename','expected','directories'}
        assert nfc(rows[0]['filename'])==nfc(rows[0]['expected'])==Path(m['defaultExportFile']).name and rows[0]['directories']==['Export']
    for stage,path in [('cancel',m['cancelSentinel']),('save',m['exportFile'])]:
        rows=[o for o in obs if o.get('stage')==stage+'-destination'];assert len(rows)==1 and set(rows[0])=={'stage','directory','filename'}
        assert nfc(rows[0]['directory'])==Path(path).parent.name and nfc(rows[0]['filename'])==Path(path).name
    for stage in ['start-cancelled-export','cancel-export-save','start-successful-export','confirm-export-save','start-default-export','confirm-default-export-save','close-export','quit-menu']:
        rows=[o for o in obs if o.get('stage')==stage];assert len(rows)==1 and rows[0]['action']=='AXPress' and rows[0]['status'] in [0,-25204]
    return dict(pointerEvents=0,keyEvents=28,nativeSavePanelCancels=1,nativeSavePanelSaves=2,untouchedDefaultFilenameVerified=True)

def accept_profile(m):
    profile=Path(m['profile']);settings=Path(m['settings']);assert not any(p.is_symlink() for folder in [profile,settings] for p in folder.rglob('*'))
    current=tree(profile);assert all(current.get(p)==h for p,h in m['profileHashes'].items())
    default_relative=str(Path(m['defaultExportFile']).relative_to(profile));extras=set(current)-set(m['profileHashes'])
    contract.exact_names(extras,{'logs/Application.Info.log','VixenApplicationData.xml',default_relative} if 'logs/Application.Info.log' in extras else {'VixenApplicationData.xml',default_relative})
    contract.verify(Path(m['fixtureManifest']),'default-exported')
    data=xml_root(profile/'VixenApplicationData.xml');expected=ET.Element('VixenApplicationData');ET.SubElement(expected,'DataFormatVersion').text='3';recent=ET.SubElement(expected,'RecentSequences');ET.SubElement(recent,'SequenceFile').text=m['openFile'];ET.SubElement(expected,'FilterShapePositions');ET.SubElement(expected,'FilterSetupFormHighQualityRendering',value='false')
    files=data.findall('./RecentSequences/SequenceFile');assert len(files)==1 and files[0].text is not None
    assert nfc(files[0].text)==nfc(m['openFile']) and Path(files[0].text).samefile(m['openFile']),'Recent sequence path differs'
    files[0].text=m['openFile']
    assert common.canonical(data)==common.canonical(expected),'Unexpected application data'
    if 'logs/Application.Info.log' in current:
        log=profile/'logs/Application.Info.log';assert log.stat().st_size<4_000_000;log.read_text()
    ref=json.loads(REFERENCE.read_text());refs={Path(p).name:Path(p) for p in ref['files']};assert set(tree(settings))==set(refs),'Unexpected settings output'
    for name,path in refs.items():
        assert sha(path)==ref['files'][str(path)]
        if name.endswith('.xml'):assert common.canonical(xml_root(settings/name))==common.canonical(xml_root(path))
        else:assert json.loads((settings/name).read_text())==json.loads(path.read_text())
    return dict(passed=True,profileFiles=current,settingsFiles=tree(settings))

def accept(out,m,fg,completed):
    counts=accept_observations(fg,m);assert set(completed)==set(CHECKPOINTS)
    for name in CHECKPOINTS:assert json.loads((out/('checkpoint-'+name+'.json')).read_text())==completed[name]
    final=contract.verify(Path(m['fixtureManifest']),'default-exported');assert final=={k:v for k,v in completed['default-exported'].items() if k!='checkpoint'}
    assert final['fseq']['sha256']==completed['exported']['fseq']['sha256'],'Explicit export changed during default save'
    return dict(**counts,finalExport=final,checkpointCount=3,privateProfileValidation=accept_profile(m))

def terminate(child,deadline):
    if child is None or child.poll() is not None:return
    for sig in [signal.SIGTERM,signal.SIGKILL]:
        try:os.killpg(child.pid,sig)
        except ProcessLookupError:return
        try:child.wait(timeout=min(2,max(.05,deadline-time.monotonic())));return
        except subprocess.TimeoutExpired:pass

def run(out):
    began=time.monotonic();deadline=began+125;m=json.loads((out/'prepared.json').read_text());app=driver=None;before=None;completed={};pending={}
    result=dict(passed=False,scope=m['scope'],sourceExecutableSha256=m['sourceExecutableSha256'],privateExecutableSha256=m['privateExecutableSha256'],cleanupErrors=[])
    try:
        before=owner_preferences();validate_prepared(out,m)
        names=subprocess.check_output(['/bin/ps','-axo','pid=,comm='],text=True,timeout=3)
        assert not any(Path(x.strip().split(None,1)[1]).name=='Vixen' or Path(x.strip().split(None,1)[1]).name.startswith('foreground-') for x in names.splitlines() if len(x.strip().split(None,1))==2),'Another native app/helper is active'
        desktop=json.loads(subprocess.check_output([m['helper'],'--desktop'],text=True,timeout=3));save(out/'desktop-before.json',desktop)
        assert desktop['accessibility'] and desktop['eventPosting'] and desktop['noPhysicalInputHeld'] and len(desktop['pointer'])==2
        assert not desktop['application']['bundleIdentifier'].startswith('com.vixenlights.') and 'securityagent' not in desktop['application']['bundleIdentifier'].lower()
        (out/'runtime').mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(m['environment'])
        assert 'VIXEN_UI_TEST' not in env
        with (out/'process.log').open('w') as log,(out/'driver.log').open('w') as dlog:
            assert deadline-time.monotonic()>115,'Complete launch and cleanup budget unavailable'
            app=subprocess.Popen([m['executable'],*m['arguments']],env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            cfg=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],openFile=m['openFile'],sequenceTitle=m['sequenceTitle'],workflow='sequence-export',exportFile=m['exportFile'],defaultExportFile=m['defaultExportFile'],cancelSentinel=m['cancelSentinel'])
            save(out/'driver-config.json',cfg);save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable']))
            assert deadline-time.monotonic()>110,'Full driver cleanup reserve unavailable'
            driver=subprocess.Popen([m['helper'],str(out/'driver-config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable'],driverPID=driver.pid,driverProcessGroup=driver.pid,driverExecutable=m['helper']))
            end=min(deadline-12,time.monotonic()+108)
            while app.poll() is None and time.monotonic()<end:
                if driver.poll() not in [None,0]:raise RuntimeError('Native workflow stopped; inspect foreground report')
                service_checkpoints(out,m,completed,pending,deadline)
                time.sleep(.05)
            assert app.poll()==0,'Private app did not finish normal Quit'
            driver.wait(timeout=min(4,max(.1,deadline-time.monotonic()-6)));assert driver.returncode==0
        result.update(appExitCode=app.returncode,driverExitCode=driver.returncode)
        fg=json.loads((out/'foreground.json').read_text());result.update(accept(out,m,fg,completed));result['passed']=True
    except BaseException as error:result['error']=repr(error)
    finally:
        if driver is not None and driver.poll() is None:
            try:save(out/'abort-request.json',dict(abort=True))
            except BaseException as e:result['cleanupErrors'].append('abort: '+repr(e))
            try:driver.wait(timeout=min(4,max(.1,deadline-time.monotonic()-5)))
            except subprocess.TimeoutExpired:pass
            except BaseException as e:result['cleanupErrors'].append('cooperative stop: '+repr(e))
        for child in [driver,app]:
            try:terminate(child,deadline)
            except BaseException as e:result['cleanupErrors'].append('owned cleanup: '+repr(e))
        def checked(name,operation):
            try:return operation()
            except BaseException as e:result['cleanupErrors'].append(name+': '+repr(e));return False
        fg=checked('foreground',lambda:json.loads((out/'foreground.json').read_text())) if (out/'foreground.json').exists() else {}
        result.update(foreground=fg,checkpoints=completed,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,ownerPreferenceHashesBefore=before,ownerPreferencesUnchanged=checked('preferences',lambda:before is not None and owner_preferences()==before),sourceInputsUnchanged=checked('sourceinputs',lambda:all(sha(p)==h for p,h in m['sourceInputs'].items())),pinsUnchanged=checked('pins',lambda:all(sha(p)==v for p,v in m['pins'].items())),inputsUnchanged=checked('input',lambda:sha(m['openFile'])==m['openFileSha256']),elapsedSeconds=time.monotonic()-began)
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['passed']=result['passed'] and not result['cleanupErrors'] and all(result[k] for k in ['processExited','driverExited','ownerPreferencesUnchanged','sourceInputsUnchanged','pinsUnchanged','inputsUnchanged','noInputHeld','restoredPrevious']) and result['elapsedSeconds']<125
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','elapsedSeconds','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path,default=ROOT/'target/private-app-vixen2-drop-location/Vixen.app');p.add_argument('--helper',type=Path,default=ROOT/'target/foreground-sequence-export-v1');p.add_argument('--approved-source-sha',default=RELEASE);p.add_argument('--canonical-app',type=Path);p.add_argument('--build-inputs',type=Path);mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve(),a.approved_source_sha,a.canonical_app.resolve() if a.canonical_app else None,a.build_inputs.resolve() if a.build_inputs else None)
    else:raise SystemExit(0 if run(out)['passed'] else 1)
