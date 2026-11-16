#!/usr/bin/env python3
"""Run only the prepared offscreen two-window layout observation after root GO.

No WindowServer input helper is launched. This is not foreground acceptance.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
tree = lambda p: {str(f.relative_to(p)):sha(f) for f in sorted(p.rglob('*')) if f.is_file()}

def prefs():
    paths = [Path.home()/'Library/Preferences/com.vixenlights.vixen3.mac.plist',Path.home()/'Library/Application Support/Vixen/Settings.xml']
    return {str(p):sha(p) if p.exists() else None for p in paths}

def front():
    value=subprocess.check_output(['/usr/bin/lsappinfo','front'],text=True,timeout=3).strip()
    assert value.startswith('ASN:'), 'Cannot observe foreground ASN'
    return value

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);args=p.parse_args()
    out=args.out.resolve();assert out.is_relative_to(ROOT/'target') and not (out/'ui').exists()
    m=json.loads((out/'prepared.json').read_text());app=Path(m['app']);exe=app/'Contents/MacOS/Vixen';source=Path(m['source'])
    assert m['foregroundReady'] is False and sha(exe)==m['appSha256']
    assert sha(out/'layout-steps.json')==m['layoutStepsSha256'] and tree(out/'profile')==m['preparedProfileHashes']
    assert all(sha(path)==digest for path,digest in m['reviewedSourcePins'].items())
    processes=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=3)
    assert not any(line.strip().endswith('/Contents/MacOS/Vixen') for line in processes.splitlines()), 'Another Vixen is running'
    before=prefs();previous=front();(out/'ui').mkdir();(out/'runtime').mkdir()
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(m['environment'])
    started=time.monotonic();process=None
    result=dict(passed=False,acceptance=False,scope='Actual current app, two private sequencer models, retained offscreen per-window layouts and state only; no physical input or foreground drag acceptance.',foregroundASNBefore=previous,ownerPreferenceHashesBefore=before,appSha256=m['appSha256'],sourceExecutableSha256=m['sourceExecutableSha256'])
    try:
        with (out/'process.log').open('w') as log:
            process=subprocess.Popen([str(exe)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            (out/'owned-process.json').write_text(json.dumps({'pid':process.pid,'executable':str(exe),'processGroup':process.pid})+'\n')
            process.wait(timeout=35)
        result['exitCode']=process.returncode
        rows=json.loads((out/'ui/result.json').read_text());result['steps']=len(rows)
        assert process.returncode==0 and len(rows)==m['layoutSteps'] and all(r['ok'] for r in rows)
        states=[r['detail'] for r in rows if r.get('detail','').startswith('speed=')]
        assert len(states)==2 and all('running=false' in state for state in states)
        focuses=[json.loads(r['detail']) for r in rows if r.get('detail','').startswith('{') and '"captured"' in r.get('detail','')]
        assert len(focuses)==2 and [v['captured']['title'] for v in focuses]==['Source Library','Destination Library']
        assert focuses[0]['captured']['identity']!=focuses[1]['captured']['identity']
        result['focusStates']=focuses
        files=[out/'ui'/(label+suffix) for label in ['source','destination'] for suffix in ['-ready.png','-controls.txt','-menus.txt','-before-toolbar.txt']]
        assert all(f.is_file() for f in files)
        result.update(passed=True,transportStopped=True,retainedFiles={str(f):sha(f) for f in files})
    except BaseException as e:
        result['error']=repr(e)
        raise
    finally:
        if process and process.poll() is None:
            os.killpg(process.pid,signal.SIGTERM)
            try:process.wait(timeout=3)
            except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
        result.update(elapsedSeconds=time.monotonic()-started,ownedProcessExited=process is not None and process.poll() is not None,sourceUnchanged=tree(source)==m['sourceHashes'],appUnchanged=sha(exe)==m['appSha256'],ownerPreferencesUnchanged=prefs()==before)
        try:result['foregroundASNAfter']=front();result['foregroundUnchanged']=result['foregroundASNAfter']==previous
        except BaseException as e:result['foregroundUnchanged']=False;result['foregroundObservationError']=repr(e)
        if not all(result[k] for k in ['ownedProcessExited','sourceUnchanged','appUnchanged','ownerPreferencesUnchanged','foregroundUnchanged']):result['passed']=False
        (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    assert result['passed'],result
    print(json.dumps({'passed':True,'steps':result['steps'],'acceptance':False}))

if __name__=='__main__':main()
