#!/usr/bin/env python3
"""Private optimized Vixen2 UI regression. Prepare first; run only in an assigned native slot."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import struct
import subprocess
import time
import uuid
import xml.etree.ElementTree as ET
from prepare_private_test_app import code_digest

ROOT=Path(__file__).resolve().parents[1]
OLD=ROOT/'target/vixen2-ui'
BASE=ROOT/'target/library-drag-fixture/profile'

def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def tree_hashes(path):return {str(p.relative_to(path)):digest(p) for p in sorted(path.rglob('*')) if p.is_file()}
def local(tag):return tag.rsplit('}',1)[-1]
def child(e,name):return next(x for x in e if local(x.tag)==name)
def text(e,name):return (child(e,name).text or '').strip()
def canonical(e,skip=()):
    return (local(e.tag),(e.text or '').strip(),sorted((local(k),v.split(':')[-1] if local(k)=='type' else v) for k,v in e.attrib.items()),[canonical(c,skip) for c in e if local(c.tag) not in skip])
def normalized_sequence(path):
    root=ET.parse(path).getroot();models={text(x,'ModuleInstanceId'):x for x in child(root,'_dataModels')};effects=[]
    for node in child(root,'_effectNodeSurrogates'):
        model=models.pop(text(node,'InstanceId'));assert text(model,'ModuleTypeId')==text(node,'TypeId')
        targets=[text(x,'NodeId') for x in child(node,'TargetNodes')]
        effects.append(dict(start=text(node,'StartTime'),length=text(node,'TimeSpan'),type=text(node,'TypeId'),targets=targets,data=canonical(model,('ModuleInstanceId',))))
    assert not models
    return dict(length=text(root,'Length'),effects=sorted(effects,key=lambda x:json.dumps(x,sort_keys=True)))
def saved_map(path):
    root=ET.parse(path).getroot();entries=[x for x in root.iter() if any(local(c.tag)=='Key' and c.text=='V2 Test Map' for c in x)];assert len(entries)==1
    rows=[x for x in entries[0].iter() if local(x.tag)=='ChannelMapping'];assert len(rows)==6
    return [dict(channelName=text(x,'ChannelName'),channelNumber=int(text(x,'ChannelNumber')),output=int(text(x,'ChannelOutput')),target=text(x,'ElementNodeId'),mixing=text(x,'RgbPixel')=='true',channelColor={local(c.tag):(c.text or '') for c in child(x,'ChannelColor')},destinationColor={local(c.tag):(c.text or '') for c in child(x,'DestinationColor')}) for x in rows]
def owner_preferences():
    paths=[Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist']
    folder=Path.home()/'Library/Application Support/Vixen'
    if folder.exists():paths+=sorted(p for p in folder.rglob('*') if p.is_file())
    return {str(p):digest(p) if p.is_file() else None for p in paths}
def frontmost():return subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=5).strip()
def assert_no_vixen():
    lines=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=5).splitlines()
    assert not any(x.strip().endswith('/Contents/MacOS/Vixen') for x in lines),'Another Vixen process is active'

def prepare(out,private_manifest):
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    private=json.loads(private_manifest.read_text());app=Path(private['app']);exe=app/'Contents/MacOS/Vixen'
    assert private['passed'] and private['signatureVerified'] and private['sourceUnchanged']
    assert digest(exe)==private['executableSha256'] and code_digest(exe)==private['executableCodeDigest']
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes());assert info['CFBundleIdentifier']==private['bundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(app)],check=True,capture_output=True,timeout=20)
    originals={str(OLD/name):digest(OLD/name) for name in ['sample.vix','bad.vix','expected.json','roundtrip.json','edges.json','converted.tim','roundtrip/result.json','edges-final/result.json']}
    profile_hashes=tree_hashes(BASE);out.mkdir(parents=True);(out/'inputs').mkdir();(out/'artifacts').mkdir()
    for name in ['sample.vix','bad.vix','expected.json','converted.tim']:shutil.copy2(OLD/name,out/'inputs'/name)
    shutil.copytree(BASE,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
    system=out/'profile/SystemData/SystemConfig.xml';tree=ET.parse(system);r=tree.getroot()
    assert len(child(r,'Channels'))==len(child(r,'Nodes'))==len(child(r,'Controllers'))==len(child(r,'Previews'))==len(child(r,'Patches'))==0
    expected=json.loads((out/'inputs/expected.json').read_text())
    for index,node_id in enumerate(expected['ids']):
        channel=str(uuid.uuid5(uuid.NAMESPACE_URL,'vixen-private-v2-channel-'+node_id));name='V2 Fixture '+str(index+1)
        ET.SubElement(child(r,'Channels'),'Channel',id=channel,name=name)
        ET.SubElement(child(r,'Nodes'),'Node',id=node_id,channelId=channel,name=name)
    tree.write(system,encoding='utf-8',xml_declaration=True)
    scripts={}
    for name in ['roundtrip','edges']:
        doc=json.loads((OLD/(name+'.json')).read_text())
        for step in doc['steps']:
            if step.get('action')=='hook' and step.get('name')=='vixen2':
                args=step.get('path',[])
                for i,value in enumerate(args):
                    if value.startswith(str(OLD)+'/'):
                        suffix=Path(value).name
                        assert suffix in ['sample.vix','bad.vix','converted.tim']
                        args[i]=str(out/('artifacts' if suffix=='converted.tim' else 'inputs')/suffix)
        target=out/(name+'.json');target.write_text(json.dumps(doc,indent=2)+'\n');scripts[name]=dict(path=str(target),sha256=digest(target),expectedSteps=len(doc['steps'])-1)
    assert [scripts[x]['expectedSteps'] for x in ['roundtrip','edges']]==[48,26]
    manifest=dict(app=str(app),privateAppManifest=str(private_manifest),privateAppManifestSha256=digest(private_manifest),appSha256=digest(exe),sourceAppSha256=private['sourceExecutableSha256'],codeDigest=private['executableCodeDigest'],bundleIdentifier=private['bundleIdentifier'],scripts=scripts,originalFiles=originals,sourceProfile=str(BASE),sourceProfileHashes=profile_hashes,preparedProfileHashes=tree_hashes(out/'profile'),inputs=tree_hashes(out/'inputs'),runnerSha256=digest(__file__),expected=expected,limits=['Native AppKit buttons/toggle; existing hooks supply filename, map name, destination drop, map selection and conversion save.','No WindowServer gestures, system color chooser interaction or normal Quit claim.','Immediate test-harness exit; private stale lock removed only after owned process exit.'])
    (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
    assert tree_hashes(BASE)==profile_hashes and all(digest(p)==v for p,v in originals.items())
    print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=74,appSha256=manifest['appSha256'],launched=False)))

def validate_run(out,name,manifest):
    script=json.loads(Path(manifest['scripts'][name]['path']).read_text())['steps'];rows=json.loads((out/name/'result.json').read_text())
    assert len(rows)==manifest['scripts'][name]['expectedSteps']
    assert all(row['step']==i and row['action']==script[i]['action'] and row['ok'] for i,row in enumerate(rows))
    # Pin the original successful semantic expectations; paths alone are rewritten.
    old=json.loads((OLD/('roundtrip' if name=='roundtrip' else 'edges-final')/'result.json').read_text())
    for i,(row,step) in enumerate(zip(rows,script)):
        if step['action']=='hook':
            wanted=old[i].get('detail','').replace(str(OLD/'converted.tim'),str(out/'artifacts/converted.tim'))
            assert row.get('detail','')==wanted,(name,i,wanted,row)
        if step['action']=='snapshot':
            png=(out/name/(step['name']+'.png')).read_bytes();assert png[:8]==b'\x89PNG\r\n\x1a\n' and len(png)>100
            w,h=struct.unpack('>II',png[16:24]);assert w>100 and h>100
    return len(rows)

def run(out):
    m=json.loads((out/'prepared.json').read_text());assert digest(__file__)==m['runnerSha256']
    app=Path(m['app']);exe=app/'Contents/MacOS/Vixen';assert digest(exe)==m['appSha256'] and digest(m['privateAppManifest'])==m['privateAppManifestSha256']
    assert all(digest(Path(v['path']))==v['sha256'] for v in m['scripts'].values())
    assert tree_hashes(out/'profile')==m['preparedProfileHashes'] and tree_hashes(out/'inputs')==m['inputs']
    assert_no_vixen();before_prefs=owner_preferences();before_front=frontmost();started=time.monotonic();report=dict(passed=False,steps=0,appSha256=m['appSha256'],sourceAppSha256=m['sourceAppSha256'],bundleIdentifier=m['bundleIdentifier'],limits=m['limits'],runs=[])
    try:
        first_map=None
        for name in ['roundtrip','edges']:
            assert time.monotonic()-started<110
            assert_no_vixen();ui=out/name;ui.mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
            env.update(VIXEN_UI_TEST=m['scripts'][name]['path'],VIXEN_UI_TEST_OUT=str(ui),VIXEN_UI_TEST_TRACE='1',VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(ui),VIXEN_MUTE='1')
            with (out/(name+'.log')).open('w') as log:
                process=subprocess.Popen([str(exe)],cwd=out,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                try:code=process.wait(timeout=60)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid,signal.SIGTERM)
                    try:process.wait(timeout=3)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
                    raise AssertionError(name+' exceeded60 seconds; owned process terminated')
            assert code==0,(name,code)
            assert frontmost()==before_front,'Foreground app changed during offscreen regression'
            steps=validate_run(out,name,m);report['steps']+=steps
            mapping=saved_map(out/'profile/SystemData/ModuleStore.xml')
            for i,row in enumerate(mapping):
                assert row['channelName']=='Channel '+str(i+1) and row['channelNumber']==i+1 and row['output']==i
                assert row['target']==m['expected']['ids'][0 if i<3 else i-2] and row['mixing']==(i<3)
                assert int(row['channelColor']['value'])==m['expected']['colors'][i]&0xffffffff
                if i<3:assert int(row['destinationColor']['knownColor'])==[141,79,37][i] and row['destinationColor']['state']=='1'
                else:assert int(row['destinationColor']['value'])==m['expected']['colors'][i]&0xffffffff and row['destinationColor']['state']=='2'
            if first_map is None:first_map=mapping
            else:assert mapping==first_map,'Cancel/overwrite-warning changed persisted map'
            shutil.copy2(out/'profile/SystemData/ModuleStore.xml',out/'artifacts'/('module-store-after-'+name+'.xml'))
            lock=out/'profile/.lock'
            if lock.exists():
                assert lock.read_text().startswith(str(process.pid)+'@'),'Unexpected private profile lock owner'
                lock.unlink()
            report['runs'].append(dict(name=name,pid=process.pid,exitCode=code,steps=steps,map=mapping))
            (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        expected=normalized_sequence(out/'inputs/converted.tim');actual=normalized_sequence(out/'artifacts/converted.tim')
        assert actual==expected and len(actual['effects'])==7 and actual['length']=='PT0.4S'
        (out/'artifacts/normalized-converted.json').write_text(json.dumps(actual,indent=2)+'\n')
        report.update(passed=True,normalizedConversionMatchesHistorical=True,effects=7,lengthMilliseconds=400,mapReopenedInFreshProcess=True,cancelPreservesPersistedMapping=True,privateLockRemovedAfterOwnedExit=True)
    except Exception as error:
        report['error']=str(error);raise
    finally:
        report.update(elapsedSeconds=time.monotonic()-started,sourceProfileUnchanged=tree_hashes(BASE)==m['sourceProfileHashes'],sourceArtifactsUnchanged=all(digest(p)==v for p,v in m['originalFiles'].items()),ownerPreferencesUnchanged=owner_preferences()==before_prefs,foregroundUnchanged=frontmost()==before_front,ownerPreferenceHashes=before_prefs,foregroundBefore=before_front,appUnchanged=digest(exe)==m['appSha256'])
        report['passed']=report['passed'] and all(report[k] for k in ['sourceProfileUnchanged','sourceArtifactsUnchanged','ownerPreferencesUnchanged','foregroundUnchanged','appUnchanged'])
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    assert report['passed'],report
    print(json.dumps({k:report[k] for k in ['passed','steps','effects','elapsedSeconds','ownerPreferencesUnchanged','foregroundUnchanged']}))

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);p.add_argument('--private-app-manifest',type=Path)
    mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare-only',action='store_true');mode.add_argument('--run-prepared',action='store_true');a=p.parse_args();out=a.out.resolve()
    assert out.is_relative_to(ROOT/'target')
    if a.prepare_only:assert a.private_app_manifest;prepare(out,a.private_app_manifest.resolve())
    else:run(out)
if __name__=='__main__':main()
