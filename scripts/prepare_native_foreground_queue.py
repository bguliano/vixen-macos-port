#!/usr/bin/env python3
"""Compose reviewed independent suite manifests; never launch apps or post input."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT/'scripts/run_native_foreground_queue.py'
SHARED = ROOT/'scripts/run_library_system_foreground_batch.py'


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def prepare(out, plans, desktop_plan, seconds=240):
    out = out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    assert 1 <= len(plans) <= 8 and 60 <= seconds <= 900
    pins = {str(RUNNER): digest(RUNNER), str(SHARED): digest(SHARED), str(Path(__file__).resolve()): digest(__file__)}
    stages = []
    for plan in plans:
        plan = plan.resolve()
        assert plan.is_relative_to(ROOT/'target')
        m = json.loads(plan.read_text())
        assert not (plan.parent/'result.json').exists(), 'Already attempted suite; preserve its result'
        name = m['stageName']
        assert name.replace('-', '').isalnum() and name not in [s['name'] for s in stages]
        timeout = m['timeoutSeconds']
        assert type(timeout) is int and 0 < timeout <= 120
        command = m['command']
        assert isinstance(command, list) and all(isinstance(v, str) for v in command)
        assert len(command) >= 6 and command[0] == sys.executable and command[1] == '-B'
        script = Path(command[2]).resolve()
        assert script.is_relative_to(ROOT/'scripts') and script.suffix == '.py'
        assert '--run-prepared' in command and '--prepare-only' not in command
        assert '--out' in command and Path(command[command.index('--out')+1]).resolve() == plan.parent
        expected = [m['executable'], m['driver']]
        assert m['ownedExecutables'] == expected
        for path, wanted in [(expected[0], m['executableSha256']), (expected[1], m['driverSha256'])]:
            assert Path(path).resolve().is_relative_to(plan.parent)
            assert digest(path) == wanted
            pins[path] = wanted
        for path, wanted in m['pinnedInputHashes'].items():
            assert Path(path).resolve().is_relative_to(plan.parent)
            assert digest(path) == wanted, 'Frozen suite input changed: '+path
            pins[path] = wanted
        assert m['originalInputHashes'][str(script)] == digest(script), 'Runner differs from reviewed suite'
        pins[str(script)] = digest(script)
        pins[str(plan)] = digest(plan)
        stages.append(dict(name=name, family=False, diagnosticContinuationAllowed=False,
                           out=str(plan.parent), timeoutSeconds=timeout,
                           ownedExecutables=expected, command=command))
    desktop_plan = desktop_plan.resolve()
    assert desktop_plan.is_relative_to(ROOT/'target')
    desktop = json.loads(desktop_plan.read_text())
    assert digest(desktop['driver']) == desktop['driverSha256']
    pins[str(desktop_plan)] = digest(desktop_plan)
    pins[desktop['driver']] = desktop['driverSha256']
    duration = sum(s['timeoutSeconds'] for s in stages)
    assert duration+20 < seconds, 'Full stage watchdogs and cleanup reserve must fit'
    m = dict(stages=stages, hashes=pins, desktopDriver=desktop['driver'],
             requestedIdleSeconds=seconds, absoluteDeadlineSeconds=seconds,
             cleanupReserveSeconds=20, stageWatchdogSumSeconds=duration,
             launchPerformed=False, scope='Independent frozen native suites, serial desktop ownership; any failed suite stops input. Preparation only.')
    out.mkdir()
    (out/'prepared.json').write_text(json.dumps(m, indent=2)+'\n')
    return m


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--plan', type=Path, action='append', required=True)
    parser.add_argument('--desktop-plan', type=Path, required=True)
    parser.add_argument('--seconds', type=int, default=240)
    args = parser.parse_args()
    m = prepare(args.out, args.plan, args.desktop_plan, args.seconds)
    print(json.dumps({'prepared':str(args.out.resolve()/'prepared.json'), 'stages':len(m['stages']),
                      'seconds':m['requestedIdleSeconds'], 'launched':False}))


if __name__ == '__main__':
    main()
