#!/usr/bin/env python3
"""Prepare/check private prop insertion/reverse/cycle drag routing. Preparation never launches.

Real drags are sent only by the owned AX/CG helper. Existing app hooks open/save
synthetic documents and read state; no selection/drop/provider hooks are used.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import time
import xml.etree.ElementTree as ET
import zipfile
import uuid
from prepare_private_test_app import code_digest

ROOT = Path(__file__).resolve().parents[1]
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def hashes(path): return {str(p.relative_to(path)):digest(p) for p in sorted(path.rglob('*')) if p.is_file()}
def owner_states():
    paths=[Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist',Path.home()/'Library/Application Support/Vixen/Settings.xml']
    return {str(p):{'exists':p.exists(),'sha256':digest(p) if p.is_file() else None} for p in paths}
def package(path):
    with zipfile.ZipFile(path) as z:
        assert len(z.namelist()) == len(set(z.namelist()))
        assert z.getinfo('prop.json').file_size < 1024*1024
        return json.loads(z.read('prop.json'))

CASES = [
    dict(id='tree-insert-before', tab='layout', select=[], source='Pixel 2', target='Pixel 1', edge='before', graph={'Source':['Pixel 2','Pixel 1'],'Target':['Pixel 3']}),
    dict(id='tree-insert-after', tab='layout', select=[], source='Pixel 1', target='Pixel 2', edge='after', graph={'Source':['Pixel 2','Pixel 1'],'Target':['Pixel 3']}),
    dict(id='tree-insert-other-before', tab='layout', select=[], source='Pixel 3', target='Pixel 1', edge='before', graph={'Source':['Pixel 3','Pixel 1','Pixel 2'],'Target':[]}),
    dict(id='tree-insert-other-after', tab='layout', select=[], source='Pixel 3', target='Pixel 2', edge='after', graph={'Source':['Pixel 1','Pixel 2','Pixel 3'],'Target':[]}),
    dict(id='tree-command-reverse', tab='layout', select=['Pixel 1','Pixel 2'], source='Pixel 2', target='Target', command=True, graph={'Source':[],'Target':['Pixel 3','Pixel 2','Pixel 1']}),
    dict(id='tree-cycle-compatible', tab='layout', select=[], source='Branch', target='Inner'),
]


def expected(original, case):
    result=copy.deepcopy(original); by_name={e['name']:e for e in result['elements']}
    for parent,names in case.get('graph',{}).items():
        by_name[parent]['childIds']=[by_name[name]['id'] for name in names]
    return result


def canonical(value):
    value=copy.deepcopy(value)
    value['prop'].pop('modifiedDate',None)
    value['elements']=sorted(value['elements'],key=lambda e:e['id'])
    return value


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--app',type=Path,required=True)
    p.add_argument('--profile',type=Path,default=ROOT/'target/native-panels-quit-menu-fixture/profile')
    p.add_argument('--prop',type=Path,default=ROOT/'target/resume-ui/prop-canvas/canvas.prp')
    p.add_argument('--helper',type=Path,default=ROOT/'target/foreground-prop-drag-edges')
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--run-prepared',action='store_true')
    p.add_argument('--check-only',action='store_true')
    p.add_argument('--layout-only',action='store_true')
    a=p.parse_args()
    app,source,prop,out,helper=(x.resolve() for x in [a.app,a.profile,a.prop,a.out,a.helper])
    assert all(x.is_relative_to(ROOT/'target') for x in [app,source,prop,out,helper])
    assert not out.is_relative_to(source)
    clone_path=app.parent/'private-app.json';clone=json.loads(clone_path.read_text())
    info=plistlib.loads((app/'Contents/Info.plist').read_bytes());exe=app/'Contents/MacOS'/info['CFBundleExecutable']
    assert clone['passed'] and clone['signatureVerified'] and clone['sourceUnchanged']
    assert clone['bundleIdentifier']==info['CFBundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    assert digest(exe)==clone['executableSha256'] and code_digest(exe)==clone['executableCodeDigest'] and b'VIXEN_SETTINGS_DIR' in exe.read_bytes()
    assert len(ET.parse(source/'SystemData/SystemConfig.xml').getroot().find('Controllers'))==0
    web=[n for n in ET.parse(source/'SystemData/ModuleStore.xml').getroot().iter() if n.tag.split('}')[-1]=='IsEnabled' and 'WebServer' in n.tag]
    assert len(web)==1 and web[0].text.lower()=='false'
    assert not list((source/'Media').rglob('*'))
    assert not any(path.is_symlink() for path in source.rglob('*'))
    if a.run_prepared or a.check_only:
        prepared=json.loads((out/'prepared.json').read_text())
        assert prepared['layoutOnly']==a.layout_only
        assert prepared['appSha256']==digest(exe) and prepared['helperSha256']==digest(helper)
        assert prepared['runnerSha256']==digest(Path(__file__)) and prepared['helperSourceSha256']==digest(ROOT/'scripts/foreground_prop_drag_edges.swift')
        assert prepared['stepsSha256']==digest(out/'steps.json') and prepared['cases']==CASES
    else:
        assert not out.exists();out.mkdir(parents=True)
        shutil.copytree(source,out/'profile');(out/'profile/.lock').unlink(missing_ok=True)
        (out/'settings').mkdir();(out/'saved').mkdir()
        original=package(prop);original['prop']['name']='Prop Drag Verification'
        next(e for e in original['elements'] if e['id']==original['rootElementId'])['name']='Prop Drag Verification'
        # Valid kind-compatible descendant target: Branch contains only groups;
        # Inner is empty and can accept groups, so only ancestry forbids this drop.
        source_group=next(e for e in original['elements'] if e['name']=='Source')
        extra=[]
        for name in ['Branch','Inner']:
            e=copy.deepcopy(source_group);e.update(id=str(uuid.uuid5(uuid.NAMESPACE_URL,'vixen-prop-drag-edge-'+name)),statePropertyId=str(uuid.uuid5(uuid.NAMESPACE_URL,'vixen-prop-drag-edge-state-'+name)),name=name,childIds=[])
            extra.append(e)
        extra[0]['childIds']=[extra[1]['id']]
        next(e for e in original['elements'] if e['id']==original['rootElementId'])['childIds'].append(extra[0]['id'])
        original['elements'].extend(extra)
        with zipfile.ZipFile(prop) as zin,zipfile.ZipFile(out/'fixture.prp','w') as zout:
            for name in zin.namelist():
                assert name in ['prop.json','background.jpg']
                zout.writestr(name,json.dumps(original).encode() if name=='prop.json' else zin.read(name))
        h=lambda *args:{'action':'hook','name':'props','path':list(args)}
        wait=lambda seconds=.4:{'action':'wait','seconds':seconds}
        steps=[wait(2),{'action':'window','id':'custom-prop-editor'},wait(.7),{'action':'focus','window':'Custom Prop Editor'}]
        for case in CASES[:1] if a.layout_only else CASES:
            name=case['id']
            steps += [h('open',str(out/'fixture.prp')),wait(.4),h('label-window','Prop Drag Verification'),h('tab',case['tab']),h('expand'),wait(.4),h('state'),h('canvas'),h('order'),{'action':'dump','name':name+'-ready'},{'action':'snapshot','name':name+'-ready'}]
            if not a.layout_only:
                steps += [wait(9),h('state'),h('canvas'),h('order'),h('save',str(out/'saved'/f'{name}.prp')),{'action':'dump','name':name+'-after'},{'action':'snapshot','name':name+'-after'},wait(.3)]
        steps += [{'action':'quit'}]
        (out/'steps.json').write_text(json.dumps({'steps':steps},indent=2)+'\n')
        prepared={'app':str(app),'appSha256':digest(exe),'cloneManifestSha256':digest(clone_path),'source':str(source),'sourceHashes':hashes(source),'sourceProp':str(prop),'sourcePropSha256':digest(prop),'fixtureSha256':digest(out/'fixture.prp'),'original':original,'cases':CASES,'helperSha256':digest(helper),'runnerSha256':digest(Path(__file__)),'helperSourceSha256':digest(ROOT/'scripts/foreground_prop_drag_edges.swift'),'steps':len(steps)-1,'stepsSha256':digest(out/'steps.json'),'layoutOnly':a.layout_only,'scope':'Actual tree insertion/reverse/kind-compatible cycle drags; no drop or selection hooks; fresh synthetic Branch/Inner fixture; independent resets'}
        (out/'prepared.json').write_text(json.dumps(prepared,indent=2)+'\n')
        print(out/'prepared.json');return
    assert digest(clone_path)==prepared['cloneManifestSha256'] and digest(out/'fixture.prp')==prepared['fixtureSha256']
    report={'passed':False,'scope':prepared['scope'],'exitCode':None,'driverExitCode':None,'forcedCleanup':False}
    ui=out/'ui';process=driver=None
    if a.check_only:
        report=json.loads((out/'result.json').read_text())
    else:
        assert not ui.exists();ui.mkdir();(out/'runtime').mkdir()
        report['ownerPreferenceStatesBefore']=owner_states()
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
        env.update(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(ui),VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
        try:
            with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as fg:
                process=subprocess.Popen([str(exe),'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                report['pid']=process.pid
                if not a.layout_only: driver=subprocess.Popen([str(helper),str(process.pid),str(out),str(exe)],stdout=fg,stderr=subprocess.STDOUT)
                deadline=time.monotonic()+120
                while process.poll() is None and time.monotonic()<deadline:
                    if driver and driver.poll() is not None and driver.returncode!=0: raise RuntimeError('Foreground driver failed; stopping owned app')
                    time.sleep(.1)
                assert process.poll() is not None,'Owned app deadline'
                report['exitCode']=process.returncode
                if driver: report['driverExitCode']=driver.wait(timeout=8)
        except BaseException as error: report['error']=repr(error)
        finally:
            if process and process.poll() is None:
                report['forcedCleanup']=True;os.killpg(process.pid,signal.SIGTERM)
                try: process.wait(timeout=3)
                except subprocess.TimeoutExpired: os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
            if driver and driver.poll() is None:
                driver.terminate()
                try: driver.wait(timeout=5)
                except subprocess.TimeoutExpired: driver.kill();driver.wait(timeout=3)
            report['ownerPreferenceStatesAfter']=owner_states()
            (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    try:
        rows=json.loads((ui/'result.json').read_text())
        assert report['exitCode']==0 and not report['forcedCleanup'] and len(rows)==prepared['steps'] and all(r['ok'] for r in rows)
        assert hashes(source)==prepared['sourceHashes'] and digest(prop)==prepared['sourcePropSha256']
        assert digest(exe)==prepared['appSha256'] and digest(out/'fixture.prp')==prepared['fixtureSha256']
        assert report['ownerPreferenceStatesBefore']==report['ownerPreferenceStatesAfter']
        # UI_TEST finish calls exit(0), so this suite does not test normal shutdown
        # or Rust Drop-based lock removal. Keep its isolated lock as evidence.
        report['profileLockRemained']=(out/'profile/.lock').exists()
        report['normalQuitVerified']=False
        report['sourceUnchanged']=True
        if a.layout_only: report.update(passed=True,acceptance=False)
        else:
            fg=json.loads((out/'foreground.json').read_text());report['foreground']=fg
            assert report['driverExitCode']==0 and fg['passed'] and fg['targetExited'] and fg['restoredPrevious']
            assert len(fg['drags'])==len(CASES)
            states=[r['detail'] for r in rows if r.get('detail','').startswith('name=Prop Drag Verification dirty=')]
            assert len(states)==2*len(CASES),states
            checked=[]
            for index,case in enumerate(CASES):
                assert 'dirty=false ' in states[index*2],(case['id'],states[index*2])
                dirty=bool(case.get('graph'))
                assert f'dirty={str(dirty).lower()} ' in states[index*2+1],(case['id'],states[index*2+1])
                saved=out/'saved'/f"{case['id']}.prp"
                got=canonical(package(saved));want=canonical(expected(prepared['original'],case))
                assert got==want,(case['id'],got,want)
                checked.append({'case':case['id'],'sha256':digest(saved),'exactPackageContent':True})
            report.update(passed=True,acceptance=True,checks=checked,documentStates=states)
    except BaseException as error: report['error']=repr(error);raise
    finally: (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':report['passed'],'result':str(out/'result.json')}))


if __name__=='__main__':main()
