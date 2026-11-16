#!/usr/bin/env python3
"""Prepare or run bounded, normal-app Open/Save panels on a private fixture.

Default is preparation only. --run-prepared requires a currently agreed exclusive
desktop interval. No UI_TEST hooks, scripted panel replacement, clipboard writes,
owner preference deletion, hardware outputs, or app launches during preparation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import time
import unicodedata
import xml.etree.ElementTree as ET

from prepare_private_test_app import code_digest

ROOT = Path(__file__).resolve().parents[1]
local = lambda name: name.rsplit('}',1)[-1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def hashes(root):
    return {str(p.relative_to(root)):digest(p) for p in root.rglob('*') if p.is_file()}


def read_only_state(path):
    if not path.exists(): return {'exists':False}
    assert path.is_file() and not path.is_symlink(), path
    return {'exists':True,'bytes':path.stat().st_size,'sha256':digest(path)}


def content(path):
    root=ET.parse(path).getroot()
    def canonical(n):
        # Keep the representation JSON-stable: prepared.json reloads tuples as
        # arrays. Native saved-file comparisons must use the same shape.
        return [n.tag,[list(pair) for pair in sorted(n.attrib.items())],(n.text or '').strip(),[canonical(c) for c in n]]
    sections=['ModuleInstanceId','ModuleTypeId','Length','SequenceLayers','_dataModels','_effectNodeSurrogates','_filterNodeSurrogates','_layerMixingFilterDataModels','_layerMixingFilterSurrogates','_mediaSurrogates','LabeledMarkCollections']
    return {name:canonical(next(c for c in root if local(c.tag)==name)) for name in sections}


def validate(prepared):
    fixture=json.loads(prepared.read_text())
    app=Path(fixture['app']).resolve();manifest=fixture['appManifest']
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes());exe=app/'Contents/MacOS'/info['CFBundleExecutable']
    assert app.is_relative_to(ROOT/'target') and info['CFBundleIdentifier']==manifest['bundleIdentifier']
    assert info['CFBundleIdentifier'].startswith('com.vixenlights.verification.') and manifest['signatureVerified']
    assert digest(exe)==manifest['executableSha256'] and code_digest(exe)==manifest['executableCodeDigest']
    assert b'VIXEN_SETTINGS_DIR' in exe.read_bytes()
    profile=Path(fixture['profile']).resolve();settings=Path(fixture['settings']).resolve()
    source=Path(fixture['openFile']).resolve();saved=Path(fixture['saveFile']).resolve()
    assert all(p.is_relative_to(prepared.parent) for p in [profile,settings,source,saved])
    assert source.is_relative_to(profile) and not saved.is_relative_to(profile) and not settings.is_relative_to(profile)
    assert fixture['environment']=={'VIXEN_DATA':str(profile),'VIXEN_SETTINGS_DIR':str(settings),'VIXEN_MUTE':'1'}
    assert fixture['arguments']==['no_controllers','no_execution']
    assert not any(p.is_symlink() for p in [prepared,app,exe,profile,settings,source,saved.parent])
    assert len(ET.parse(profile/'SystemData/SystemConfig.xml').getroot().find('Controllers'))==0
    store=ET.parse(profile/'SystemData/ModuleStore.xml').getroot()
    web=[n for n in store.iter() if local(n.tag)=='IsEnabled' and 'WebServer' in n.tag]
    assert len(web)==1 and (web[0].text or '').lower()=='false'
    assert not list((profile/'Media').rglob('*'))
    assert fixture['effectInstanceId'] in source.read_text(encoding='utf-8-sig')
    return fixture,exe,profile,settings,source,saved


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--prepared',type=Path,required=True)
    p.add_argument('--helper',type=Path,default=ROOT/'target/foreground-native-panels')
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--run-prepared',action='store_true')
    a=p.parse_args();prepared=a.prepared.resolve();out=a.out.resolve();helper=a.helper.resolve()
    assert prepared.is_relative_to(ROOT/'target') and out.is_relative_to(ROOT/'target')
    fixture,exe,profile,settings,source,saved=validate(prepared)
    assert helper.is_file() and not out.is_relative_to(profile)
    if not a.run_prepared:
        assert not out.exists();out.mkdir(parents=True)
        assert hashes(profile)==fixture['initialProfileHashes'] and digest(source)==fixture['originalSequenceSha256']
        assert not saved.exists() and not (profile/'.lock').exists() and not list(settings.iterdir())
        record={'prepared':True,'launched':False,'fixtureManifest':str(prepared),'fixtureManifestSha256':digest(prepared),'helper':str(helper),'helperSha256':digest(helper),'helperSourceSha256':digest(ROOT/'scripts/foreground_native_panels.swift'),'runnerSha256':digest(Path(__file__)),'appExecutable':str(exe),'appSha256':digest(exe),'originalContent':content(source),'scope':'Actual normal-app Open/Cancel/Open Unicode source/Save As Cancel/Save As Unicode destination/close/reopen/normal Quit. AX structures not yet observed on this host; unknown or cross-process UI aborts with diagnostics.'}
        (out/'prepared.json').write_text(json.dumps(record,indent=2)+'\n');print(out/'prepared.json');return
    record=json.loads((out/'prepared.json').read_text())
    assert record['fixtureManifest']==str(prepared) and record['fixtureManifestSha256']==digest(prepared)
    assert record['helperSha256']==digest(helper) and record['helperSourceSha256']==digest(ROOT/'scripts/foreground_native_panels.swift') and record['runnerSha256']==digest(Path(__file__))
    assert record['appSha256']==digest(exe) and hashes(profile)==fixture['initialProfileHashes']
    assert not saved.exists() and not (profile/'.lock').exists() and not list(settings.iterdir())
    assert not (out/'run-config.json').exists(), 'Use a fresh fixture and run folder after any attempted launch'
    subprocess.run(['codesign','--verify','--deep','--strict',str(Path(fixture['app']))],capture_output=True,check=True,timeout=15)
    state=json.loads(subprocess.run([str(helper),'--desktop-state'],capture_output=True,text=True,check=True,timeout=10).stdout)
    (out/'desktop-before.json').write_text(json.dumps(state,indent=2)+'\n')
    assert state['accessibility'] and state['eventPosting'], 'Permissions are unavailable; no permission prompt or app launch attempted'
    assert state['pid']>0 and not state['vixenProcesses'], 'Another Vixen process is present; stop without touching it'
    owner_paths=[Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist',Path.home()/'Library/Application Support/Vixen/Settings.xml']
    owner_before={str(path):read_only_state(path) for path in owner_paths}
    runtime=out/'runtime';runtime.mkdir()
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(fixture['environment']);env['TMPDIR']=str(runtime)+'/'
    assert 'VIXEN_UI_TEST' not in env and 'VIXEN_TEST_SAVE_AS' not in env
    process=driver=None;failure=None;driver_code=None;process_code=None;forced=False;restore=None
    started=time.monotonic()
    with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as driver_log:
        try:
            process=subprocess.Popen([str(exe),*fixture['arguments']],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            config={'pid':process.pid,'out':str(out),'executable':str(exe),'bundleIdentifier':fixture['appManifest']['bundleIdentifier'],'openFile':str(source),'saveFile':str(saved),'originalSequenceSha256':fixture['originalSequenceSha256'],'previousPID':state['pid']}
            (out/'run-config.json').write_text(json.dumps(config,indent=2)+'\n')
            driver=subprocess.Popen([str(helper),str(out/'run-config.json')],stdout=driver_log,stderr=subprocess.STDOUT,start_new_session=True)
            driver_code=driver.wait(timeout=165)
            if driver_code!=0: raise RuntimeError('Native panel driver stopped; inspect foreground.json and failure.ax.json')
            process_code=process.wait(timeout=10)
        except BaseException as error:
            failure=str(error)
        finally:
            if driver is not None and driver.poll() is None:
                os.killpg(driver.pid,signal.SIGKILL);driver.wait();driver_code=driver.returncode
            if process is not None and process.poll() is None:
                # Capture only the owned test process before cleanup. This
                # distinguishes blocked termination from missing key delivery.
                try:
                    sampled=subprocess.run(['/usr/bin/sample',str(process.pid),'1','1','-file',str(out/'owned-app.sample.txt')],capture_output=True,text=True,timeout=5)
                    (out/'sample.log').write_text(sampled.stdout+sampled.stderr)
                except BaseException as error:
                    (out/'sample.log').write_text(str(error))
                forced=True;os.killpg(process.pid,signal.SIGTERM)
                try: process.wait(timeout=5)
                except subprocess.TimeoutExpired: os.killpg(process.pid,signal.SIGKILL);process.wait()
            if process is not None: process_code=process.returncode
            # Restore only the exact app recorded immediately before this run.
            try:
                current=json.loads(subprocess.run([str(helper),'--desktop-state'],capture_output=True,text=True,check=True,timeout=10).stdout)
                if current['pid']!=state['pid'] and process is not None and current['pid']==process.pid:
                    restored=subprocess.run([str(helper),'--restore',str(state['pid'])],capture_output=True,text=True,timeout=5)
                    restore={'exitCode':restored.returncode,'stdout':restored.stdout}
            except BaseException as error: restore={'error':str(error)}
    foreground=json.loads((out/'foreground.json').read_text()) if (out/'foreground.json').exists() else {}
    owner_after={str(path):read_only_state(path) for path in owner_paths}
    result={'passed':False,'scope':record['scope'],'elapsedSeconds':round(time.monotonic()-started,3),'driverExitCode':driver_code,'appExitCode':process_code,'forcedCleanup':forced,'failure':failure,'foreground':foreground,'fallbackRestore':restore,'ownerPreferenceStatesBefore':owner_before,'ownerPreferenceStatesAfter':owner_after,'ownerPreferenceFilesUnchanged':owner_before==owner_after,'profileLockRemoved':not (profile/'.lock').exists(),'appSha256':digest(exe),'sourceSha256':digest(source),'savedExists':saved.exists(),'privateSettingsFiles':hashes(settings),'profileAfter':hashes(profile)}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    assert failure is None and not forced and process_code==0 and driver_code==0,result
    assert foreground.get('passed') and foreground.get('targetExited') and foreground.get('restoredPrevious'),foreground
    assert owner_before==owner_after and digest(exe)==record['appSha256'] and digest(helper)==record['helperSha256']
    assert digest(source)==fixture['originalSequenceSha256'] and not (profile/'.lock').exists()
    assert saved.is_file() and content(saved)==record['originalContent']
    observed={row['stage']:row for row in foreground['steps']}
    for stage in ['initial','open-cancelled','save-as-cancelled']: assert not observed[stage]['savedExists']
    assert observed['saved-live']['savedSha256']==observed['reopened-live']['savedSha256']==digest(saved)
    normalized=lambda path: unicodedata.normalize('NFC',path)
    assert normalized(observed['source-opened']['openedDocument'])==normalized(str(source)) and normalized(observed['saved-reopened']['openedDocument'])==normalized(str(saved))
    result.update(passed=True,savedSha256=digest(saved),savedContent=content(saved),commandQQuitTested=False,normalQuitMethod='Observed Quit Vixen menu AXPress; owner hold-to-quit safeguard unchanged')
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':True,'report':str(out/'result.json')}))


if __name__=='__main__':main()
