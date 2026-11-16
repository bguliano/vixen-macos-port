#!/usr/bin/env python3
"""Private normal-app Color Collection Picker ownership and discard verification.

Preparation and offline consumer checks do not launch the native app. Root owns
all --run-prepared invocations. Exact two-collection XML and Rust readback gate
both disk checkpoints; the helper alone performs native AX actions and Open keys.
"""
import argparse,hashlib,json,math,os,plistlib,re,shutil,signal,subprocess,sys,time
from pathlib import Path
import xml.etree.ElementTree as ET
import test_vixen2_physical as common
from prepare_private_test_app import code_digest
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/editor-library-fixture-v2/profile'
SETTINGS=ROOT/'target/collection-picker-settings-fixture-v1'
INSPECTOR=ROOT/'target/build-main/rust/debug/examples/collection_picker_fixture'
RELEASE='d1293962718f3da58ed9796ad43ff022f1fe775786b23aa864b590c32cb8a215'
CHECKPOINTS=['accepted','cancelled']
sha=common.sha;tree=common.tree;save=common.save;owner_preferences=common.owner_preferences
T='{http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed}'
D='{http://schemas.datacontract.org/2004/07/System.Drawing}'
I='{http://www.w3.org/2001/XMLSchema-instance}'
INITIAL=[dict(name='Collection A',description='Private green collection',colors=[4278255360]),dict(name='Collection B',description='Private blue collection',colors=[4278190335])]
EXPECTED=[dict(name='Collection A',description='Private green collection',colors=[4278255360,4294901760]),dict(name='Collection B',description='Private blue collection',colors=[4278190335])]

def xml_root(path,limit=65536):
    assert path.is_file() and not path.is_symlink() and path.stat().st_size<=limit
    text=path.read_text(encoding='utf-8-sig');body=text.lstrip()
    if body.startswith('<?xml '):
        end=body.find('?>');assert end>=0;body=body[end+2:]
    assert '<!' not in body and '<?' not in body,'Unexpected XML declaration/entity/comment'
    return ET.fromstring(text)

def require_collections(path,expected):
    root=xml_root(path)
    assert root.tag==T+'ArrayOfColorCollection' and not root.attrib and not (root.text or '').strip()
    assert len(root)==2
    actual=[]
    for node in root:
        assert node.tag==T+'ColorCollection' and not node.attrib and not (node.text or '').strip()
        assert [x.tag for x in node]==[T+'Color',T+'Description',T+'Name']
        colors,description,name=list(node)
        assert all(not x.attrib and not (x.tail or '').strip() for x in node)
        assert len(name)==len(description)==0 and not (colors.text or '').strip()
        values=[]
        for color in colors:
            assert color.tag==D+'Color' and not color.attrib and not (color.text or '').strip() and not (color.tail or '').strip()
            assert [x.tag for x in color]==[D+'knownColor',D+'name',D+'state',D+'value']
            known,label,state,value=list(color)
            assert all(len(x)==0 and not (x.tail or '').strip() for x in color)
            assert not known.attrib and known.text=='0' and label.attrib=={I+'nil':'true'} and not (label.text or '').strip()
            assert not state.attrib and state.text=='2' and not value.attrib and value.text is not None and value.text.isdecimal()
            number=int(value.text);assert str(number)==value.text and 0<=number<=4294967295;values.append(number)
        actual.append(dict(name=name.text or '',description=description.text or '',colors=values))
    assert actual==expected,'Exact collection order/descriptions/colors differ'
    return dict(collections=actual,colorCollectionsSha256=sha(path))

def inspect_saved(out,m):
    proof=require_collections(out/'settings/ColorCollections.xml',EXPECTED)
    loaded=json.loads(subprocess.check_output([m['inspector'],str(out/'settings'),'--inspect'],text=True,timeout=1))
    assert loaded==EXPECTED,'Rust loader differs from independently parsed XML'
    return dict(**proof,rustReadback=loaded)

