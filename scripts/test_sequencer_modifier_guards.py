#!/usr/bin/env python3
"""Native Control-key isolation and inactive Preview shortcut regression.

Uses a copied profile/sequence and CGEvent-backed local AppKit events, with muted
output. The input sequence must contain at least three effects on the Ruler
fixture's first visible row; only the private copy's first three are retained.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def check(out: Path) -> None:
    rows = json.loads((out / 'ui/result.json').read_text())
    assert len(rows) == 45 and all(row['ok'] for row in rows), rows
    states = [json.loads(row['detail']) for row in rows if row.get('detail', '').startswith('{')]
    assert len(states) == 9, len(states)
    base = states[0]
    key = lambda state: [(e['id'], e['start'], e['duration']) for e in state['elements']]
    assert len(base['selected']) == len(base['elements']) == 3
    assert sorted(e['duration'] for e in base['elements']) == [10_000_000, 20_000_000, 30_000_000]
    for state in states[:7]:
        assert key(state) == key(base), state['elements']
        assert state['undo'] == base['undo'] and state['redo'] == base['redo']
        assert state['modified'] == base['modified']
        assert state['transport'] == 'stopped' and state['playbackLineX'] == -1
    aligned = states[7]
    assert all(e['duration'] == 10_000_000 for e in aligned['elements']), aligned['elements']
    assert [e['start'] for e in aligned['elements']] == [e['start'] for e in base['elements']]
    assert len(aligned['undo']) == len(base['undo']) + 1
    assert key(states[8]) == key(base) and states[8]['redo']
    routes = [row['detail'] for row in rows if row.get('detail', '').startswith('posted ')]
    assert len(routes) == 8, routes
    assert all('route=control passed through' in route for route in routes[:6]), routes
    assert 'route=table ' in routes[6] and 'handled=true' in routes[6], routes[6]
    assert 'route=window undo/redo' in routes[7], routes[7]
    print('PASS: 45 native steps; Control-D/I/F5 and forwarded equivalents unchanged; Cmd-U aligns exact durations; Undo restores')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, default=Path('build/Vixen.app'))
    parser.add_argument('--profile', type=Path)
    parser.add_argument('--sequence', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    out = args.out.resolve()
    if args.check_only:
        check(out)
        return
    assert args.profile and args.sequence
    assert not out.exists(), 'Use a fresh output directory'
    out.mkdir(parents=True)
    profile = out / 'profile'
    shutil.copytree(args.profile, profile)
    (profile / '.lock').unlink(missing_ok=True)
    sequence = profile / 'Sequence/Ruler.tim'
    text = args.sequence.read_text(encoding='utf-8-sig')
    group = re.search(r'(<_effectNodeSurrogates\b[^>]*>)(.*?)(</_effectNodeSurrogates>)', text, re.S)
    assert group
    effects = re.findall(r'<EffectNodeSurrogate>.*?</EffectNodeSurrogate>', group[2], re.S)
    assert len(effects) >= 3
    changed = []
    for effect, start, duration in zip(effects[:3], [0, 3, 5], [1, 2, 3]):
        effect = re.sub(r'<StartTime>.*?</StartTime>', f'<StartTime>PT{start}S</StartTime>', effect)
        effect = re.sub(r'<TimeSpan>.*?</TimeSpan>', f'<TimeSpan>PT{duration}S</TimeSpan>', effect)
        changed.append(effect)
    text = text[:group.start()] + group[1] + '\n'.join(changed) + group[3] + text[group.end():]
    sequence.write_text(text, encoding='utf-8-sig')
    steps = [
        {'action': 'window', 'id': 'administration'}, {'action': 'wait', 'seconds': 2},
        {'action': 'window', 'id': 'sequencer'}, {'action': 'wait', 'seconds': 4},
        {'action': 'focus', 'window': 'Ruler'},
        {'action': 'hook', 'name': 'sequencer.resize', 'path': ['1400', '900']},
        {'action': 'wait', 'seconds': 0.5}, {'action': 'mouse', 'x': 440, 'y': 215},
        {'action': 'wait', 'seconds': 0.4},
        {'action': 'menu', 'path': ['Edit', 'Select All Elements']},
        {'action': 'wait', 'seconds': 0.4},
        {'action': 'hook', 'name': 'sequencer.pointer', 'path': ['440', '215']},
        {'action': 'hook', 'name': 'timelineState'},
    ]
    for forwarded in [False, True]:
        for key in ['d', 'i', 'f5']:
            steps += [
                {'action': 'hook', 'name': 'sequencer.keyEvent', 'path': [key, 'ctrl'] + (['forwarded'] if forwarded else [])},
                {'action': 'wait', 'seconds': 0.4}, {'action': 'hook', 'name': 'timelineState'},
                {'action': 'focus', 'window': 'Ruler'},
            ]
    for key in ['u', 'z']:
        steps += [{'action': 'hook', 'name': 'sequencer.keyEvent', 'path': [key, 'cmd']},
                  {'action': 'wait', 'seconds': 0.4}, {'action': 'hook', 'name': 'timelineState'}]
    steps += [{'action': 'menus', 'name': 'modifier-menu'}, {'action': 'snapshot', 'name': 'modifiers'},
              {'action': 'quit'}]
    assert len(steps) == 46  # Quit exits before writing a result row.
    script = out / 'steps.json'
    script.write_text(json.dumps({'steps': steps}, indent=2))
    env = {key: value for key, value in os.environ.items() if not key.startswith(('VIXEN_', 'DYLD_'))}
    env.update(VIXEN_UI_TEST=str(script), VIXEN_UI_TEST_OUT=str(out/'ui'), VIXEN_DATA=str(profile),
               VIXEN_TEST_SEQUENCE=str(sequence), VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        result = subprocess.run([str(args.app.resolve()/'Contents/MacOS/Vixen')], env=env,
                                stdout=log, stderr=subprocess.STDOUT, timeout=60)
    assert result.returncode == 0, result.returncode
    check(out)


if __name__ == '__main__':
    main()
