#!/usr/bin/env python3
"""Prepare/run the private normal-app Vixen2 physical mapping/color workflow.

All app interaction belongs to the independently reviewed Swift helper. Python
only prepares isolated files, starts/stops owned children, and validates persisted
XML at two explicit checkpoints. No VIXEN_UI_TEST, model/drop/color hooks.
"""
import argparse,collections,hashlib,json,math,os,re,shutil,signal,subprocess,sys,time,uuid
from pathlib import Path
import xml.etree.ElementTree as ET
import test_vixen2_physical as discovery
import vixen2_physical_contract as contract
from check_mapper_file_panels import validate_private_profile
ROOT=Path(__file__).resolve().parents[1]
BASE=discovery.BASE;SAMPLE=discovery.SAMPLE;RELEASE=discovery.RELEASE;MAP_NAME=discovery.MAP_NAME
sha=discovery.sha;tree=discovery.tree;save=discovery.save;owner_preferences=discovery.owner_preferences
CHECKPOINTS=['accepted','cancelled']

def prepare(out,app,helper,approved_source=RELEASE,canonical_app=None,build_inputs=None):
    assert re.fullmatch('[0-9a-f]{64}',approved_source),'Explicit approved source SHA required'
    clone=json.loads((app.parent/'private-app.json').read_text())
    if approved_source!=RELEASE:assert canonical_app is not None and build_inputs is not None,'New release requires explicit canonical app and build inputs'
    canonical_app=canonical_app or Path(clone['sourceApp'])
    assert canonical_app.resolve()==Path(clone['sourceApp']).resolve(),'Clone source differs from approved canonical app'
    assert sha(canonical_app/'Contents/MacOS/Vixen')==approved_source and clone['sourceExecutableSha256']==approved_source
    # Reuse proven fixture/isolation mechanics without modifying the pinned
    # discovery script or its default97498 contract. The temporary value is
    # confined to this preparation process and restored even on failure.
    original_release=discovery.RELEASE
    try:
        discovery.RELEASE=approved_source;discovery.prepare(out,app,helper)
    finally:discovery.RELEASE=original_release
    m=json.loads((out/'prepared.json').read_text());system=out/'profile/SystemData/SystemConfig.xml';xml=ET.parse(system);nodes=xml.getroot().find('Nodes');leaves=list(nodes)
    assert len(leaves)==4 and [n.attrib['id'] for n in leaves]==m['destinationIDs']
    group_id=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-vixen2-physical-group'))
    group=ET.SubElement(nodes,'Node',id=group_id,name='V2 Fixture Group')
    for leaf in leaves[2:]:nodes.remove(leaf);group.append(leaf)
    xml.write(system,encoding='utf-8',xml_declaration=True)
    # Preserve namespace declarations referenced only from xsi:type values in
    # the source curve library. ElementTree's generic rewrite drops those QName
    # bindings, so insert only the separately generated WebServer module into
    # the byte-identical original ModuleData instead of rewriting old modules.
    store=out/'profile/SystemData/ModuleStore.xml'
    generated=ET.parse(store).getroot();web=[x for x in generated.iter('Module') if x.get('assemblyName')=='Module.App.WebServer'];assert len(web)==1
    base_data=(BASE/'SystemData/ModuleStore.xml').read_bytes()
    assert base_data.count(b'</ModuleData>')==1 and not any(x.get('assemblyName')=='Module.App.WebServer' for x in ET.fromstring(base_data).iter('Module'))
    fragment=ET.tostring(web[0],encoding='utf-8')
    replacement=b'\n    '+fragment+b'\n  </ModuleData>'
    store.write_bytes(base_data.replace(b'</ModuleData>',replacement))
    assert store.read_bytes().replace(replacement,b'</ModuleData>')==base_data
    m.update(discoveryOnly=False,profileHashes=tree(out/'profile'),inputSeconds=120,cleanupSeconds=135,outerSeconds=155,destinationGroupID=group_id,expectedRows=contract.expected_rows(m['openFile'],m['destinationIDs']),initialOtherXML=contract.other_xml(out/'profile/SystemData/ModuleStore.xml'),initialSystemConfig=contract.canonical(ET.parse(system).getroot()),checkpoints=CHECKPOINTS,scope='Normal private clone of source '+approved_source+' whole app with actual leaf/group/blank destination drags, physical Delete, overwrite Cancel, native Color picker retarget/untouched-close/Cancel, persisted XML and normal Quit. No UI_TEST/internal model/drop/color callbacks. Original source/show/controllers/web isolated.')
    m['moduleStorePreparation']=dict(sourceSha256=sha(BASE/'SystemData/ModuleStore.xml'),insertedModuleSha256=hashlib.sha256(fragment).hexdigest(),sourceBytesPreserved=True,method='Raw single ModuleData closing-tag insertion; existing namespace declarations and QName values untouched.')
    m['expectedModuleStoreBackup']=backup_path(out/'profile/SystemData/ModuleStore.xml')
    m['approvedSourceExecutableSha256']=approved_source;m['approvedCanonicalApp']=str(canonical_app.resolve())
    m['approvedBuildInputs']=str(build_inputs.resolve()) if build_inputs else None
    for p in [Path(__file__).resolve(),ROOT/'scripts/test_vixen2_physical.py',ROOT/'scripts/vixen2_physical_contract.py',ROOT/'scripts/foreground_vixen2_full_physical.swift',ROOT/'scripts/check_mapper_file_panels.py',ROOT/'scripts/test_vixen2_full_control.py',ROOT/'scripts/test_vixen2_xml_qnames.py']:
        m['pins'][str(p)]=sha(p)
    for p in [canonical_app/'Contents/MacOS/Vixen',canonical_app/'Contents/Info.plist']+([build_inputs] if build_inputs else []):m['pins'][str(p.resolve())]=sha(p)
    save(out/'prepared.json',m)
    validate_prepared(out,json.loads((out/'prepared.json').read_text()))
    print(json.dumps(dict(fullPrepared=str(out/'prepared.json'),launched=False,expectedRows=m['expectedRows'])))

