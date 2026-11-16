#!/usr/bin/env python3
"""Run a frozen queue only within its explicitly coordinated desktop interval."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time

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


def main():
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
    assert all(not s['family'] and s['diagnosticContinuationAllowed'] is False for s in m['stages'])
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
                item['continuation']=continuation_evidence(stage,data)
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


if __name__=='__main__':main()
