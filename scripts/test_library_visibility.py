#!/usr/bin/env python3
"""Check native library drag registration while switching hidden dock tabs.

Runs offscreen in a unique-domain test app, with the generated controller-free
library profile. The probe only reads existing native view state; it never calls
drag or drop handlers. Actual WindowServer drag acceptance is a separate check.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    app, out = args.app.resolve(), args.out.resolve()
    assert out.is_relative_to(ROOT / 'target') and not out.exists()
    command = [sys.executable, str(ROOT / 'scripts/test_library_drag_foreground.py'),
               '--app', str(app), '--out', str(out), '--layout-only']
    subprocess.run(command + ['--prepare-only'], check=True, timeout=30)
    plan_path = out / 'steps.json'
    plan = json.loads(plan_path.read_text())
    assert plan['steps'].pop()['action'] == 'quit'
    expected = ['curve', 'color', 'gradient', 'curve']
    probe = {'action': 'hook', 'name': 'sequencer.libraryDestinations', 'path': []}
    plan['steps'].append(probe)
    for panel in ['Color Library', 'Gradient Library', 'Curve Library']:
        plan['steps'] += [{'action': 'hook', 'name': 'sequencer.panel', 'path': [panel]},
                          {'action': 'wait', 'seconds': .4}, probe]
    plan['steps'].append({'action': 'quit'})
    plan_path.write_text(json.dumps(plan, indent=2) + '\n')
    manifest_path = out / 'prepared.json'
    manifest = json.loads(manifest_path.read_text())
    manifest['steps'] = len(plan['steps']) - 1
    manifest['visibilityPanels'] = expected
    manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
    subprocess.run(command + ['--run-prepared'], check=True, timeout=90)
    rows = json.loads((out / 'ui/result.json').read_text())
    states = [json.loads(row['detail']) for step, row in zip(plan['steps'], rows)
              if step.get('name') == 'sequencer.libraryDestinations']
    assert len(states) == len(expected)
    checks = []
    for current, state in zip(expected, states):
        assert set(item['kind'] for item in state) == {'color', 'curve', 'gradient'}, state
        for kind in ['color', 'curve', 'gradient']:
            matching = [item for item in state if item['kind'] == kind]
            assert len(matching) == 5 and sum(item['role'] == 'empty' for item in matching) == 1, matching
            active = kind == current
            assert all(item['hidden'] == (not active) and item['registered'] == (3 if active else 0)
                       for item in matching), (current, matching)
        checks.append({'selectedPanel': current, 'nativeDestinations': state})
    base = json.loads((out / 'result.json').read_text())
    report = {'passed': True, 'steps': base['steps'], 'appSha256': base['appSha256'],
              'sourceUnchanged': base['sourceUnchanged'], 'transportStopped': base['transportStopped'],
              'checks': checks,
              'scope': 'Native visibility and registered drag types across four tab states, 60 destination-state assertions; no foreground drag acceptance claim.'}
    (out / 'visibility-result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: report[k] for k in ['passed', 'steps', 'scope']}))


if __name__ == '__main__':
    main()
