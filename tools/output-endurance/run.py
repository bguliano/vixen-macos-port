#!/usr/bin/env python3
"""Prepare or execute the bounded, private, loopback-only twenty-minute runner."""
import argparse, hashlib, json, os, signal, stat, subprocess, sys, time, threading
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
INPUTS=ROOT/'target/package-unicode-integration/build-inputs.json'
BINARY=ROOT/'target/output-endurance-cargo/debug/vixen-output-endurance'

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):
    q=p.with_suffix(p.suffix+'.tmp'); q.write_text(json.dumps(v,indent=2)+'\n'); q.replace(p)
def tree(p):
    result={}
    for f in sorted(p.rglob('*')):
        if f.is_symlink(): raise ValueError('Symlink in private fixture: '+str(f))
        if f.is_file(): result[str(f.relative_to(p))]=sha(f)
    return result
def inputs():
    m=json.loads(INPUTS.read_text()); assert len(m['sources'])==811
    assert all(sha(ROOT/p)==h for p,h in m['sources'].items())
    return m['sources']
def environment(out):
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(VIXEN_SETTINGS_DIR=str(out/'settings'),TMPDIR=str(out/'runtime')+'/',RUST_BACKTRACE='1')
    return env

def prepare(out):
    assert out.is_absolute() and not out.exists(); out.mkdir()
    (out/'settings').mkdir();(out/'runtime').mkdir(); src=inputs()
    exe=out/'vixen-output-endurance';exe.write_bytes(BINARY.read_bytes());exe.chmod(0o555)
    with (out/'prepare.log').open('w') as log:
        subprocess.run([str(exe),'prepare',str(out)],env=environment(out),stdout=log,stderr=subprocess.STDOUT,timeout=60,check=True)
    assert tree(out/'settings')=={}
    controls=subprocess.run([str(exe),'controls'],capture_output=True,text=True,timeout=10,check=True)
    c=json.loads(controls.stdout);assert c==dict(passed=True,checks=6,pureOnly=True);save(out/'pure-controls.json',c)
    support=[ROOT/'tools/output-endurance'/p for p in ['Cargo.toml','Cargo.lock','src/main.rs','run.py','check_parent.py']]
    save(out/'prepared.json',dict(status='prepared-not-executed',durationSeconds=1200,childIndependentDeadlineSeconds=1240,parentLimitSeconds=1280,productionInputs=src,inputManifestSha256=sha(INPUTS),binary=str(exe),pins={str(p):sha(p) for p in support+[exe,INPUTS]},profileHashes=tree(out/'profile'),settingsHashes={},native=False,loopbackOnly=True,hardwareVerified=False))
    print(json.dumps(dict(prepared=str(out/'prepared.json'),profileFiles=len(tree(out/'profile')))))

def validate(out,m):
    assert m['status']=='prepared-not-executed' and m['durationSeconds']==1200 and m['parentLimitSeconds']==1280 and m['childIndependentDeadlineSeconds']==1240
    assert m['productionInputs']==inputs() and sha(INPUTS)==m['inputManifestSha256']
    assert all(sha(p)==h for p,h in m['pins'].items())
    assert Path(m['binary'])==out/'vixen-output-endurance' and not Path(m['binary']).is_symlink() and stat.S_IMODE(Path(m['binary']).stat().st_mode)==0o555
    assert tree(out/'profile')==m['profileHashes'] and tree(out/'settings')=={}
    assert not any((out/p).exists() for p in ['parent.claim','run.claim','result.json','native-result.json','cycles.jsonl'])

def resource_sample(pid,elapsed):
    row=dict(elapsedSeconds=elapsed,diagnosticOnly=True)
    for name,command in [('rssKiB',['/bin/ps','-o','rss=','-p',str(pid)]),('threadListing',['/bin/ps','-M','-p',str(pid)]),('fdListing',['/usr/sbin/lsof','-n','-P','-a','-p',str(pid),'-F','f'])]:
        try:
            p=subprocess.run(command,capture_output=True,text=True,timeout=2)
            if name=='rssKiB':row[name]=int(p.stdout.strip()) if p.returncode==0 else None
            elif name=='threadListing':row['threadRows']=max(0,len(p.stdout.splitlines())-1) if p.returncode==0 else None
            else:row['numericDescriptors']=sum(x[1:].isdigit() for x in p.stdout.splitlines() if x.startswith('f')) if p.returncode==0 else None
            if p.returncode:row[name+'Error']='exit '+str(p.returncode)
        except Exception as e:row[name+'Error']=str(e)
    return row

def cleanup(child,deadline):
    errors=[]
    if child is None:return errors
    try:
        if child.poll() is None:
            os.killpg(child.pid,signal.SIGTERM)
            try:child.wait(timeout=min(4,max(.01,deadline-time.monotonic()-1)))
            except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait(timeout=1)
    except ProcessLookupError:pass
    except Exception as e:errors.append(repr(e))
    return errors