def add_disabled_web(profile):
    store=profile/'SystemData/ModuleStore.xml';doc=ET.parse(store);root=doc.getroot();data=root.find('ModuleData')
    assert data is not None
    existing=[x for x in data if x.get('assemblyName')=='Module.App.WebServer']
    if existing:
        assert len(existing)==1
        common.assert_services(profile)
        return
    module='8ac1cdf9-8d57-43b6-81de-93003ed2f907';ns='{http://schemas.datacontract.org/2004/07/VixenModules.App.WebServer}'
    entry=ET.SubElement(data,'Module',dataModelType='VixenModules.App.WebServer.Data, WebServer',moduleType=module,moduleInstance=module,assemblyName='Module.App.WebServer');d=ET.SubElement(entry,ns+'Data')
    ET.SubElement(d,'ModuleInstanceId').text=module;ET.SubElement(d,'ModuleTypeId').text=module;ET.SubElement(d,ns+'HttpPort').text='8888';ET.SubElement(d,ns+'IsEnabled').text='false';doc.write(store,encoding='utf-8',xml_declaration=True)
    common.assert_services(profile)

def prepare(out,app,helper):
    assert out.is_relative_to(ROOT/'target') and not out.exists();out.mkdir(parents=True)
    manifest=app.parent/'private-app.json';clone=json.loads(manifest.read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['sourceUnchanged'] and clone['signatureVerified'] and clone['sourceExecutableSha256']==RELEASE
    assert sha(exe)==clone['executableSha256'] and code_digest(exe)==clone['executableCodeDigest']
    assert plistlib.loads((app/'Contents/Info.plist').read_bytes())['CFBundleIdentifier']==clone['bundleIdentifier']
    helper_version=helper.name.removeprefix('foreground-collection-picker-');assert re.fullmatch('v[0-9]+',helper_version)
    helper_preparation=ROOT/('target/collection-picker-helper-preparation-'+helper_version+'.json');proof=json.loads(helper_preparation.read_text())
    assert proof['compiled'] and proof['signatureVerified'] and proof['helperSha256']==sha(helper) and proof['sourceSha256']==sha(ROOT/'scripts/foreground_collection_picker.swift')
    profile=out/'profile';shutil.copytree(BASE,profile);(profile/'.lock').unlink(missing_ok=True);(profile/'fixture.json').unlink();add_disabled_web(profile)
    settings=out/'settings';shutil.copytree(SETTINGS,settings)
    require_collections(settings/'ColorCollections.xml',INITIAL)
    sequence=profile/'Sequence/Editor Library.tim'
    files=[Path(__file__).resolve(),ROOT/'scripts/foreground_collection_picker.swift',ROOT/'scripts/test_collection_picker_controls.py',ROOT/'scripts/test_vixen2_physical.py',ROOT/'scripts/prepare_private_test_app.py',ROOT/'crates/vixen-session/examples/collection_picker_fixture.rs',helper_preparation,manifest,exe,app/'Contents/Info.plist',helper,INSPECTOR]
    reference=json.loads((ROOT/'target/collection-picker-settings-reference.json').read_text())
    files += [ROOT/'scripts/collection_picker_settings_contract.py',ROOT/'target/collection-picker-settings-reference.json',Path(reference['nativeAcceptance'])]+[Path(p) for p in list(reference['files'])+list(reference['sourcePins'])]
    m=dict(preparedOnly=True,launched=False,workflow='collection-picker',app=str(app),executable=str(exe),helper=str(helper),helperPreparation=str(helper_preparation),bundleIdentifier=clone['bundleIdentifier'],sourceApp=clone['sourceApp'],sourceExecutableSha256=RELEASE,privateExecutableSha256=sha(exe),codeDigest=clone['executableCodeDigest'],sourceProfile=str(BASE),sourceProfileHashes=tree(BASE),sourceSettings=str(SETTINGS),sourceSettingsHashes=tree(SETTINGS),profile=str(profile),profileHashes=tree(profile),settings=str(settings),settingsHashes=tree(settings),openFile=str(sequence),openFileSha256=sha(sequence),sequenceTitle='Editor Library – profile',inspector=str(INSPECTOR),checkpoints=CHECKPOINTS,initialCollections=INITIAL,expectedCollections=EXPECTED,environment=dict(VIXEN_DATA=str(profile),VIXEN_SETTINGS_DIR=str(settings),VIXEN_MUTE='1'),arguments=['no_controllers','no_execution'],inputSeconds=90,cleanupSeconds=105,outerSeconds=125,pins={str(p):sha(p) for p in files},scope='Normal private d129 whole app: actual Collection Picker commits pending red to original collection A while selecting B; accepted save and blue Don\'t Save preserve exact collection data. Native Color/Picker AX controls, four Open key pairs, no pointer posts or UI_TEST. Isolated profile/settings; web/controllers/execution disabled.')
    save(out/'prepared.json',m);validate_prepared(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),launched=False)))

