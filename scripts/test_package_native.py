#!/usr/bin/env python3
"""Prepare/run private normal-app package panels. Root owns all native launches."""
import argparse,re,hashlib,json,os,plistlib,signal,subprocess,time,unicodedata
from pathlib import Path
import xml.etree.ElementTree as ET
import package_native_contract as contract
import prepare_package_native_fixture as fixture
from prepare_private_test_app import code_digest
from check_mapper_file_panels import validate_private_profile
ROOT=Path(__file__).resolve().parents[1];BASE=fixture.SOURCE;SAMPLE=BASE/'Sequence/Panel Source Café.tim'
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
def tree(path):return {str(p.relative_to(path)):sha(p) for p in sorted(path.rglob('*')) if p.is_file()}
def save(path,value):
    temp=Path(str(path)+'.tmp');temp.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n');temp.replace(path)
def owner_preferences():
    paths=[Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist'];folder=Path.home()/'Library/Application Support/Vixen'
    if folder.exists():paths+=sorted(p for p in folder.rglob('*') if p.is_file())
    return {str(p):sha(p) if p.is_file() else None for p in paths}
def backup_path(path):return backup_name(path.stat().st_mtime_ns)
def backup_name(mtime_ns):return 'SystemData/auto_backup/ModuleStore_'+time.strftime('%m%d%Y_%I%M%S',time.localtime(mtime_ns/1_000_000_000))+'.xml'
def store_snapshot(path):
    before=path.stat();digest=sha(path);after=path.stat()
    assert not path.is_symlink() and (before.st_ino,before.st_size,before.st_mtime_ns)==(after.st_ino,after.st_size,after.st_mtime_ns),'ModuleStore changed during checkpoint'
    return dict(sha256=digest,mtimeNs=after.st_mtime_ns,backupPath=backup_name(after.st_mtime_ns))
def accepted_backups(profile,versions):
    # Exactly three package closes save ModuleStore. Each backs up its previous
    # version; same-second names replace earlier content. At most three names
    # exist, so the production >3 retention branch cannot apply to this fixture.
    assert 1<=len(versions)<=3
    expected={}
    for version in versions:
        assert set(version)=={'sha256','mtimeNs','backupPath'} and isinstance(version['mtimeNs'],int)
        assert version['backupPath']==backup_name(version['mtimeNs'])
        expected[version['backupPath']]=version['sha256']
    folder=profile/'SystemData/auto_backup'
    assert folder.is_dir() and not any(p.is_symlink() for p in folder.rglob('*'))
    actual={str(p.relative_to(profile)):sha(p) for p in folder.rglob('*') if p.is_file()}
    assert actual==expected,'ModuleStore backup set or bytes differ from exact preceding saved versions'
    return expected
RELEASE='945941c664fc2796c32c7783b2eb8137f502dab0e6587a2ba0680746f2700865'
CHECKPOINTS=['exported','cancelled-import','imported']
def prepare(out,app,helper,approved_source=RELEASE,canonical_app=None,build_inputs=None):
    assert re.fullmatch("[0-9a-f]{64}",approved_source),"Invalid approved source SHA"
    if approved_source!=RELEASE:assert canonical_app is not None and build_inputs is not None,"New release requires explicit canonical app and build inputs"
    clonepath=app.parent/'private-app.json';clone=json.loads(clonepath.read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged'] and clone['sourceExecutableSha256']==approved_source
    canonical_app=(canonical_app or Path(clone['sourceApp'])).resolve()
    assert canonical_app==Path(clone['sourceApp']).resolve() and sha(canonical_app/'Contents/MacOS/Vixen')==approved_source
    build_inputs=(build_inputs or ROOT/'target/vixen2-drop-integration/build-inputs.json').resolve()
    source_pins=json.loads(build_inputs.read_text())['sources'];assert len(source_pins)==811 and all(sha(ROOT/p)==h for p,h in source_pins.items())
    assert sha(exe)==clone['executableSha256'] and code_digest(exe)==clone['executableCodeDigest']
    m=fixture.prepare(out)
    m.update(app=str(app),executable=str(exe),helper=str(helper),bundleIdentifier=clone['bundleIdentifier'],sourceApp=clone['sourceApp'],sourceExecutableSha256=approved_source,approvedSourceExecutableSha256=approved_source,approvedCanonicalApp=str(canonical_app),approvedBuildInputs=str(build_inputs),privateExecutableSha256=sha(exe),codeDigest=clone['executableCodeDigest'],sourceProfileHashes=tree(BASE),sourceTimSha256=sha(SAMPLE),profileHashes=tree(out/'profile'),openFile=m['sourceSequence'],environment=m['requiredEnvironment'],arguments=m['requiredArguments'],inputSeconds=240,cleanupSeconds=255,outerSeconds=280,checkpoints=CHECKPOINTS,initialModuleStore=store_snapshot(out/'profile/SystemData/ModuleStore.xml'))
    m['scope']='Normal private clone of release '+approved_source+'. Real native package source/output/package/map/import-collision file panels, Cancel, exact ZIP/TIM/settings readback and normal Quit. No UI_TEST/internal model/file-panel substitution; controllers/web disabled and original files preserved.'
    for p in [Path(__file__).resolve(),ROOT/'scripts/foreground_package_native.swift',ROOT/'scripts/check_mapper_file_panels.py',ROOT/'scripts/prepare_private_test_app.py',build_inputs,clonepath,exe,app/'Contents/Info.plist',helper,Path(m['sourceSequence']),Path(m['mapFile'])]:m['pins'][str(p)]=sha(p)
    m['pins'][str(build_inputs)]=sha(build_inputs);m['pins'][str(canonical_app/'Contents/MacOS/Vixen')]=sha(canonical_app/'Contents/MacOS/Vixen')
    save(out/'prepared.json',m);validate_prepared(out,json.loads((out/'prepared.json').read_text()));return m

def validate_prepared(out,m):
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists() and not list((out/'runtime').iterdir()) and not list((out/'settings').iterdir())
    assert [m[k] for k in ['inputSeconds','cleanupSeconds','outerSeconds']]==[240,255,280] and m['checkpoints']==CHECKPOINTS
    assert all(sha(p)==v for p,v in m['pins'].items()) and tree(BASE)==m['sourceProfileHashes'] and sha(SAMPLE)==m['sourceTimSha256']
    assert tree(out/'profile')==m['profileHashes']
    approved=m['approvedSourceExecutableSha256'];assert re.fullmatch('[0-9a-f]{64}',approved)
    assert m['sourceExecutableSha256']==approved and Path(m['sourceApp']).resolve()==Path(m['approvedCanonicalApp'])
    assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==approved and sha(m['executable'])==m['privateExecutableSha256']
    inputs=json.loads(Path(m['approvedBuildInputs']).read_text())['sources'];assert len(inputs)==811 and all(sha(ROOT/p)==h for p,h in inputs.items())
    assert code_digest(Path(m['executable']))==m['codeDigest'] and plistlib.loads((Path(m['app'])/'Contents/Info.plist').read_bytes())['CFBundleIdentifier']==m['bundleIdentifier']
    assert m['environment']==dict(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1',TMPDIR=str(out/'runtime')+'/') and m['arguments']==['no_controllers','no_execution'] and m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    assert not Path(m['packageOutput']).exists() and not Path(m['importedSequence']).exists();contract.require_source(m)
    assert store_snapshot(out/'profile/SystemData/ModuleStore.xml')==m['initialModuleStore']

def checkpoint_state(out,m,name,completed):
    assert json.loads((out/('checkpoint-'+name+'.request')).read_text())==dict(checkpoint=name)
    profile=out/'profile';versions=[m['initialModuleStore']]
    if name=='exported':result=contract.export_state(m)
    else:
        exported=completed.get('exported');assert exported is not None,'Import checkpoint before export'
        result=(contract.cancelled_import_state if name=='cancelled-import' else contract.imported_state)(m,exported)
        versions.append(exported['moduleStore'])
        if name=='imported':versions.append(completed['cancelled-import']['moduleStore'])
    settings=contract.saved_settings(m)
    if name!='exported':assert settings==completed['exported']['settings'],'Import close changed export settings or unrelated modules'
    snapshot=store_snapshot(profile/'SystemData/ModuleStore.xml')
    assert snapshot['sha256']==settings['moduleStoreSha256']
    return dict(checkpoint=name,**result,settings=settings,moduleStore=snapshot,backups=accepted_backups(profile,versions))

def service_checkpoints(out,m,completed,pending,deadline):
    for index,name in enumerate(CHECKPOINTS):
        request=out/('checkpoint-'+name+'.request');response=out/('checkpoint-'+name+'.json')
        if not request.exists() or name in completed:continue
        assert request.is_file() and not request.is_symlink() and request.stat().st_size<4096
        assert list(completed)==CHECKPOINTS[:index] and json.loads(request.read_text())==dict(checkpoint=name),'Unexpected/out-of-order checkpoint'
        until=pending.setdefault(name,min(deadline-8,time.monotonic()+2.5))
        try:state=checkpoint_state(out,m,name,completed)
        except (AssertionError,OSError,ValueError,ET.ParseError) as error:
            if time.monotonic()>=until:save(response,dict(passed=False,checkpoint=name,error=repr(error)));raise
            continue
        assert time.monotonic()<until;save(response,state);completed[name]=state

def accept(out,m,fg,completed):
    assert fg.get("pendingKeyUpAcknowledgements")==[],"Posted keyUPs remain unacknowledged"
    assert fg.get('passed') is True and fg.get('workflow')=='package-file-panels' and fg.get('normalQuitMenu') is True
    for k in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','inputCompleted']:assert fg.get(k) is True,k
    assert not fg['cleanupErrors'] and fg['physicalAfterRestoration']==dict(buttons=[],keys=[],modifiers=0)
    assert fg['mouseDowns']==fg['mouseUps']==0 and fg['keyDowns']==fg['keyUps']==30
    events=fg['events'];assert len(events)==60 and all(set(e)=={'key','down'} for e in events)
    one=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False)]]
    tab=[dict(key=48,down=True),dict(key=48,down=False)];assert events==one+one+tab+one+one+one+one+one+tab
    assert list(completed)==CHECKPOINTS
    for name in CHECKPOINTS:assert json.loads((out/('checkpoint-'+name+'.json')).read_text())==completed[name]
    final=contract.imported_state(m,completed['exported']);assert final=={k:v for k,v in completed['imported'].items() if k not in ['checkpoint','settings','moduleStore','backups']}
    settings=contract.saved_settings(m);assert settings==completed['exported']['settings']
    obs=fg['observations']
    assert [o['savedPath'] for o in obs if 'savedPath' in o]==[m['packageOutput'],m['importedSequence']]
    assert [o['selectedPath'] for o in obs if 'selectedPath' in o]==[m['packageOutput'],m['mapFile'],m['packageOutput'],m['mapFile']]
    assert all(unicodedata.normalize('NFC',o['observedSelectedPath'])==unicodedata.normalize('NFC',o['selectedPath']) for o in obs if 'selectedPath' in o)
    assert [o['stage'] for o in obs if o.get('panelCancelled')]==['package-save-cancel','package-open-cancel','map-open-cancel']
    for stage in ['source-chooser-cancelled','source-chooser-accepted']:assert sum(o['stage']==stage for o in obs)==1
    status=[(o['stage'],o['status'],o['count']) for o in obs if 'count' in o]
    assert status==[('exported','Package Export Complete','1 sequence(s) completed; 0 skipped.'),('cancelled-import','Package Import Complete','0 sequence(s) completed; 1 skipped.'),('imported','Package Import Complete','1 sequence(s) completed; 0 skipped.')]
    profile=out/'profile';current=tree(profile);initial=m['profileHashes'];imported=str(Path(m['importedSequence']).relative_to(profile))
    backups=accepted_backups(profile,[m['initialModuleStore'],completed['exported']['moduleStore'],completed['cancelled-import']['moduleStore']])
    assert backups==completed['imported']['backups'] and store_snapshot(profile/'SystemData/ModuleStore.xml')==completed['imported']['moduleStore']
    assert not list((out/'settings').iterdir()),'Normal package-only workflow unexpectedly wrote Rust settings'
    assert not any(p.is_symlink() for p in profile.rglob('*')) and set(current)-set(initial)<=set(backups)|{imported,'VixenApplicationData.xml','logs/Application.Info.log'}
    assert set(initial)<=set(current) and all(current[p]==h for p,h in initial.items() if p!='SystemData/ModuleStore.xml')
    assert all(current.get(path)==digest for path,digest in backups.items()) and imported in current and 'VixenApplicationData.xml' in current
    appdata=validate_private_profile(profile,{k:v for k,v in current.items() if k!='VixenApplicationData.xml'});assert appdata['accepted']
    assert not list(Path(m['packageOutput']).parent.glob('*.part-*'))
    return dict(imported=final,settings=settings,checkpointCount=3,keyEvents=60,pointerEvents=0,privateProfileValidation=appdata,moduleStoreBackups=backups,settingsDirectoryEmpty=True)

