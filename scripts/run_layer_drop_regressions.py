#!/usr/bin/env python3
"""Prepare/run the two bounded Color and Layer fix regressions; no automatic retry."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time
import sys
import hashlib

# This earlier runtime stays immutable for the retained v5 evidence.
from run_library_system_foreground_batch import (
    ROOT, digest, continuation_evidence, desktop_snapshot, assert_no_owned_processes,
)


def owned_inventory(stage, deadline):
    remaining = deadline-time.monotonic()
    assert remaining > 0, 'Cleanup observation deadline reached'
    rows = subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='], text=True, timeout=min(1,remaining))
    wanted = set(stage['ownedExecutables'])
    found = []
    for line in rows.splitlines():
        parts = line.strip().split(None,1)
        if len(parts) == 2 and parts[1] in wanted:
            pid = int(parts[0]); assert pid > 1
            found.append((pid, parts[1]))
    return found


def cleanup_owned(stage, deadline):
    """Cover helpers/hosts that establish their own process groups.

    Prelaunch inventory proved these unique frozen private executables absent.
    Exact executable observation is a fallback if interrupted child metadata was
    never published. Recheck each identity immediately before every signal.
    """
    observations = []
    for number, sig in enumerate([signal.SIGTERM, signal.SIGKILL]):
        found = owned_inventory(stage,deadline)
        for pid, executable in found:
            if (pid,executable) not in owned_inventory(stage,deadline): continue
            try:
                own_group = os.getpgid(pid) == pid
                if own_group: os.killpg(pid,sig)
                else: os.kill(pid,sig)
                observations.append({'pid':pid,'executable':executable,'signal':sig.name,
                    'processGroup':own_group,'identitySource':'exact private executable inventory; absent before launch'})
            except ProcessLookupError:
                observations.append({'pid':pid,'action':'already exited before signal'})
        until = min(deadline, time.monotonic()+(1.5 if number == 0 else .5))
        while time.monotonic() < until:
            if not owned_inventory(stage,deadline): return observations
            time.sleep(min(.05,max(0,until-time.monotonic())))
    remaining = owned_inventory(stage,deadline)
    assert not remaining, 'Owned process cleanup incomplete: '+repr(remaining)
    return observations


def run_main():
    began = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,required=True)
    parser.add_argument('--run-prepared',action='store_true',required=True)
    args=parser.parse_args();out=args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not (out/'result.json').exists()
    m=json.loads((out/'prepared.json').read_text())
    deadline=began+m['absoluteDeadlineSeconds']
    assert m['requestedIdleSeconds'] == m['absoluteDeadlineSeconds']
    assert sum(s['timeoutSeconds'] for s in m['stages'])+20 < m['absoluteDeadlineSeconds']
    assert all(s['diagnosticContinuationAllowed'] is False for s in m['stages'])
    assert all(not (Path(s['out'])/'result.json').exists() for s in m['stages'])
    report={'passed':False,'stages':[],'requestedIdleSeconds':m['requestedIdleSeconds']}
    try:
        assert all(digest(p)==v for p,v in m['hashes'].items()), 'Frozen input changed'
        previous=desktop_snapshot(m,out,'initial',deadline);report['previousApplication']=previous
        assert 'securityagent' not in previous['bundleIdentifier'].lower()
        for stage in m['stages']:
            item={'name':stage['name'],'command':stage['command'],'timeoutSeconds':stage['timeoutSeconds']}
            report['stages'].append(item);started=time.monotonic();process=None
            try:
                assert deadline-time.monotonic() >= stage['timeoutSeconds']+20, 'No full watchdog/cleanup budget'
                assert all(digest(p)==v for p,v in m['hashes'].items()), 'Frozen input changed between stages'
                assert desktop_snapshot(m,out,stage['name']+'-before',deadline)==previous, 'Desktop ownership changed; no launch'
                item['processCheckBefore']=assert_no_owned_processes(stage,deadline)
                # Hashing and native observations consume time. Never renew the
                # watchdog after those checks without proving the remaining budget.
                assert deadline-time.monotonic() >= stage['timeoutSeconds']+20, 'Prelaunch checks consumed watchdog/cleanup budget; no launch'
                with (out/(stage['name']+'.log')).open('w') as log:
                    process=subprocess.Popen(stage['command'],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                    item['runnerPID']=process.pid
                    code=process.wait(timeout=min(stage['timeoutSeconds'],max(.001,deadline-time.monotonic()-20)))
                item['exitCode']=code
                result=Path(stage['out'])/'result.json'
                assert result.exists(), 'Stage result missing'
                data=json.loads(result.read_text());item.update(result=str(result),resultSha256=digest(result),passed=code==0 and data['passed'])
                fg=None
                if stage['family']:
                    fg=json.loads((Path(stage['out'])/'foreground.json').read_text())
                    assert data.get('exitCode') == 0 and data.get('driverExitCode') == 0
                item['continuation']=continuation_evidence(stage,data,fg)
                item['processCheckAfter']=assert_no_owned_processes(stage,deadline)
                item['desktopAfter']=desktop_snapshot(m,out,stage['name']+'-after',deadline)
                assert item['desktopAfter']==previous, 'Desktop ownership changed after stage'
                assert all(digest(p)==v for p,v in m['hashes'].items()), 'Frozen input changed during stage'
                assert item['passed'] and item['continuation']['safeToContinue'], 'Stage failed or cleanup proof incomplete; later input withheld'
            except BaseException:
                try:
                    if process and process.poll() is None:
                        try: os.killpg(process.pid,signal.SIGTERM)
                        except ProcessLookupError: pass
                        try: process.wait(timeout=min(2,max(.001,deadline-time.monotonic())))
                        except subprocess.TimeoutExpired:
                            try: os.killpg(process.pid,signal.SIGKILL)
                            except ProcessLookupError: pass
                            process.wait(timeout=min(2,max(.001,deadline-time.monotonic())))
                except BaseException as error:
                    item['runnerCleanupError']=repr(error)
                # A parent-exit race must not strand separately grouped children.
                if process:
                    try: item['ownedCleanup']=cleanup_owned(stage,deadline)
                    except BaseException as error: item['cleanupError']=repr(error)
                raise
            finally:
                item['elapsedSeconds']=time.monotonic()-started
                report['elapsedSeconds']=time.monotonic()-began
                (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        report['passed']=True
    except BaseException as error:
        report['error']=repr(error)
        raise
    finally:
        report['elapsedSeconds']=time.monotonic()-began
        report['withinRequestedIdleInterval']=time.monotonic() <= deadline
        if not report['withinRequestedIdleInterval']: report['passed']=False
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'stages':len(report['stages']),'elapsedSeconds':report['elapsedSeconds']}))




def prepare(args):
    out=args.out.resolve();assert out.is_relative_to(ROOT/'target') and not out.exists()
    color=args.color_out.resolve() if args.color_out else None;layer=args.layer_out.resolve()
    stages=[];pins={}
    def pin(path, expected=None):
        path=Path(path).resolve();assert path.is_relative_to(ROOT)
        actual=digest(path)
        assert expected is None or actual==expected, 'Stale preparation: '+str(path)
        pins[str(path)]=actual
    for file in ['run_layer_drop_regressions.py','run_library_system_foreground_batch.py','run_native_foreground_queue.py',
                 'test_library_family_foreground.py','foreground_library_family.swift',
                 'test_layer_drag_regression.py','test_layer_drag_foreground.py','foreground_layer_drag_regression.swift']:
        pin(ROOT/'scripts'/file)
    jobs=([(color,True,'color',75)] if color else [])+[(layer,False,'layers',115)]
    for directory,family,name,seconds in jobs:
        assert directory.is_relative_to(ROOT/'target') and not (directory/'result.json').exists() and not (directory/'ui').exists()
        path=directory/'prepared.json';m=json.loads(path.read_text());pin(path)
        if family:
            assert m['kind']=='color' and m['steps']==29
            pin(ROOT/'scripts/test_library_family_foreground.py',m['runnerSha256'])
            pin(ROOT/'scripts/foreground_library_family.swift',m['helperSourceSha256'])
            exe=Path(m['app'])/'Contents/MacOS/Vixen'
            pin(exe,m['appSha256']);pin(m['helper'],m['helperSha256'])
            pin(directory/'calibration.json',m['calibrationSha256']);pin(directory/'steps.json',m['stepsSha256'])
            cal=json.loads((directory/'calibration.json').read_text());pin(cal['referenceImage'],cal['referenceImageSha256'])
            pin(Path(m['app'])/'Contents/Info.plist');pin(Path(m['app']).parent/'private-app.json')
            command=[sys.executable,'-B',str(ROOT/'scripts/test_library_family_foreground.py'),'--kind','color',
                     '--out',str(directory),'--helper',m['helper'],'--app',m['app'],'--run-prepared']
        else:
            assert m['steps']==59 and m['allInDeadlineSeconds']==110
            for file,value in m['frozenHashes'].items():pin(file,value)
            exe=Path(m['app'])
            command=[sys.executable,'-B',str(ROOT/'scripts/test_layer_drag_regression.py'),'--out',str(directory),'--run-prepared']
        for rel,value in m['sourceHashes'].items():pin(Path(m['source'])/rel,value)
        stages.append(dict(name=name,family=family,diagnosticContinuationAllowed=False,out=str(directory),timeoutSeconds=seconds,
            ownedExecutables=[str(exe),m['helper']],command=command))
    assert len({s['ownedExecutables'][0] for s in stages})==1, 'Suites target different candidates'
    if args.mapper_out:
        directory=args.mapper_out.resolve()
        assert directory.is_relative_to(ROOT/'target') and not (directory/'result.json').exists()
        path=directory/'prepared.json';m=json.loads(path.read_text());pin(path)
        assert m['stageName']=='mapper-file-panels' and m['timeoutSeconds']==110 and m['totalBoundSeconds']==110
        expected=[sys.executable,'-B',str(ROOT/'scripts/check_mapper_file_panels.py'),'--out',str(directory),'--run-prepared']
        assert m['command']==expected, 'Unexpected Mapper command'
        assert m['ownedExecutables']==[m['executable'],m['driver']]
        assert all(Path(v).is_relative_to(directory) for v in m['ownedExecutables'])
        for file,value in m['pinnedInputHashes'].items():pin(file,value)
        pin(m['executable'],m['executableSha256']);pin(m['driver'],m['driverSha256'])
        pin(ROOT/'scripts/check_mapper_file_panels.py')
        stages.append(dict(name='mapper-file-panels',family=False,diagnosticContinuationAllowed=False,out=str(directory),timeoutSeconds=110,
            ownedExecutables=m['ownedExecutables'],command=expected))
    if args.name_out:
        directory=args.name_out.resolve();assert directory.is_relative_to(ROOT/'target') and not(directory/'result.json').exists()
        path=directory/'prepared.json';m=json.loads(path.read_text());pin(path)
        assert m['steps']==40 and m['timeoutSeconds']==55 and m['inputDeadlineSeconds']==30
        assert m['executable']==stages[0]['ownedExecutables'][0]
        for file,value in m['pins'].items():pin(file,value)
        for rel,value in m['sourceHashes'].items():pin(Path(m['source'])/rel,value)
        command=[sys.executable,'-B',str(ROOT/'scripts/test_layer_name_tab_foreground.py'),'--out',str(directory),
                 '--app',m['app'],'--helper',m['helper'],'--profile',m['source'],'--run-prepared']
        stages.append(dict(name='layer-name-tab',family=False,diagnosticContinuationAllowed=False,out=str(directory),timeoutSeconds=60,
            ownedExecutables=[m['executable'],m['helper']],command=command))
    desktop=ROOT/'target/system-panels-ownership-release-04/system-panel-driver'
    pin(desktop)
    duration=sum(s['timeoutSeconds'] for s in stages)
    seconds=360 if duration+20 >=240 else 240
    m=dict(stages=stages,hashes=pins,desktopDriver=str(desktop),requestedIdleSeconds=seconds,absoluteDeadlineSeconds=seconds,
        cleanupReserveSeconds=20,stageWatchdogSumSeconds=sum(s['timeoutSeconds'] for s in stages),launchPerformed=False,
        scope='Candidate afb actual Layer59 steps/five OS drags with optional Color29, independent Mapper panels or40-step layer-name/tab suite; exact frozen commands/settings isolation; anyfailurewithholdslaterinput')
    assert sum(s['timeoutSeconds'] for s in stages)+20 < m['absoluteDeadlineSeconds']
    out.mkdir();(out/'prepared.json').write_text(json.dumps(m,indent=2)+'\n')
    print(json.dumps({'prepared':str(out/'prepared.json'),'frozenInputs':len(pins),'launched':False}))

def main():
    if '--prepare-only' not in sys.argv:return run_main()
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--prepare-only',action='store_true',required=True)
    p.add_argument('--out',type=Path,required=True);p.add_argument('--color-out',type=Path);p.add_argument('--layer-out',type=Path,required=True)
    p.add_argument('--mapper-out',type=Path)
    p.add_argument('--name-out',type=Path)
    prepare(p.parse_args())

if __name__=='__main__':main()