def validate_prepared(out,m):
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists() and not (out/'runtime').exists()
    assert m['workflow']=='collection-picker' and [m[k] for k in ['inputSeconds','cleanupSeconds','outerSeconds']]==[90,105,125]
    assert all(sha(p)==h for p,h in m['pins'].items())
    proof=json.loads(Path(m['helperPreparation']).read_text());assert proof['helperSha256']==sha(m['helper']) and proof['sourceSha256']==sha(ROOT/'scripts/foreground_collection_picker.swift')
    assert tree(BASE)==m['sourceProfileHashes'] and tree(SETTINGS)==m['sourceSettingsHashes']
    assert tree(out/'profile')==m['profileHashes'] and tree(out/'settings')==m['settingsHashes']
    assert m['checkpoints']==CHECKPOINTS and m['initialCollections']==INITIAL and m['expectedCollections']==EXPECTED
    assert m['openFile']==str(out/'profile/Sequence/Editor Library.tim') and sha(m['openFile'])==m['openFileSha256']
    assert m['sequenceTitle']=='Editor Library – profile' and m['inspector']==str(INSPECTOR)
    assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==RELEASE and sha(m['executable'])==m['privateExecutableSha256']
    assert m['environment']==dict(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1') and m['arguments']==['no_controllers','no_execution']
    assert m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    common.assert_services(out/'profile');require_collections(out/'settings/ColorCollections.xml',INITIAL)

def checkpoint_state(out,m,name,accepted):
    assert name in CHECKPOINTS and json.loads((out/('checkpoint-'+name+'.request')).read_text())==dict(checkpoint=name)
    if name=='cancelled':assert accepted is not None,'Cancelled before accepted'
    state=inspect_saved(out,m)
    if name=='cancelled':assert state['colorCollectionsSha256']==accepted['colorCollectionsSha256'],'Cancel changed saved collection bytes'
    current=tree(out/'profile');assert all(current.get(p)==h for p,h in m['profileHashes'].items()),'Existing profile file changed'
    common.assert_services(out/'profile')
    return dict(passed=True,checkpoint=name,**state)

def service_checkpoints(out,m,completed,pending,deadline):
    for name in CHECKPOINTS:
        request=out/('checkpoint-'+name+'.request');response=out/('checkpoint-'+name+'.json')
        if not request.exists() or name in completed:continue
        assert request.is_file() and not request.is_symlink() and request.stat().st_size<4096
        assert json.loads(request.read_text())==dict(checkpoint=name),'Unexpected checkpoint request'
        if name=='cancelled':assert 'accepted' in completed,'Out-of-order checkpoint'
        until=pending.setdefault(name,min(deadline-8,time.monotonic()+2.5))
        try:state=checkpoint_state(out,m,name,completed.get('accepted'))
        except (AssertionError,OSError,ValueError,ET.ParseError,subprocess.SubprocessError) as error:
            if time.monotonic()>=until:
                save(response,dict(passed=False,checkpoint=name,error=repr(error)));raise AssertionError('Persisted '+name+' checkpoint failed: '+repr(error))
            continue
        assert time.monotonic()<until,'Checkpoint response expired';save(response,state);completed[name]=state

def accept_observations(fg):
    assert fg.get('passed') is True and fg.get('collectionPickerWorkflow') is True and fg.get('normalQuitMenu') is True
    for k in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','inputCompleted']:assert fg.get(k) is True,k
    assert not fg['cleanupErrors'] and fg['physicalAfterRestoration']==dict(buttons=[],keys=[],modifiers=0)
    assert fg['mouseDowns']==fg['mouseUps']==0 and fg['keyDowns']==fg['keyUps']==4
    expected=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False)]]
    assert fg['events']==expected,'Unexpected key sequence or pointer event'
    obs=fg['observations'];select=[o for o in obs if 'nativeMenuItem' in o]
    assert [(o['stage'],o['selected'],o['nativeMenuItem']) for o in select]==[('initial-A','Collection A','Collection A'),('pending-red-switch-B','Collection B','Collection B'),('reopened-A','Collection A','Collection A')]
    assert all(o['before'] in ['', 'Collection A','Collection B'] for o in select) and select[1]['before']=='Collection A'
    assert [o['value'] for o in obs if o.get('stage')=='color-hex' and 'value' in o]==['FF0000','0000FF']
    assert [(o['expected'],o['observed']) for o in obs if o.get('stage')=='picker-start-hex']==[('FFFFFF','FFFFFF')]*2
    for stage in ['accept-collections','discard-pending-blue','quit-menu']:
        actions=[o for o in obs if o.get('stage')==stage];assert len(actions)==1 and actions[0]['action']=='AXPress' and actions[0]['status'] in [0,-25204]
    return dict(nativeCollectionSelections=3,nativeColorEdits=2,keyEvents=8,pointerEvents=0)