def accept_native(out):
    n=json.loads((out/'native-result.json').read_text());assert n['passed'] is True
    assert 1200<=n['durationSeconds']<1240 and n['cycles']>=100 and n['loopWraps']==n['cycles']
    assert n['engineObjectsDropped'] is True and n['controllerStopped'] is True and n['loopbackOnly'] is True and n['audioOpened'] is False and n['nativeUI'] is False and n['hardwareVerification'] is False
    rows=[json.loads(x) for x in (out/'cycles.jsonl').read_text().splitlines()]
    assert len(rows)==n['cycles']; last=0
    for i,row in enumerate(rows,1):
        assert row['cycle']==i and row['loopWraps']==i and row['errors']==row['panics']==row['namedContexts']==0
        assert row['packets']==row['packetsSent']==row['litPackets']+row['darkPackets'] and row['packets']>last
        assert row['shutdownMs']<5000
        last=row['packets']
    assert last==n['packets'] and rows[-1]['litPackets']==n['litPackets'] and rows[-1]['darkPackets']==n['darkPackets']
    assert not (out/'watchdog.json').exists()
    return n

def deadline_abort(owned):
    child=owned.get('child')
    try:
        if child is not None and child.poll() is None: os.killpg(child.pid,signal.SIGKILL)
    finally: os._exit(124)

cancel_signal=None
def termination_signal(signum,frame):
    global cancel_signal
    cancel_signal=signum

def check_cancel():
    if cancel_signal is not None: raise SystemExit('Received signal '+str(cancel_signal))

def run(out):
    global cancel_signal
    cancel_signal=None
    start=time.monotonic();deadline=start+1280;child=None;m=None;before=None
    owned={}; watchdog=threading.Timer(1280,deadline_abort,args=(owned,));watchdog.daemon=True;watchdog.start()
    previous_sigterm=signal.signal(signal.SIGTERM,termination_signal)
    previous_sigint=signal.signal(signal.SIGINT,termination_signal)
    report=dict(passed=False,scope='Twenty-minute headless production playback/output lifecycle with actual E1.31 packets on127.0.0.1; resource trends diagnostic only; no physical hardware/audio/GUI claim.')
    try:
        m=json.loads((out/'prepared.json').read_text());validate(out,m);before=inputs();check_cancel()
        with (out/'parent.claim').open('x') as f:f.write(str(os.getpid()))
        with (out/'process.log').open('w') as log, (out/'resources.jsonl').open('x') as resources:
            assert deadline-time.monotonic()>1250
            check_cancel()
            child=subprocess.Popen([m['binary'],'run',str(out)],env=environment(out),stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned['child']=child
            check_cancel()
            report.update(pid=child.pid,processGroup=child.pid);save(out/'owned-process.json',dict(pid=child.pid,processGroup=child.pid,executable=m['binary']))
            sample_at=0
            while child.poll() is None:
                check_cancel()
                assert time.monotonic()<deadline-6,'Parent execution deadline'
                if time.monotonic()>=sample_at:
                    row=resource_sample(child.pid,time.monotonic()-start);resources.write(json.dumps(row)+'\n');resources.flush();sample_at=time.monotonic()+30
                    print(json.dumps(dict(running=True,pid=child.pid,elapsedSeconds=round(time.monotonic()-start,1))),flush=True)
                time.sleep(.25)
            report['exitCode']=child.wait(timeout=1);assert report['exitCode']==0
            report['native']=accept_native(out);check_cancel();report['passed']=True
    except BaseException as e:report['error']=repr(e)
    finally:
        errors=cleanup(child,deadline)
        try:
            report['productionInputsUnchanged']=before is not None and inputs()==before
            report['pinsUnchanged']=m is not None and all(sha(p)==h for p,h in m['pins'].items())
            report['profileUnchanged']=m is not None and tree(out/'profile')==m['profileHashes']
            report['settingsUnchanged']=tree(out/'settings')=={}
        except Exception as e:errors.append(repr(e))
        report.update(cleanupErrors=errors,childExited=child is None or child.poll() is not None,elapsedSeconds=time.monotonic()-start,withinDeadline=time.monotonic()<deadline)
        report['passed']=report['passed'] and not errors and all(report.get(k) is True for k in ['productionInputsUnchanged','pinsUnchanged','profileUnchanged','settingsUnchanged','childExited','withinDeadline'])
        save(out/'result.json',report)
        watchdog.cancel();signal.signal(signal.SIGTERM,previous_sigterm);signal.signal(signal.SIGINT,previous_sigint)
    print(json.dumps(dict(passed=report['passed'],elapsedSeconds=report['elapsedSeconds'],error=report.get('error'))));return 0 if report['passed'] else 1

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',required=True,type=Path);p.add_argument('--prepare',action='store_true');a=p.parse_args();out=a.out.resolve()
    if a.prepare:prepare(out)
    else:raise SystemExit(run(out))