def terminate(child,deadline):
    if child is None or child.poll() is not None:return
    for sig in [signal.SIGTERM,signal.SIGKILL]:
        try:os.killpg(child.pid,sig)
        except ProcessLookupError:return
        try:child.wait(timeout=min(2,max(.05,deadline-time.monotonic())));return
        except subprocess.TimeoutExpired:pass

def run(out):
    began=time.monotonic();deadline=began+280;m=json.loads((out/'prepared.json').read_text());app=driver=None;before=None;completed={};pending={}
    result=dict(passed=False,scope=m['scope'],sourceExecutableSha256=m['sourceExecutableSha256'],privateExecutableSha256=m['privateExecutableSha256'],cleanupErrors=[])
    try:
        before=owner_preferences();validate_prepared(out,m)
        names=subprocess.check_output(['/bin/ps','-axo','pid=,comm='],text=True,timeout=3)
        assert not any(Path(x.strip().split(None,1)[1]).name=='Vixen' or Path(x.strip().split(None,1)[1]).name.startswith('foreground-') for x in names.splitlines() if len(x.strip().split(None,1))==2),'Another native app/helper is active'
        desktop=json.loads(subprocess.check_output([m['helper'],'--desktop'],text=True,timeout=3));save(out/'desktop-before.json',desktop)
        assert desktop['accessibility'] and desktop['eventPosting'] and desktop['noPhysicalInputHeld'] and len(desktop['pointer'])==2
        assert not desktop['application']['bundleIdentifier'].startswith('com.vixenlights.') and 'securityagent' not in desktop['application']['bundleIdentifier'].lower()
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(m['environment'],TMPDIR=str(out/'runtime')+'/')
        assert 'VIXEN_UI_TEST' not in env
        with (out/'process.log').open('w') as log,(out/'driver.log').open('w') as dlog:
            assert deadline-time.monotonic()>270,'Complete launch and cleanup budget unavailable'
            app=subprocess.Popen([m['executable'],*m['arguments']],env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            cfg=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],workflow='package-file-panels',profileExportDirectory=str(out/'profile/Export')+'/',**{k:m[k] for k in ['sourceSequence','packageOutput','mapFile','importedSequence']})
            save(out/'driver-config.json',cfg);save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable']))
            assert deadline-time.monotonic()>265,'Full driver cleanup reserve unavailable'
            driver=subprocess.Popen([m['helper'],str(out/'driver-config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable'],driverPID=driver.pid,driverProcessGroup=driver.pid,driverExecutable=m['helper']))
            end=min(deadline-12,time.monotonic()+258)
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
        result.update(foreground=fg,checkpoints=completed,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,ownerPreferenceHashesBefore=before,ownerPreferencesUnchanged=checked('preferences',lambda:before is not None and owner_preferences()==before),sourceProfileUnchanged=checked('sourceprofile',lambda:tree(BASE)==m['sourceProfileHashes']),sourceTimUnchanged=checked('sourcetim',lambda:sha(SAMPLE)==m['sourceTimSha256']),pinsUnchanged=checked('pins',lambda:all(sha(p)==v for p,v in m['pins'].items())),inputsUnchanged=checked('input',lambda:sha(m['openFile'])==m['pins'][m['openFile']]),elapsedSeconds=time.monotonic()-began)
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['passed']=result['passed'] and not result['cleanupErrors'] and all(result[k] for k in ['processExited','driverExited','ownerPreferencesUnchanged','sourceProfileUnchanged','sourceTimUnchanged','pinsUnchanged','inputsUnchanged','noInputHeld','restoredPrevious']) and result['elapsedSeconds']<280
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','elapsedSeconds','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path,default=ROOT/'target/private-app-vixen2-drop-location/Vixen.app');p.add_argument('--helper',type=Path);p.add_argument('--approved-source-sha',default=RELEASE);p.add_argument('--canonical-app',type=Path);p.add_argument('--build-inputs',type=Path);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:
        m=prepare(out,a.app.resolve(),a.helper.resolve(),a.approved_source_sha,a.canonical_app.resolve() if a.canonical_app else None,a.build_inputs.resolve() if a.build_inputs else None);print(json.dumps(dict(prepared=str(out/'prepared.json'),launched=False,source=m['sourceExecutableSha256'])))
    else:raise SystemExit(0 if run(out)['passed'] else 1)
