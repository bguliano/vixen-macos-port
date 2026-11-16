#!/usr/bin/env python3
"""Prepare a normal-app Vixen2 native discovery; root coordinates all launches.

No UI_TEST, model/color/drop hook, shared profile, controller or web listener.
The initial discovery observes actual Mapper and system Color AX controls,
then cancels and quits. Physical workflow acceptance is a later frozen run.
"""
import argparse,hashlib,json,os,plistlib,shutil,signal,subprocess,sys,time,uuid
from pathlib import Path
import xml.etree.ElementTree as ET
from prepare_private_test_app import code_digest

ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/library-drag-fixture/profile'
SAMPLE=ROOT/'target/vixen2-ui/sample.vix'
RELEASE='97498b4bf93ed7416bfdfbeffaef82cdb5dbaa5768bf79af5db79c942375f3f8'
MAP_NAME='Physical Vixen2 Map'
local=lambda name:name.rsplit('}',1)[-1]
sha=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest()
def tree(path):return {str(p.relative_to(path)):sha(p) for p in sorted(path.rglob('*')) if p.is_file()}
def save(path,value):
    tmp=Path(str(path)+'.tmp');tmp.write_text(json.dumps(value,indent=2,sort_keys=True)+'\n');tmp.replace(path)
def canonical(node):return [node.tag,[list(p) for p in sorted(node.attrib.items())],(node.text or '').strip(),[canonical(c) for c in node]]
def owner_preferences():
    paths=[Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist'];folder=Path.home()/'Library/Application Support/Vixen'
    if folder.exists():paths+=sorted(p for p in folder.rglob('*') if p.is_file())
    return {str(p):sha(p) if p.is_file() else None for p in paths}
def assert_services(profile):
    system=ET.parse(profile/'SystemData/SystemConfig.xml').getroot()
    assert all(len(system.find(name))==0 for name in ['Controllers','Patches','Previews'])
    modules=ET.parse(profile/'SystemData/ModuleStore.xml').getroot()
    web=[e for e in modules.iter() if local(e.tag)=='IsEnabled' and 'WebServer' in e.tag]
    assert len(web)==1 and web[0].text=='false','Private WebServer must explicitly remain disabled'
    permitted={'Module.App.Curves','Module.App.ColorGradients','Module.App.WebServer','Module.SequenceType.Vixen2x'}
    assert all(e.attrib.get('assemblyName') in permitted for e in modules.iter() if local(e.tag)=='Module')
    assert not (profile/'Programs').exists() and not (profile/'Schedules').exists()
def map_entries(profile):
    root=ET.parse(profile/'SystemData/ModuleStore.xml').getroot()
    return [e for e in root.iter() if local(e.tag)=='Key' and (e.text or '')==MAP_NAME]
def prepare(out,app,helper):
    assert out.is_relative_to(ROOT/'target') and not out.exists();out.mkdir(parents=True)
    manifest=app.parent/'private-app.json';clone=json.loads(manifest.read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['sourceUnchanged'] and clone['signatureVerified'] and clone['sourceExecutableSha256']==RELEASE
    assert sha(exe)==clone['executableSha256'] and code_digest(exe)==clone['executableCodeDigest']
    assert plistlib.loads((app/'Contents/Info.plist').read_bytes())['CFBundleIdentifier']==clone['bundleIdentifier']
    profile=out/'profile';shutil.copytree(BASE,profile);(profile/'.lock').unlink(missing_ok=True)
    shutil.rmtree(profile/'Sequence');(profile/'fixture.json').unlink()
    ids=[str(uuid.uuid5(uuid.NAMESPACE_URL,'private-vixen2-physical-'+str(i))) for i in range(1,5)]
    system=profile/'SystemData/SystemConfig.xml';xml=ET.parse(system);r=xml.getroot()
    assert all(len(r.find(n))==0 for n in ['Channels','Nodes','Controllers','Patches','Previews'])
    for i,node in enumerate(ids):
        name='V2 Fixture '+str(i+1);channel=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-vixen2-physical-channel-'+node))
        ET.SubElement(r.find('Channels'),'Channel',id=channel,name=name);ET.SubElement(r.find('Nodes'),'Node',id=node,channelId=channel,name=name)
    xml.write(system,encoding='utf-8',xml_declaration=True)
    store=profile/'SystemData/ModuleStore.xml';xml=ET.parse(store);r=xml.getroot();data=r.find('ModuleData')
    module='8ac1cdf9-8d57-43b6-81de-93003ed2f907';ns='{http://schemas.datacontract.org/2004/07/VixenModules.App.WebServer}'
    e=ET.SubElement(data,'Module',dataModelType='VixenModules.App.WebServer.Data, WebServer',moduleType=module,moduleInstance=module,assemblyName='Module.App.WebServer');d=ET.SubElement(e,ns+'Data')
    ET.SubElement(d,'ModuleInstanceId').text=module;ET.SubElement(d,'ModuleTypeId').text=module;ET.SubElement(d,ns+'HttpPort').text='8888';ET.SubElement(d,ns+'IsEnabled').text='false';xml.write(store,encoding='utf-8',xml_declaration=True)
    (out/'inputs').mkdir();sample=out/'inputs/Private Vixen2 Sample.vix';shutil.copy2(SAMPLE,sample)
    parsed=ET.parse(sample).getroot();assert parsed.find('Audio') is None and parsed.find('Profile') is None
    (out/'settings').mkdir();assert_services(profile)
    files=[Path(__file__).resolve(),ROOT/'scripts/foreground_vixen2_physical.swift',ROOT/'scripts/prepare_private_test_app.py',manifest,exe,app/'Contents/Info.plist',helper,sample]
    m=dict(preparedOnly=True,launched=False,discoveryOnly=True,app=str(app),executable=str(exe),helper=str(helper),bundleIdentifier=clone['bundleIdentifier'],sourceApp=clone['sourceApp'],sourceExecutableSha256=RELEASE,privateExecutableSha256=sha(exe),codeDigest=clone['executableCodeDigest'],sourceProfile=str(BASE),sourceProfileHashes=tree(BASE),sourceVix=str(SAMPLE),sourceVixSha256=sha(SAMPLE),profile=str(profile),profileHashes=tree(profile),openFile=str(sample),mapName=MAP_NAME,destinationIDs=ids,settings=str(out/'settings'),environment=dict(VIXEN_DATA=str(profile),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1'),arguments=['no_controllers','no_execution'],inputSeconds=60,cleanupSeconds=75,outerSeconds=95,pins={str(p):sha(p) for p in files},scope='Normal private97498 full app; accepted Open Sequence panel path, actual Mapper and system Color control discovery, untouched picker close, map/converter Cancel, normal Quit menu. No VIXEN_UI_TEST or internal callbacks. This discovery is not drag/color ownership acceptance.')
    save(out/'prepared.json',m);validate_prepared(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),launched=False,source=RELEASE)))
def validate_prepared(out,m):
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists() and not (out/'runtime').exists()
    assert m['discoveryOnly'] is True and m['outerSeconds']==95 and m['inputSeconds']==60 and m['cleanupSeconds']==75
    assert all(sha(p)==v for p,v in m['pins'].items())
    assert tree(Path(m['sourceProfile']))==m['sourceProfileHashes'] and sha(m['sourceVix'])==m['sourceVixSha256']
    assert tree(out/'profile')==m['profileHashes'] and not list((out/'settings').iterdir())
    assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==RELEASE and sha(m['executable'])==m['privateExecutableSha256']
    assert m['environment']==dict(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1')
    assert m['arguments']==['no_controllers','no_execution'] and m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    assert_services(out/'profile');assert not map_entries(out/'profile')
def terminate(child,deadline):
    if child is None or child.poll() is not None:return
    for sig in [signal.SIGTERM,signal.SIGKILL]:
        try:os.killpg(child.pid,sig)
        except ProcessLookupError:return
        try:child.wait(timeout=min(2,max(.05,deadline-time.monotonic())));return
        except subprocess.TimeoutExpired:pass
def accept_discovery(out,m,fg):
    assert fg.get('passed') is True and fg.get('discoveryOnly') is True and fg.get('normalQuitMenu') is True
    for key in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','inputCompleted']:assert fg.get(key) is True,key
    assert fg['mouseDowns']==fg['mouseUps']==0 and fg['keyDowns']==fg['keyUps']==4 and not fg['cleanupErrors']
    assert not any(e.get('kind') in [1,2,6] for e in fg['events'])
    mapper=json.loads((out/'mapper-discovery.ax.json').read_text());color=json.loads((out/'color-discovery.ax.json').read_text())
    assert any(r['description']=='Vixen 2 channel mappings' for r in mapper['rows'])
    assert any(r['description']=='Vixen 2 destination elements' for r in mapper['rows'])
    assert all(any(name in [r['title'],r['description'],r['value']] for r in mapper['rows']) for name in ['Channel 1','Channel 6','V2 Fixture 1','Set Import Color'])
    assert any(r['title']=='Set Import Color' for r in color['rows'])
    assert not map_entries(out/'profile'),'Discovery unexpectedly saved a map'
    assert_services(out/'profile')
    return dict(mapperAXRows=len(mapper['rows']),colorAXRows=len(color['rows']),nativeColorPanelObserved=True,mapNotSaved=True)
def run(out):
    began=time.monotonic();deadline=began+95;m=json.loads((out/'prepared.json').read_text());app=driver=None;before=None
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
            assert deadline-time.monotonic()>85,'Complete launch and cleanup budget unavailable'
            app=subprocess.Popen([m['executable'],*m['arguments']],env=env,cwd=out,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            cfg=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],openFile=m['openFile'],mapName=m['mapName'],discovery=True)
            save(out/'driver-config.json',cfg);save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable']))
            assert deadline-time.monotonic()>80,'Full driver cleanup reserve unavailable'
            driver=subprocess.Popen([m['helper'],str(out/'driver-config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            save(out/'owned-processes.json',dict(appPID=app.pid,appProcessGroup=app.pid,appExecutable=m['executable'],driverPID=driver.pid,driverProcessGroup=driver.pid,driverExecutable=m['helper']))
            end=min(deadline-10,time.monotonic()+78)
            while app.poll() is None and time.monotonic()<end:
                if driver.poll() not in [None,0]:raise RuntimeError('Native discovery stopped; inspect foreground report')
                time.sleep(.05)
            assert app.poll()==0,'Private app did not finish normal Quit'
            driver.wait(timeout=min(4,max(.1,deadline-time.monotonic()-6)));assert driver.returncode==0
        result.update(appExitCode=app.returncode,driverExitCode=driver.returncode)
        fg=json.loads((out/'foreground.json').read_text());result.update(accept_discovery(out,m,fg));result['passed']=True
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
        result.update(foreground=fg,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,ownerPreferenceHashesBefore=before,ownerPreferencesUnchanged=checked('preferences',lambda:before is not None and owner_preferences()==before),sourceProfileUnchanged=checked('sourceprofile',lambda:tree(BASE)==m['sourceProfileHashes']),sourceVixUnchanged=checked('sourcevix',lambda:sha(SAMPLE)==m['sourceVixSha256']),pinsUnchanged=checked('pins',lambda:all(sha(p)==v for p,v in m['pins'].items())),elapsedSeconds=time.monotonic()-began)
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['passed']=result['passed'] and not result['cleanupErrors'] and all(result[k] for k in ['processExited','driverExited','ownerPreferencesUnchanged','sourceProfileUnchanged','sourceVixUnchanged','pinsUnchanged','noInputHeld','restoredPrevious']) and result['elapsedSeconds']<95
        save(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','elapsedSeconds','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--app',type=Path,default=ROOT/'target/vixen2-physical-97498-private/Vixen.app');p.add_argument('--helper',type=Path);mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.helper.resolve())
    else:raise SystemExit(0 if run(out)['passed'] else 1)
