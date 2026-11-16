#!/usr/bin/env python3
"""Prepare/run a private pinned full-app Mapper destination-drag acceptance.

Prepare never launches app/helper. Runtime requires the root's exclusive native slot.
Seven WindowServer drags cover valid leaf/group table/outline targets and blank
rejections. Real Delete clears a source; unsaved final mutation is canceled.
Existing hooks only set up, read, supply a tree, save, and reopen private files.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import signal
import subprocess
import sys
import time
import uuid
import xml.etree.ElementTree as ET
from prepare_private_test_app import code_digest
ROOT=Path(__file__).resolve().parents[1]
SOURCE_PROFILE=ROOT/'target/library-drag-fixture/profile'
D129='d1293962718f3da58ed9796ad43ff022f1fe775786b23aa864b590c32cb8a215'
IDS={name:str(uuid.uuid5(uuid.NAMESPACE_URL,'private-mapper-drag-'+name)) for name in ['Source A','Source B','Source Group','Source Child','Destination A','Destination B','Destination Group','Destination Child']}
STAGES=[
 dict(marker='table-a',container='Element mappings',source='Destination A',target='Source A',operation='drag'),
 dict(marker='table-b',container='Element mappings',source='Destination B',target='Source B',operation='drag'),
 dict(marker='table-blank',container='Element mappings',source='Destination A',target='blank',operation='drag'),
 dict(marker='outline-group',container='Source elements',source='Destination Group',target='Source Group',operation='drag'),
 dict(marker='outline-leaf',container='Source elements',source='Destination B',target='Source A',operation='drag'),
 dict(marker='outline-blank',container='Source elements',source='Destination A',target='blank',operation='drag'),
 dict(marker='clear',container='Source elements',source='',target='Source A',operation='clear'),
 dict(marker='cancel',container='Source elements',source='Destination A',target='Source B',operation='cancel',expectedBefore='Source B → Destination B')]
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def hashes(p):return {str(f.relative_to(p)):sha(f) for f in sorted(p.rglob('*')) if f.is_file()}
def write(p,value):Path(p).write_text(json.dumps(value,indent=2,sort_keys=True)+'\n')
def hook(*args):return dict(action='hook',name='elementMapper',path=list(args))
def wait(n=.3):return dict(action='wait',seconds=n)
def owner_preferences():
    paths=[Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist'];folder=Path.home()/'Library/Application Support/Vixen'
    if folder.exists():paths+=sorted(p for p in folder.rglob('*') if p.is_file())
    return {str(p):sha(p) if p.is_file() else None for p in paths}
def parse_state(detail):
    assert detail.startswith('name='),detail
    return {source:target for source,target in re.findall(r'\[([0-9a-fA-F-]{36})\]=([^|]*)',detail)}
def prepare(out,app,driver,source_sha=D129):
    assert out.is_relative_to(ROOT/'target') and not (out/'prepared.json').exists()
    out.mkdir(parents=True,exist_ok=True)
    clone_path=app.parent/'private-app.json';clone=json.loads(clone_path.read_text());exe=app/'Contents/MacOS/Vixen'
    assert clone['passed'] and clone['sourceUnchanged'] and clone['signatureVerified'] and clone['sourceExecutableSha256']==source_sha
    assert clone['app']==str(app) and clone['executableSha256']==sha(exe) and clone['executableCodeDigest']==code_digest(exe)
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes());assert info['CFBundleIdentifier']==clone['bundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(app)],check=True,capture_output=True,timeout=20)
    profile=out/'profile';shutil.copytree(SOURCE_PROFILE,profile);(profile/'.lock').unlink(missing_ok=True)
    cfg=profile/'SystemData/SystemConfig.xml';tree=ET.parse(cfg);r=tree.getroot()
    assert all(len(r.find(tag))==0 for tag in ['Channels','Nodes','Controllers','Previews','Patches'])
    for name in ['Destination A','Destination B','Destination Child']:
        channel=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-mapper-drag-channel-'+name));ET.SubElement(r.find('Channels'),'Channel',id=channel,name=name)
        if name!='Destination Child':ET.SubElement(r.find('Nodes'),'Node',id=IDS[name],channelId=channel,name=name)
    group=ET.SubElement(r.find('Nodes'),'Node',id=IDS['Destination Group'],name='Destination Group')
    ET.SubElement(group,'Node',id=IDS['Destination Child'],channelId=str(uuid.uuid5(uuid.NAMESPACE_URL,'private-mapper-drag-channel-Destination Child')),name='Destination Child')
    tree.write(cfg,encoding='utf-8',xml_declaration=True)
    inputs=out/'inputs';inputs.mkdir();sources=inputs/'sources.json';source_tree=inputs/'source-tree.v3e'
    write(sources,[dict(id=IDS[n],name=n) for n in ['Source A','Source B']])
    tree_data=dict(id=str(uuid.UUID(int=0)),name='Root',children=[dict(id=IDS[n],name=n) for n in ['Source A','Source B']]+[dict(id=IDS['Source Group'],name='Source Group',children=[dict(id=IDS['Source Child'],name='Source Child')])]);write(source_tree,tree_data)
    artifacts=out/'artifacts';artifacts.mkdir();saved=artifacts/'Mapper Drag Saved.vmap'
    state={IDS['Source A']:'',IDS['Source B']:''};observations=[]
    steps=[dict(action='window',id='administration'),wait(.8),hook('open',str(sources)),hook('show'),wait(.6)]
    def observe():
        observations.append(dict(index=len(steps),expected=dict(state)));steps.append(hook('state'))
    for stage in STAGES:
        marker=stage['marker']
        if marker=='outline-group':steps.extend([hook('tree',str(source_tree)),wait(.3)])
        if marker=='cancel':steps.extend([hook('save',str(saved)),wait(.2)])
        observe();steps.extend([dict(action='snapshot',name=marker),wait(7)])
        if marker=='table-a':state[IDS['Source A']]=IDS['Destination A']
        elif marker=='table-b':state[IDS['Source B']]=IDS['Destination B']
        elif marker=='outline-group':state[IDS['Source Group']]=IDS['Destination Group']
        elif marker=='outline-leaf':state[IDS['Source A']]=IDS['Destination B']
        elif marker=='clear':state[IDS['Source A']]=''
    cancel_index=len(steps);steps.extend([hook('result'),hook('open',str(sources)),hook('load',str(saved))]);observe()
    steps.extend([dict(action='snapshot',name='reopened'),wait(.5),hook('cancel'),dict(action='quit')]);write(out/'steps.json',dict(steps=steps))
    pins={str(p):sha(p) for p in [Path(__file__).resolve(),ROOT/'scripts/foreground_mapper_drag.swift',ROOT/'scripts/prepare_private_test_app.py',driver,exe,clone_path,app/'Contents/Info.plist',out/'steps.json',sources,source_tree]}
    manifest=dict(preparedOnly=True,launched=False,stageName='mapper-destination-drag',app=str(app),executable=str(exe),driver=str(driver),bundleIdentifier=clone['bundleIdentifier'],privateAppManifest=str(clone_path),sourceApp=str(clone['sourceApp']),sourceAppSha256=source_sha,sourceProfile=str(SOURCE_PROFILE),sourceProfileHashes=hashes(SOURCE_PROFILE),profileHashes=hashes(profile),inputHashes=hashes(inputs),pins=pins,stages=STAGES,observations=observations,cancelResultIndex=cancel_index,expectedSaved=state,sourceTree=tree_data,savedMap=str(saved),steps=len(steps)-1,inputDeadlineSeconds=90,driverCleanupDeadlineSeconds=105,timeoutSeconds=135,command=[sys.executable,'-B',str(Path(__file__).resolve()),'--out',str(out),'--run-prepared'],scope='Private code-identical source-pinned full app; seven real OS drags on table/outline valid/blank rows, physical Delete, native Cancel after observed unsaved mutation, exact saved .vmap targets and reopened state. Hooks only setup/readback/tree/save. No file-panel, normal-Quit, Vixen2, or arbitrary cross-app payload claim.')
    write(out/'prepared.json',manifest);print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=manifest['steps'],launched=False)))

def validate(out,m,foreground,rows):
    assert foreground['passed'] and foreground['inputCompleted'] and foreground['targetExited'] and foreground['restoredPrevious'] and foreground['restoredPointer'] and foreground['noInputHeld'] and not foreground['cleanupErrors']
    assert foreground['mouseDowns']==foreground['mouseUps']==8 and foreground['keyDowns']==foreground['keyUps']==1
    assert len(rows)==m['steps'] and all(row['ok'] for row in rows)
    for obs in m['observations']:assert parse_state(rows[obs['index']]['detail'])==obs['expected'],(obs,rows[obs['index']])
    assert rows[m['cancelResultIndex']]['detail']=='cancelled'
    assert foreground['cancelDismissalObserved'] and isinstance(foreground['cancelAXStatus'],int)
    assert foreground['unsavedMappingBefore']=='Source B → Destination B'
    assert foreground['unsavedMappingAfter']=='Source B → Destination A'
    assert all(foreground[key]==sha(m['savedMap']) for key in ['savedHashBeforeUnsavedChange','savedHashAfterCancel','savedHashAfterReopen'])
    for stage in m['stages']:
        done=json.loads((out/('driver-'+stage['marker']+'.json')).read_text());assert done==dict(completed=True,stage=stage['marker'])
    saved=json.loads(Path(m['savedMap']).read_text());actual={x['sourceId']:x.get('targetId','') for x in saved['elementMappings']}
    assert actual==m['expectedSaved'] and saved['sourceTree']==m['sourceTree'] and saved['name']=='Mapper Drag Saved'
    assert {x['sourceId']:x['sourceName'] for x in saved['elementMappings']}=={IDS[n]:n for n in ['Source A','Source B','Source Group']}
    assert {x['sourceId']:x.get('targetName','') for x in saved['elementMappings']}=={IDS['Source A']:'',IDS['Source B']:'Destination B',IDS['Source Group']:'Destination Group'}
    assert not Path(m['savedMap']+'.part').exists()
    return dict(savedMapSha256=sha(m['savedMap']),savedMappings=actual,observations=len(m['observations']),cancelledUnsavedMapping=True)

def run(out):
    started=time.monotonic();deadline=started+135;m=json.loads((out/'prepared.json').read_text());assert not (out/'result.json').exists()
    result=dict(passed=False,scope=m['scope'],processExited=False,driverExited=False,cleanupCompleted=False,finalizationErrors=[]);app=driver=None
    before=owner_preferences();result['ownerPreferencesBefore']=before
    try:
        assert all(sha(p)==h for p,h in m['pins'].items()) and hashes(SOURCE_PROFILE)==m['sourceProfileHashes'] and hashes(out/'profile')==m['profileHashes'] and hashes(out/'inputs')==m['inputHashes']
        assert sha(Path(m['sourceApp'])/'Contents/MacOS/Vixen')==m['sourceAppSha256']
        running=subprocess.check_output(['/bin/ps','-axo','pid=,comm='],text=True,timeout=5)
        process_names=[Path(line.strip().split(None,1)[1]).name for line in running.splitlines() if len(line.strip().split(None,1))==2]
        assert not any(name in ['Vixen','MapperPanels','WindowLifecycle'] or name.startswith('foreground-') for name in process_names),'Another native Vixen or foreground helper is active'
        desktop=json.loads(subprocess.check_output([m['driver'],'--desktop'],text=True,timeout=5));assert desktop['accessibility'] and desktop['eventPosting'] and desktop['noPhysicalInputHeld'];write(out/'desktop-before.json',desktop)
        assert not desktop['application']['bundleIdentifier'].startswith('com.vixenlights.') and 'securityagent' not in desktop['application']['bundleIdentifier'].lower()
        for folder in ['ui','runtime','settings']:(out/folder).mkdir()
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log,(out/'driver.log').open('w') as dlog:
            assert deadline-time.monotonic()>120,'Insufficient complete run budget before app launch'
            app=subprocess.Popen([m['executable']],cwd=out,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned=dict(appPID=app.pid,appExecutable=m['executable'],appProcessGroup=app.pid);write(out/'owned-processes.json',owned)
            config=dict(pid=app.pid,out=str(out),executable=m['executable'],bundleIdentifier=m['bundleIdentifier'],previousApplication=desktop['application'],pointer=desktop['pointer'],stages=m['stages'],savedMap=m['savedMap']);write(out/'driver-config.json',config)
            assert deadline-time.monotonic()>115,'Insufficient input and cleanup budget before driver launch'
            driver=subprocess.Popen([m['driver'],str(out/'driver-config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(driverPID=driver.pid,driverExecutable=m['driver'],driverProcessGroup=driver.pid);write(out/'owned-processes.json',owned)
            app_deadline=min(deadline-20,time.monotonic()+110)
            while app.poll() is None:
                if driver.poll() not in [None,0]:raise RuntimeError('Foreground helper failed; no subsequent input is authorized')
                if time.monotonic()>=app_deadline:raise TimeoutError('App bound reached')
                time.sleep(.1)
            driver.wait(timeout=min(10,max(.1,deadline-time.monotonic()-10)))
        result.update(exitCode=app.returncode,driverExitCode=driver.returncode)
        assert app.returncode==driver.returncode==0
        foreground=json.loads((out/'foreground.json').read_text());rows=json.loads((out/'ui/result.json').read_text());result['foreground']=foreground
        result.update(validate(out,m,foreground,rows));result['passed']=True
    except BaseException as exc:result['error']=repr(exc)
    finally:
        # A cooperative stop gives the input driver a chance to release held input
        # under its exact ownership guards before any process is terminated.
        if driver is not None and driver.poll() is None:
            try:write(out/'abort-request.json',dict(abort=True))
            except BaseException as exc:result['finalizationErrors'].append('abort request: '+repr(exc))
            try:driver.wait(timeout=min(3,max(.1,deadline-time.monotonic()-5)))
            except subprocess.TimeoutExpired:pass
            except BaseException as exc:result['finalizationErrors'].append('cooperative stop: '+repr(exc))
        for name,child in [('driver',driver),('app',app)]:
            try:
                if child is not None and child.poll() is None:
                    for sig in [signal.SIGTERM,signal.SIGKILL]:
                        try:os.killpg(child.pid,sig)
                        except ProcessLookupError:break
                        try:child.wait(timeout=min(2,max(.1,deadline-time.monotonic())));break
                        except subprocess.TimeoutExpired:continue
            except BaseException as exc:result['finalizationErrors'].append(name+': '+repr(exc))
        def checked(name,operation):
            try:return operation()
            except BaseException as exc:result['finalizationErrors'].append(name+': '+repr(exc));return False
        fg=checked('foreground report',lambda:json.loads((out/'foreground.json').read_text())) if (out/'foreground.json').exists() else {}
        result.update(foreground=fg,elapsedSeconds=time.monotonic()-started,processExited=app is not None and app.poll() is not None,driverExited=driver is not None and driver.poll() is not None,sourceUnchanged=checked('source',lambda:hashes(SOURCE_PROFILE)==m['sourceProfileHashes']),inputsUnchanged=checked('inputs',lambda:hashes(out/'inputs')==m['inputHashes']),pinsUnchanged=checked('pins',lambda:all(sha(p)==h for p,h in m['pins'].items())),ownerPreferencesUnchanged=checked('preferences',lambda:owner_preferences()==before))
        result['noInputHeld']=isinstance(fg,dict) and fg.get('noInputHeld') is True;result['restoredPrevious']=isinstance(fg,dict) and fg.get('restoredPrevious') is True
        result['cleanupCompleted']=result['processExited'] and result['driverExited'] and result['noInputHeld'] and not result['finalizationErrors']
        result['passed']=result['passed'] and result['cleanupCompleted'] and result['restoredPrevious'] and all(result[k] for k in ['sourceUnchanged','inputsUnchanged','pinsUnchanged','ownerPreferencesUnchanged']) and result['elapsedSeconds']<135
        write(out/'result.json',result)
    print(json.dumps({k:result.get(k) for k in ['passed','error','exitCode','driverExitCode','elapsedSeconds','cleanupCompleted','noInputHeld','restoredPrevious']}));return result
if __name__=='__main__':
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--out',type=Path,required=True);ap.add_argument('--app',type=Path,default=ROOT/'target/mapper-drag-d129-private/Vixen.app');ap.add_argument('--driver',type=Path);ap.add_argument('--source-sha',default=D129);mode=ap.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=ap.parse_args();out=a.out.resolve()
    if a.prepare_only:prepare(out,a.app.resolve(),a.driver.resolve() if a.driver else out/'foreground-mapper-drag',a.source_sha)
    else:raise SystemExit(0 if run(out)['passed'] else 1)
