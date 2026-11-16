#!/usr/bin/env python3
"""Sequential owned desktop checks; run only for the prepared authorized interval.

Preparation writes exact commands and hashes. One absolute budget covers all stages.
A stage is refused unless its entire watchdog plus cleanup reserve fits. A narrowly
classified diagnostic failure may release the desktop for the next independent
stage; ownership, timeout or cleanup failures always stop the queue.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()


def continuation_evidence(stage, result, foreground=None):
    """Pure gate; caller additionally checks live desktop/processes and frozen pins."""
    reasons = []
    if not result.get('passed') and stage.get('diagnosticContinuationAllowed') is False:
        reasons.append('this suite has no diagnostic continuation policy')
    if not result.get('ownerPreferencesUnchanged'): reasons.append('owner preferences not verified')
    if stage['family']:
        fg = foreground or {}
        for key in ['targetExited', 'restoredPrevious']:
            if fg.get(key) is not True: reasons.append(key+' not verified')
        for key in ['heldButtonAfterCleanup', 'heldKeyAfterCleanup']:
            if fg.get(key) is not False: reasons.append(key+' not cleared')
        for key in ['sourceUnchanged', 'appUnchanged']:
            if result.get(key) is not True: reasons.append(key+' not verified')
        if not result.get('passed'):
            if fg.get('failurePhase') != 'calibration_preinput': reasons.append('failure is outside calibration preflight')
            if fg.get('failureKind') != 'calibration_rejected': reasons.append('failure is not a classified raster diagnostic')
            for key in ['pointerInputPosted', 'keyboardInputPosted']:
                if fg.get(key) is not False: reasons.append(key+' not ruled out')
    else:
        for key in ['processExited', 'cleanupCompleted', 'noInputHeld', 'restoredPrevious', 'inputsUnchanged', 'appUnchanged', 'driverUnchanged']:
            if result.get(key) is not True: reasons.append(key+' not verified')
        if not result.get('passed'):
            if result.get('failurePhase') != 'rgb_control_readiness' or result.get('failureKind') != 'diagnostic':
                reasons.append('failure is not a classified RGB readiness diagnostic')
            if result.get('inputOwnershipMaintained') is not True: reasons.append('input ownership not verified')
    return {'safeToContinue': not reasons, 'reasons': reasons}


def desktop_snapshot(manifest, out, label, deadline):
    """Read-only native observation; this action never activates or posts input."""
    remaining = deadline-time.monotonic()
    assert remaining > 0, 'Absolute deadline reached before desktop check'
    config = out/(label+'-desktop-config.json')
    output = out/(label+'-desktop.json')
    config.write_text(json.dumps({'output':str(output)})+'\n')
    subprocess.run([manifest['desktopDriver'], str(config), 'desktop'], check=True,
                   capture_output=True, timeout=min(3,remaining))
    application = json.loads(output.read_text())['application']
    assert type(application['pid']) is int and application['pid'] > 1
    assert application['bundleIdentifier'] and application['executable']
    return application


def assert_no_owned_processes(stage, deadline):
    remaining = deadline-time.monotonic()
    assert remaining > 0, 'Absolute deadline reached before process check'
    observed = subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='], text=True, timeout=min(2,remaining))
    wanted = set(stage['ownedExecutables'])
    matches = []
    for line in observed.splitlines():
        fields = line.strip().split(None,1)
        if len(fields) == 2 and fields[1] in wanted: matches.append(line.strip())
    assert not matches, 'Owned verification process remains: '+repr(matches)
    return {'ownedExecutables':sorted(wanted), 'matchingProcesses':matches}


def cleanup_family(directory):
    path = directory/'owned-processes.json'
    if not path.exists(): return []
    state = json.loads(path.read_text())
    prepared = json.loads((directory/'prepared.json').read_text())
    expected = {'app': str(Path(prepared['app'])/'Contents/MacOS/Vixen'), 'driver': prepared['helper']}
    observations = []
    for key in ['app', 'driver']:
        if key+'PID' not in state: continue
        pid, executable = state[key+'PID'], state[key+'Executable']
        assert type(pid) is int and pid > 1 and executable == expected[key], 'Owned cleanup metadata differs from frozen private plan'
        current = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'comm='], capture_output=True, text=True, timeout=1)
        if current.returncode == 0 and current.stdout.strip() == executable:
            group = key == 'app' and state.get('appProcessGroup') == pid and os.getpgid(pid) == pid
            if group: os.killpg(pid, signal.SIGTERM)
            else: os.kill(pid, signal.SIGTERM)
            observations.append({'pid':pid, 'executable':executable, 'action':'SIGTERM exact owned group' if group else 'SIGTERM exact owned executable'})
            until = time.monotonic()+1
            while time.monotonic() < until:
                current = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'comm='], capture_output=True, text=True, timeout=1)
                if current.returncode != 0 or current.stdout.strip() != executable: break
                time.sleep(.05)
            else:
                current = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'comm='], capture_output=True, text=True, timeout=1)
                if current.returncode == 0 and current.stdout.strip() == executable:
                    if group and os.getpgid(pid) == pid: os.killpg(pid, signal.SIGKILL)
                    else: os.kill(pid, signal.SIGKILL)
                    observations.append({'pid':pid, 'action':'SIGKILL after bounded owned-executable recheck'})
        else: observations.append({'pid':pid, 'action':'not running at expected owned executable; no signal'})
    return observations


def main():
    began = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--color-out', type=Path, default=ROOT/'target/library-color-foreground-current-v5')
    parser.add_argument('--gradient-out', type=Path, default=ROOT/'target/library-gradient-foreground-current-v5')
    parser.add_argument('--system-out', type=Path, default=ROOT/'target/system-panels-ownership-release-04')
    parser.add_argument('--extra-plan', type=Path, action='append', default=[], help='Reviewed independent suite manifest; preparation only')
    parser.add_argument('--access-preflight', type=Path, default=ROOT/'target/library-family-calibration-v5/access-preflight.json')
    parser.add_argument('--seconds', type=int, default=240, choices=range(240,901))
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare-only', action='store_true')
    mode.add_argument('--run-prepared', action='store_true')
    args = parser.parse_args(); out = args.out.resolve()
    assert out.is_relative_to(ROOT/'target')
    if args.prepare_only:
        assert not out.exists(); out.mkdir()
        stages = []
        files = [Path(__file__).resolve(), ROOT/'scripts/test_library_family_foreground.py',
                 ROOT/'scripts/foreground_library_family.swift', ROOT/'scripts/check_system_panels.py']
        for kind in ['color', 'gradient']:
            folder = getattr(args,kind+'_out').resolve()
            assert folder.is_relative_to(ROOT/'target') and not (folder/'result.json').exists()
            manifest = folder/'prepared.json'; m = json.loads(manifest.read_text())
            files += [manifest, folder/'calibration.json', folder/'steps.json', Path(m['helper'])]
            stages.append(dict(name=kind, family=True, diagnosticContinuationAllowed=True, out=str(folder), timeoutSeconds=75,
                ownedExecutables=[str(Path(m['app'])/'Contents/MacOS/Vixen'), m['helper']],
                command=[sys.executable, '-B', str(ROOT/'scripts/test_library_family_foreground.py'), '--kind', kind,
                    '--out', str(folder), '--helper', m['helper'], '--app', m['app'], '--run-prepared']))
        folder = args.system_out.resolve()
        assert folder.is_relative_to(ROOT/'target') and not (folder/'result.json').exists()
        manifest = folder/'prepared.json'; m = json.loads(manifest.read_text())
        files += [manifest, Path(m['executable']), Path(m['driver'])]
        stages.append(dict(name='system-panels', family=False, diagnosticContinuationAllowed=True, out=str(folder), timeoutSeconds=60,
            ownedExecutables=[m['executable'],m['driver']],
            command=[sys.executable, '-B', str(ROOT/'scripts/check_system_panels.py'), '--build', m['build'],
                     '--configuration', 'release', '--out', str(folder), '--ownership', '--run-prepared']))
        desktop_driver=m['driver']
        for extra in args.extra_plan:
            extra=extra.resolve()
            assert extra.is_relative_to(ROOT/'target')
            extra_manifest=json.loads(extra.read_text())
            folder=extra.parent
            assert not (folder/'result.json').exists()
            name=extra_manifest['stageName']
            assert name.replace('-','').isalnum() and name not in [s['name'] for s in stages]
            timeout=extra_manifest['timeoutSeconds']
            assert type(timeout) is int and 0 < timeout <= 75
            command=extra_manifest['command']
            assert isinstance(command,list) and command and all(isinstance(x,str) for x in command)
            # No shell is used. Extra suites are explicitly prepared/reviewed local commands.
            stages.append(dict(name=name,family=False,diagnosticContinuationAllowed=False,out=str(folder),timeoutSeconds=timeout,
                ownedExecutables=extra_manifest['ownedExecutables'],command=command))
            files += [extra, *map(Path,extra_manifest['pinnedInputHashes']),
                      *map(Path,extra_manifest['ownedExecutables'])]
        preflight = args.access_preflight.resolve()
        access = json.loads(preflight.read_text())
        assert access['accessibilityTrusted'] and access['eventPostingPreflight'] and not access['permissionRequested'] and not access['inputPosted']
        files.append(preflight)
        prepared = dict(stages=stages, hashes={str(p):digest(p) for p in set(files)},
            desktopDriver=desktop_driver,
            requestedIdleSeconds=args.seconds, absoluteDeadlineSeconds=args.seconds, cleanupReserveSeconds=20,
            stageWatchdogSumSeconds=sum(s['timeoutSeconds'] for s in stages),
            familyInputDeadlineSeconds=40, familyGuardedCleanupDeadlineSeconds=48,
            familyConservativeRunnerSeconds=67, systemInputAndCleanupSeconds=44,
            launchPerformed=False, scope='Actual Color/Gradient OS pointer moves/copy/name followed by real system Color/Font AX input. Muted private library profiles; system host opens no Core/show. No production edits.')
        assert prepared['stageWatchdogSumSeconds']+prepared['cleanupReserveSeconds'] < args.seconds
        (out/'prepared.json').write_text(json.dumps(prepared, indent=2)+'\n')
        print(json.dumps(dict(prepared=str(out/'prepared.json'), launched=False, requestedIdleSeconds=args.seconds)))
        return
    m = json.loads((out/'prepared.json').read_text())
    deadline = began+m['absoluteDeadlineSeconds']
    assert all(digest(p) == wanted for p,wanted in m['hashes'].items())
    assert not (out/'result.json').exists()
    assert all(not (Path(s['out'])/'result.json').exists() for s in m['stages'])
    report = dict(passed=False, requestedIdleSeconds=m['requestedIdleSeconds'], stages=[], verificationFailures=[])
    try:
        previous = desktop_snapshot(m,out,'initial',deadline)
        report['previousApplication'] = previous
        assert 'securityagent' not in previous['bundleIdentifier'].lower(), 'System authorization owns desktop'
        for stage in m['stages']:
            assert deadline-time.monotonic() >= stage['timeoutSeconds']+20, 'Insufficient absolute budget for next full stage; no launch'
            assert all(digest(p) == wanted for p,wanted in m['hashes'].items()), 'Frozen inputs changed between stages'
            assert desktop_snapshot(m,out,stage['name']+'-before',deadline) == previous, 'Desktop ownership changed between stages; no launch'
            assert_no_owned_processes(stage,deadline)
            started = time.monotonic(); process = None
            item = dict(name=stage['name'], command=stage['command'], timeoutSeconds=stage['timeoutSeconds'])
            report['stages'].append(item)
            try:
                with (out/(stage['name']+'.log')).open('w') as log:
                    process = subprocess.Popen(stage['command'], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                    code = process.wait(timeout=stage['timeoutSeconds'])
                item['exitCode'] = code
                result = Path(stage['out'])/'result.json'
                if result.exists():
                    item['result'] = str(result); item['resultSha256'] = digest(result)
                    data = json.loads(result.read_text()); item['passed'] = code == 0 and data['passed']
                    assert not data['passed'] or code == 0, 'Successful report conflicts with failing runner exit'
                else:
                    raise AssertionError('Stage result missing; later desktop checks withheld')
                fg_path=Path(stage['out'])/'foreground.json'
                fg=json.loads(fg_path.read_text()) if stage['family'] and fg_path.exists() else None
                item['continuation'] = continuation_evidence(stage,data,fg)
                item['processCheck'] = assert_no_owned_processes(stage,deadline)
                item['desktopAfter'] = desktop_snapshot(m,out,stage['name']+'-after',deadline)
                assert item['desktopAfter'] == previous, 'Desktop changed after stage; later checks withheld'
                assert all(digest(p) == wanted for p,wanted in m['hashes'].items()), 'Frozen inputs changed during stage'
                assert item['continuation']['safeToContinue'], 'Cleanup/ownership proof incomplete; later checks withheld'
                if not item['passed']:
                    report['verificationFailures'].append(stage['name'])
                    item['continuedAfterDiagnosticFailure'] = True
            except BaseException:
                if process and process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try: process.wait(timeout=3)
                    except subprocess.TimeoutExpired: os.killpg(process.pid, signal.SIGKILL); process.wait(timeout=3)
                if stage['family']: item['ownedCleanup'] = cleanup_family(Path(stage['out']))
                raise
            finally:
                item['elapsedSeconds'] = time.monotonic()-started
                report['elapsedSeconds'] = time.monotonic()-began
                (out/'result.json').write_text(json.dumps(report, indent=2)+'\n')
        report['passed'] = not report['verificationFailures']
        report['allStagesAttempted'] = True
    except BaseException as error:
        report['error'] = repr(error)
        raise
    finally:
        report['elapsedSeconds'] = time.monotonic()-began
        report['withinRequestedIdleInterval'] = time.monotonic() <= deadline
        if not report['withinRequestedIdleInterval']: report['passed'] = False
        (out/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(passed=report['passed'], stages=len(report['stages']), elapsedSeconds=report['elapsedSeconds'])))
    if not report['passed']: raise SystemExit(1)


if __name__ == '__main__': main()