def validate_prepared(out,m):
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists() and not (out/'runtime').exists()
    assert m['discoveryOnly'] is False and [m[k] for k in ['inputSeconds','cleanupSeconds','outerSeconds']]==[120,135,155]
    assert m['checkpoints']==CHECKPOINTS and m['expectedRows']==contract.expected_rows(m['openFile'],m['destinationIDs'])
    assert all(sha(p)==h for p,h in m['pins'].items())
    assert tree(BASE)==m['sourceProfileHashes'] and sha(SAMPLE)==m['sourceVixSha256']
    assert tree(out/'profile')==m['profileHashes'] and not list((out/'settings').iterdir())
    approved=m['approvedSourceExecutableSha256'];assert re.fullmatch('[0-9a-f]{64}',approved)
    assert m['sourceExecutableSha256']==approved and Path(m['sourceApp']).resolve()==Path(m['approvedCanonicalApp'])
    assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==approved and sha(m['executable'])==m['privateExecutableSha256']
    if approved!=RELEASE:
        assert m['approvedBuildInputs'] is not None
        inputs=json.loads(Path(m['approvedBuildInputs']).read_text())['sources'];assert len(inputs)==808
        assert all(sha(ROOT/p)==h for p,h in inputs.items()),'Approved production build input drift'
    assert m['environment']==dict(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1')
    assert m['arguments']==['no_controllers','no_execution'] and m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    discovery.assert_services(out/'profile');assert not discovery.map_entries(out/'profile')
    config=ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot();nodes=config.find('Nodes');assert len(nodes)==3
    assert [x.attrib['id'] for x in nodes[:2]]==m['destinationIDs'][:2]
    assert nodes[2].attrib==dict(id=m['destinationGroupID'],name='V2 Fixture Group') and [x.attrib['id'] for x in nodes[2]]==m['destinationIDs'][2:]
    assert m['initialOtherXML']==contract.other_xml(out/'profile/SystemData/ModuleStore.xml') and m['initialSystemConfig']==contract.canonical(config)
    assert m['expectedModuleStoreBackup']==backup_path(out/'profile/SystemData/ModuleStore.xml')

def backup_path(store):
    # XElementFileWriter stamps the *old* file's mtime, using a 12-hour clock.
    return 'SystemData/auto_backup/ModuleStore_'+time.strftime('%m%d%Y_%I%M%S',time.localtime(store.stat().st_mtime))+'.xml'

def checkpoint_state(out,m,name,accepted):
    assert name in CHECKPOINTS
    request=json.loads((out/('checkpoint-'+name+'.request')).read_text())
    assert request==dict(checkpoint=name),'Unexpected checkpoint request schema'
    if name=='cancelled':assert accepted is not None,'Cancelled checkpoint arrived before accepted'
    state=contract.require_saved(out/'profile/SystemData/ModuleStore.xml',MAP_NAME,m['expectedRows'],m['initialOtherXML'])
    if name=='cancelled':assert state['moduleStoreSha256']==accepted['moduleStoreSha256'],'Cancel changed saved ModuleStore bytes'
    discovery.assert_services(out/'profile')
    return dict(passed=True,checkpoint=name,**state)

def service_checkpoints(out,m,completed,pending,deadline):
    """Nonblocking bounded polling; writes an affirmative response only after exact disk proof."""
    for name in CHECKPOINTS:
        request=out/('checkpoint-'+name+'.request');response=out/('checkpoint-'+name+'.json')
        if not request.exists():continue
        if name in completed:continue
        assert request.is_file() and not request.is_symlink() and request.stat().st_size<4096
        assert json.loads(request.read_text())==dict(checkpoint=name),'Unexpected checkpoint request'
        if name=='cancelled':assert 'accepted' in completed,'Out-of-order checkpoint'
        until=pending.setdefault(name,min(deadline-8,time.monotonic()+2.5))
        try:
            state=checkpoint_state(out,m,name,completed.get('accepted'))
        except (AssertionError,OSError,ValueError,ET.ParseError) as error:
            if time.monotonic()>=until:
                save(response,dict(passed=False,checkpoint=name,error=repr(error)))
                raise AssertionError('Persisted '+name+' checkpoint failed: '+repr(error))
            continue
        assert time.monotonic()<until,'Checkpoint response budget expired'
        save(response,state);completed[name]=state

def accept(out,m,fg,completed):
    assert fg.get('passed') is True and fg.get('discoveryOnly') is False and fg.get('normalQuitMenu') is True
    for key in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','inputCompleted']:assert fg.get(key) is True,key
    assert not fg['cleanupErrors'] and fg['physicalAfterRestoration']==dict(buttons=[],keys=[],modifiers=0)
    assert set(completed)==set(CHECKPOINTS)
    for name in CHECKPOINTS:assert json.loads((out/('checkpoint-'+name+'.json')).read_text())==completed[name]
    final=contract.require_saved(out/'profile/SystemData/ModuleStore.xml',MAP_NAME,m['expectedRows'],m['initialOtherXML'])
    assert final['moduleStoreSha256']==completed['accepted']['moduleStoreSha256']==completed['cancelled']['moduleStoreSha256']
    assert contract.canonical(ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot())==m['initialSystemConfig']
    discovery.assert_services(out/'profile')
    counts=accept_observations(fg,m)
    profile_validation=accept_profile(out,m)
    return dict(savedMapRows=final['rows'],savedModuleStoreSha256=final['moduleStoreSha256'],checkpointCount=2,unrelatedModuleXMLUnchanged=True,privateProfileValidation=profile_validation,**counts)

def accept_profile(out,m):
    profile=out/'profile'
    assert not any(p.is_symlink() for p in profile.rglob('*')),'Linked profile output'
    current=tree(profile);initial=m['profileHashes']
    assert set(initial)=={'ColorPalette.xml','SystemData/SystemConfig.xml','SystemData/ModuleStore.xml'}
    backup=m['expectedModuleStoreBackup']
    assert set(initial)<=set(current) and set(current)-set(initial)<={'logs/Application.Info.log','VixenApplicationData.xml',backup},'Unknown profile output or stale lock'
    # save_map saves ModuleStore once; cancellation does not save. shutdown only
    # writes application data (boot.rs::shutdown), so exactly this old-byte backup
    # is expected. Never allow arbitrary auto_backup files or accepted-state copies.
    assert current.get(backup)==initial['SystemData/ModuleStore.xml'],'Missing or changed pre-save ModuleStore backup'
    assert current['ColorPalette.xml']==initial['ColorPalette.xml'],'Unrelated palette changed'
    assert 'VixenApplicationData.xml' in current,'Normal Quit did not persist application data'
    if 'logs/Application.Info.log' in current:
        log=profile/'logs/Application.Info.log';assert log.stat().st_size<4_000_000;log.read_text()
    # The two explicit persisted-state checks already constrain SystemConfig and
    # ModuleStore. Reuse the independent strict empty/default application-data
    # validator with all observed known files except its candidate document pinned.
    allowed={k:v for k,v in current.items() if k!='VixenApplicationData.xml'}
    validation=validate_private_profile(profile,allowed)
    assert validation['accepted'],'Unexpected application data '+str(validation)
    return dict(passed=True,files=current,applicationData=validation)

def accept_observations(fg,m):
    assert fg['mouseDowns']==fg['mouseUps']==11 and fg['keyDowns']==fg['keyUps']==5
    events=fg['events'];pointer=[e for e in events if 'kind' in e];keys=[e for e in events if 'key' in e]
    assert len(events)==153 and len(pointer)==143 and len(keys)==10
    assert collections.Counter(e['kind'] for e in pointer)=={1:11,2:11,5:11,6:110}
    held=False;held_keys=set()
    for e in events:
        if 'kind' in e:
            assert set(e)=={'kind','point','window'} and len(e['point'])==2 and all(isinstance(x,(int,float)) and math.isfinite(x) for x in e['point']) and e['window']>0
            if e['kind']==1:assert not held;held=True
            elif e['kind']==2:assert held;held=False
            elif e['kind']==6:assert held
            else:assert e['kind']==5 and not held
        else:
            assert set(e)=={'key','down'} and isinstance(e['down'],bool)
            if e['down']:assert e['key'] not in held_keys;held_keys.add(e['key'])
            else:assert e['key'] in held_keys;held_keys.remove(e['key'])
    assert not held and not held_keys
    assert keys==[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False),(51,True),(51,False)]]
    observations=fg['observations'];drags=[o['stage'] for o in observations if o.get('dragCompleted') is True]
    assert drags==['map-leaf-1','map-leaf-2','map-group','blank-table-drop','overwrite-cancel-drag']
    row_observations=[o for o in observations if 'rows' in o]
    target_progress=[[],[1],[1,2],[1,2,3,4],[1,2,3,4],[1,2,3],[1,2,3],[1,2,3],[1,2,3],[1,2,3],[1,2,3]]
    stages=['initial-six-rows','after-leaf-1','after-leaf-2','after-group-leaves','after-rejected-blank','after-physical-delete','after-overwrite-cancel','after-row1-color-ownership','before-map-accept','first-reopen-exact-rows','cancelled-reopen-exact-rows']
    assert [o['stage'] for o in row_observations]==stages
    source_names=['Red','Green','Blue','White','Yellow','']
    for step,(o,targets) in enumerate(zip(row_observations,target_progress)):
        expected=[]
        for i,r in enumerate(m['expectedRows']):
            target='V2 Fixture '+str(i+1) if i+1 in targets else ''
            imported=source_names[i] if target else ''
            if step>=7 and i==0:imported='Lime'
            if step>=8 and i==1:imported='Red'
            expected.append([r['number'],r['output'],r['name'],source_names[i],target,imported,''])
        assert o['rows']==expected,'Actual AX row state differs at '+o['stage']
    blank=[o for o in observations if o['stage']=='blank-drop-geometry'];assert len(blank)==1
    x,y=blank[0]['point'];bx,by,bw,bh=blank[0]['table'];assert bx<x<bx+bw and by<y<by+bh and y>blank[0]['lastRowBottom']+40
    comparison=[o for o in observations if o['stage']=='blank-drop-comparison'];assert len(comparison)==1
    assert comparison[0]['beforeRows']==comparison[0]['afterRows']==row_observations[3]['rows']==row_observations[4]['rows']
    red=[o for o in observations if o['stage']=='unchanged-active-red-observed'];assert len(red)==1 and red[0]['value']=='rgb 1 0 0 1'
    # Native field readback proves the cancelled color edit is distinct, even
    # though only the accepted state is allowed to reach persisted XML.
    rgb=[o['value'] for o in observations if o.get('stage')=='color-hex' and 'value' in o]
    assert rgb==['00FF00','0000FF'],'Missing distinct native color edits'
    return dict(pointerEvents=143,keyEvents=10,physicalDrags=5,nativeRowObservations=11)