def accept_profile(out,m):
    profile=out/'profile';settings=out/'settings'
    assert not any(p.is_symlink() for folder in [profile,settings] for p in folder.rglob('*'))
    current=tree(profile);initial=m['profileHashes'];assert all(current.get(p)==h for p,h in initial.items()),'Existing profile changed'
    assert set(current)-set(initial)<={'logs/Application.Info.log','VixenApplicationData.xml'},'Unknown profile output or stale lock'
    data=xml_root(profile/'VixenApplicationData.xml',4096)
    expected=ET.Element('VixenApplicationData');ET.SubElement(expected,'DataFormatVersion').text='3';recent=ET.SubElement(expected,'RecentSequences');ET.SubElement(recent,'SequenceFile').text=m['openFile'];ET.SubElement(expected,'FilterShapePositions');ET.SubElement(expected,'FilterSetupFormHighQualityRendering',value='false')
    assert common.canonical(data)==common.canonical(expected),'Unexpected application data or recent sequence'
    if 'logs/Application.Info.log' in current:
        log=profile/'logs/Application.Info.log';assert log.stat().st_size<4_000_000;log.read_text()
    settings_proof=accept_settings(settings,m)
    return dict(passed=True,profileFiles=current,settingsValidation=settings_proof)

def accept_settings(settings,m):
    from collection_picker_settings_contract import require_settings
    return require_settings(settings,m)

def accept(out,m,fg,completed):
    counts=accept_observations(fg);assert set(completed)==set(CHECKPOINTS)
    for name in CHECKPOINTS:assert json.loads((out/('checkpoint-'+name+'.json')).read_text())==completed[name]
    final=inspect_saved(out,m);assert final['colorCollectionsSha256']==completed['accepted']['colorCollectionsSha256']==completed['cancelled']['colorCollectionsSha256']
    return dict(**counts,**final,checkpointCount=2,privateProfileValidation=accept_profile(out,m))

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
        (out/'runtime').mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(m['environment'],TMPDIR=str(out/'runtime')+'/')
        assert 'VIXEN_UI_TEST' not in env
        with (out/'process.log').open('w') as log,(out/'driver.log').open('w') as dlog:
            assert deadline-time.monotonic()>115,'Complete launch and cleanup budget unavailable'
            app=subprocess.Popen([m['executable'],*m['arguments']],env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            cfg=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],openFile=m['openFile'],sequenceTitle=m['sequenceTitle'],workflow='collection-picker')
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
        result.update(foreground=fg,checkpoints=completed,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,ownerPreferenceHashesBefore=before,ownerPreferencesUnchanged=checked('preferences',lambda:before is not None and owner_preferences()==before),sourceProfileUnchanged=checked('sourceprofile',lambda:tree(BASE)==m['sourceProfileHashes']),sourceSettingsUnchanged=checked('sourcesettings',lambda:tree(SETTINGS)==m['sourceSettingsHashes']),pinsUnchanged=checked('pins',lambda:all(sha(p)==v for p,v in m['pins'].items())),inputsUnchanged=checked('input',lambda:sha(m['openFile'])==m['openFileSha256']),elapsedSeconds=time.monotonic()-began)
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['passed']=result['passed'] and not result['cleanupErrors'] and all(result[k] for k in ['processExited','driverExited','ownerPreferencesUnchanged','sourceProfileUnchanged','sourceSettingsUnchanged','pinsUnchanged','inputsUnchanged','noInputHeld','restoredPrevious']) and result['elapsedSeconds']<125
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','elapsedSeconds','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path,default=ROOT/'target/private-app-raster-mapper-integration/Vixen.app');p.add_argument('--helper',type=Path,default=ROOT/'target/foreground-collection-picker-v6');mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve())
    else:raise SystemExit(0 if run(out)['passed'] else 1)