def terminate(child,deadline):
    if child is None or child.poll() is not None:return
    for sig in [signal.SIGTERM,signal.SIGKILL]:
        try:os.killpg(child.pid,sig)
        except ProcessLookupError:return
        try:child.wait(timeout=min(2,max(.05,deadline-time.monotonic())));return
        except subprocess.TimeoutExpired:pass

def run(out):
    began=time.monotonic();deadline=began+155;m=json.loads((out/'prepared.json').read_text());app=driver=None;before=None;completed={};pending={}
    result=dict(passed=False,scope=m['scope'],sourceExecutableSha256=m['sourceExecutableSha256'],privateExecutableSha256=m['privateExecutableSha256'],cleanupErrors=[])
    try:
        before=owner_preferences();validate_prepared(out,m)
        names=subprocess.check_output(['/bin/ps','-axo','pid=,comm='],text=True,timeout=3)
        assert not any(Path(x.strip().split(None,1)[1]).name=='Vixen' or Path(x.strip().split(None,1)[1]).name.startswith('foreground-') for x in names.splitlines() if len(x.strip().split(None,1))==2),'Another native app/helper is active'
        desktop=json.loads(subprocess.check_output([m['helper'],'--desktop'],text=True,timeout=3));save(out/'desktop-before.json',desktop)
        assert desktop['accessibility'] and desktop['eventPosting'] and desktop['noPhysicalInputHeld'] and len(desktop['pointer'])==2
        assert not desktop['application']['bundleIdentifier'].startswith('com.vixenlights.') and 'securityagent' not in desktop['application']['bundleIdentifier'].lower()
        (out/'runtime').mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(m['environment'],TMPDIR=str(out/'runtime')+'/')
        assert 'VIXEN_UI_TEST' not in env
        with (out/'process.log').open('w') as log,(out/'driver.log').open('w') as dlog:
            assert deadline-time.monotonic()>145,'Complete launch and cleanup budget unavailable'
            app=subprocess.Popen([m['executable'],*m['arguments']],env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            cfg=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],openFile=m['openFile'],mapName=m['mapName'],discovery=False)
            save(out/'driver-config.json',cfg);save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable']))
            assert deadline-time.monotonic()>140,'Full driver cleanup reserve unavailable'
            driver=subprocess.Popen([m['helper'],str(out/'driver-config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable'],driverPID=driver.pid,driverProcessGroup=driver.pid,driverExecutable=m['helper']))
            end=min(deadline-12,time.monotonic()+138)
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
            try:driver.wait(timeout=min(2,max(.1,deadline-time.monotonic()-5)))
            except subprocess.TimeoutExpired:pass
            except BaseException as e:result['cleanupErrors'].append('cooperative stop: '+repr(e))
        for child in [driver,app]:
            try:terminate(child,deadline)
            except BaseException as e:result['cleanupErrors'].append('owned cleanup: '+repr(e))
        def checked(name,operation):
            try:return operation()
            except BaseException as e:result['cleanupErrors'].append(name+': '+repr(e));return False
        fg=checked('foreground',lambda:json.loads((out/'foreground.json').read_text())) if (out/'foreground.json').exists() else {}
        result.update(foreground=fg,checkpoints=completed,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,ownerPreferenceHashesBefore=before,ownerPreferencesUnchanged=checked('preferences',lambda:before is not None and owner_preferences()==before),sourceProfileUnchanged=checked('sourceprofile',lambda:tree(BASE)==m['sourceProfileHashes']),sourceVixUnchanged=checked('sourcevix',lambda:sha(SAMPLE)==m['sourceVixSha256']),pinsUnchanged=checked('pins',lambda:all(sha(p)==v for p,v in m['pins'].items())),inputsUnchanged=checked('input',lambda:sha(m['openFile'])==m['pins'][m['openFile']]),elapsedSeconds=time.monotonic()-began)
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['passed']=result['passed'] and not result['cleanupErrors'] and all(result[k] for k in ['processExited','driverExited','ownerPreferencesUnchanged','sourceProfileUnchanged','sourceVixUnchanged','pinsUnchanged','inputsUnchanged','noInputHeld','restoredPrevious']) and result['elapsedSeconds']<155
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','elapsedSeconds','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path,default=ROOT/'target/vixen2-physical-97498-private/Vixen.app');p.add_argument('--helper',type=Path);p.add_argument('--approved-source-sha',default=RELEASE);p.add_argument('--canonical-app',type=Path);p.add_argument('--build-inputs',type=Path);mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve(),a.approved_source_sha,a.canonical_app.resolve() if a.canonical_app else None,a.build_inputs.resolve() if a.build_inputs else None)
    else:raise SystemExit(0 if run(out)['passed'] else 1)
