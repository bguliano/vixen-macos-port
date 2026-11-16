#!/usr/bin/env python3
"""Bounded foreground-app ownership check using private fixtures and local CG events.

No system-wide keys, AppleEvents, output devices, original profile, or CUA are used.
Requires explicit desktop readiness; the guard stops for SecurityAgent.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
local = lambda tag: tag.split('}')[-1]
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()


def disk_state(profile):
    store = profile / 'SystemData/ModuleStore.xml'
    sequence = profile / 'Sequence/Foreground Sequence.tim'
    preview = next(n for n in ET.parse(store).getroot().iter() if local(n.tag) == 'VixenPreviewData')
    display = next(n for n in preview if local(n.tag) == 'DisplayItems')
    effects = next(n for n in ET.parse(sequence).getroot() if local(n.tag) == '_effectNodeSurrogates')
    return {'shapes': len(display), 'effects': len(effects), 'storeSha256': sha(store), 'sequenceSha256': sha(sequence)}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--app', type=Path, default=ROOT/'target/build-release/Vixen.app')
    ap.add_argument('--generator', type=Path, default=ROOT/'target/vixen-session/debug/examples/preview_dialogs')
    ap.add_argument('--sequence-template', type=Path, default=ROOT/'target/export-sheet-native-final/profile/Sequence/Export Short.tim')
    ap.add_argument('--guard', type=Path, default=ROOT/'target/foreground-tools/guard')
    ap.add_argument('--expected-app-sha256', default='0d7edc262607ad317134ab6f2fc03e8730d72151d7d6b0b74342d0b6331f992d')
    ap.add_argument('--focus-probe', action='store_true', help='Retain the first Save/focus diagnosis only; never claims acceptance')
    ap.add_argument('--out', type=Path, required=True)
    args = ap.parse_args()
    out = args.out.resolve()
    assert out.is_relative_to(ROOT/'target') and not out.exists()
    app = args.app.resolve()/'Contents/MacOS/Vixen'
    assert sha(app) == args.expected_app_sha256
    status = json.loads(subprocess.check_output([str(args.guard), '--status']))
    assert status['pid'] and 'securityagent' not in status['bundle'].lower(), status
    out.mkdir()
    profile = out/'profile'
    subprocess.run([str(args.generator.resolve()), str(profile)], check=True, capture_output=True)
    config = ET.parse(profile/'SystemData/SystemConfig.xml').getroot()
    assert len(config.find('Controllers')) == 0
    node = next(n for n in config.iter('Node') if n.get('name') == 'Lamp-1')
    source_hash = sha(args.sequence_template)
    text = args.sequence_template.read_text(encoding='utf-8-sig')
    assert text.count('<EffectNodeSurrogate>') == 1 and '<_mediaSurrogates xmlns="" />' in text
    text = re.sub(r'<NodeId>[^<]+</NodeId>', f'<NodeId>{node.attrib["id"]}</NodeId>', text)
    text = text.replace('<Name>Private test lights</Name>', '<Name>Lamp-1</Name>')
    text = text.replace('<TimeSpan>PT30S</TimeSpan>', '<TimeSpan>PT2S</TimeSpan>')
    sequence = profile/'Sequence/Foreground Sequence.tim'
    sequence.write_text(text, encoding='utf-8-sig')
    baseline = disk_state(profile)
    assert (baseline['shapes'], baseline['effects']) == (2, 1)
    h = lambda name, *args: {'action': 'hook', 'name': name, 'path': list(args)}
    wait = lambda seconds=.4: {'action': 'wait', 'seconds': seconds}
    focus = lambda title: {'action': 'focus', 'window': title}
    info = lambda: {'action': 'designerInfo'}
    pkey = lambda key, code, mods='cmd': {'action': 'designerKeyEvent', 'text': key, 'index': code, 'label': mods}
    skey = lambda key, *mods: h('sequencer.keyEvent', key, *mods)
    checkpoints = {}
    steps = [{'action': 'window', 'id': 'administration'}, wait(1), h('admin.openSequence', str(sequence)), wait(2),
             {'action': 'previewTarget', 'text': 'Preview Dialog Verification'},
             {'action': 'window', 'id': 'preview-setup'}, wait(1),
             {'action': 'designerOnScreen', 'x': 80, 'y': 80}, wait(),
             {'action': 'snapshot', 'name': 'activation-ready'}, wait(3), focus('Preview Configuration'),
             h('timelineState'), {'action': 'designerCommand', 'text': 'selectAll'},
             {'action': 'designerCommand', 'text': 'group'}, info()]
    for key, code, mods in [('z',6,'cmd'),('y',16,'cmd'),('z',6,'cmd'),('z',6,'cmd shift')]:
        steps += [pkey(key,code,mods), info(), h('timelineState')]
    def checkpoint(name, shapes, effects):
        steps.extend([{'action': 'snapshot', 'name': name}, wait(3)])
        checkpoints[name] = (shapes, effects)
    checkpoint('before-preview-save', 2, 1)
    steps += [h('sequencer.focusState'), {'action':'menus','name':'preview-save-menu'}, pkey('s',1), wait(1), h('sequencer.focusState'), info(), h('timelineState')]
    checkpoint('preview-saved-before-close', 1, 1)
    probe_steps = steps[:] + [{'action':'quit'}]
    steps += [pkey('z',6), info(), focus('Foreground Sequence'), wait(),
              h('sequencer.activate','Foreground Sequence'), h('sequencer.focusState'), {'action': 'designerMenuInfo'},
              {'action': 'menu', 'path': ['Edit','Select All Elements']}, wait(), h('timelineState')]
    for key, mods in [('delete',()),('z',('cmd',)),('y',('cmd',))]:
        steps += [skey(key,*mods), wait(), h('timelineState'), info(), {'action': 'designerMenuInfo'}]
    steps += [skey('s','cmd'), wait(1), h('timelineState'), info()]
    checkpoint('sequence-saved-preview-unsaved', 1, 0)
    steps += [focus('Preview Configuration'), pkey('y',16), h('sequencer.focusState'), info(), h('timelineState'),
              pkey('z',6), info(), h('timelineState'), pkey('s',1), wait(1), info(), h('timelineState')]
    checkpoint('preview-resaved-before-close', 2, 0)
    steps += [{'action': 'quit'}]
    if args.focus_probe:
        steps = probe_steps
        checkpoints = {key:value for key,value in checkpoints.items() if key in ('before-preview-save','preview-saved-before-close')}
    script = out/'steps.json'; script.write_text(json.dumps({'steps':steps},indent=2))
    ui, runtime = out/'ui', out/'runtime'; ui.mkdir(); runtime.mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(VIXEN_DATA=str(profile),VIXEN_MUTE='1',VIXEN_UI_TEST=str(script),VIXEN_UI_TEST_OUT=str(ui),
               VIXEN_UI_TEST_TRACE='1',TMPDIR=str(runtime)+'/')
    report = {'passed':False,'appSha256':sha(app),'foregroundBefore':status,'baseline':baseline,'checkpoints':{},
              'scope':'Foreground app and real key/main ownership, local CG-backed AppKit events; not physical keyboard/pointer routing'}
    process = None; guard = None
    try:
        with (out/'process.log').open('w') as log:
            process = subprocess.Popen([str(app)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            deadline = time.monotonic()+100
            while process.poll() is None and time.monotonic()<deadline:
                if guard is None and (ui/'activation-ready.png').exists():
                    guard = subprocess.Popen([str(args.guard.resolve()),str(process.pid),str(out/'foreground.json'),'80'])
                for name,wanted in checkpoints.items():
                    if name not in report['checkpoints'] and (ui/(name+'.png')).exists():
                        actual=disk_state(profile)
                        actual['expectedShapes'], actual['expectedEffects'] = wanted
                        actual['matched'] = (actual['shapes'],actual['effects']) == wanted
                        actual['observedWhileProcessAlive']=process.poll() is None
                        report['checkpoints'][name]=actual
                        (out/'live-checkpoints.json').write_text(json.dumps(report['checkpoints'],indent=2))
                time.sleep(.1)
            assert process.poll() is not None,'Native deadline expired'
        if guard: guard.wait(timeout=8)
        report['exitCode']=process.returncode
        if (out/'foreground.json').exists():
            report['foreground']=json.loads((out/'foreground.json').read_text())
            assert not report['foreground'].get('blocked'),report['foreground']
        rows=json.loads((ui/'result.json').read_text())
        report['steps']=len(rows)
        assert process.returncode==0 and len(rows)==len(steps)-1 and all(r['ok'] for r in rows)
        foreground=json.loads((out/'foreground.json').read_text());report['foreground']=foreground
        assert foreground['activated'] and not foreground['blocked'] and foreground['targetExited']
        assert foreground['frontmostAfterActivationPID']==process.pid
        assert foreground['restored'] and foreground['restoredPID']==foreground['previousPID']
        assert set(report['checkpoints'])==set(checkpoints)
        if args.focus_probe:
            report['diagnosticOnly'] = True
            report['focusStates'] = [json.loads(r['detail']) for r in rows if r['action']=='hook' and steps[r['step']]['name']=='sequencer.focusState']
            report['keyTelemetry'] = [r['detail'] for r in rows if r['action']=='designerKeyEvent']
            report['windowOwnershipValid'] = bool(report['keyTelemetry']) and all('keyBefore=Preview Configuration keyAfter=Preview Configuration main=Preview Configuration' in detail for detail in report['keyTelemetry'])
            report['saveMatched'] = report['checkpoints']['preview-saved-before-close']['matched']
            report['recorded'] = True
            return
        preview=[r for r in rows if r['action']=='designerKeyEvent']
        assert len(preview)==9
        assert all('keyBefore=Preview Configuration keyAfter=Preview Configuration main=Preview Configuration' in r['detail'] and 'previewActive=true' in r['detail'] for r in preview),preview
        assert all('fileSave=true' in r['detail'] for r in preview if 'key s ' in r['detail']),preview
        infos=[r['detail'] for r in rows if r['action']=='designerInfo']
        assert [int(re.search(r' shapes=(\d+)',s)[1]) for s in infos]==[1,2,1,2,1,1,2,2,2,2,2,1,2,2],infos
        states=[json.loads(r['detail']) for r in rows if r['action']=='hook' and steps[r['step']]['name']=='timelineState']
        counts=[len(s['elements']) for s in states]
        assert counts==[1,1,1,1,1,1,1,0,1,0,0,0,0,0],counts
        assert all(s['transport']=='stopped' for s in states)
        assert all(s['elements']==states[0]['elements'] and s['undo']==states[0]['undo'] and s['modified']==states[0]['modified'] for s in states[:6])
        assert not states[10]['modified'] and all(s['elements']==states[10]['elements'] and s['undo']==states[10]['undo'] for s in states[11:])
        menu=[r['detail'] for r in rows if r['action']=='designerMenuInfo']
        assert len(menu)==4 and all('key=Foreground Sequence active=false' in s for s in menu),menu
        assert sha(args.sequence_template)==source_hash and sha(app)==report['appSha256']
        assert all(c['matched'] for c in report['checkpoints'].values()), report['checkpoints']
        checks=report['checkpoints']
        assert checks['before-preview-save']['sequenceSha256']==checks['preview-saved-before-close']['sequenceSha256']
        assert checks['preview-saved-before-close']['storeSha256']==checks['sequence-saved-preview-unsaved']['storeSha256']
        assert checks['sequence-saved-preview-unsaved']['sequenceSha256']==checks['preview-resaved-before-close']['sequenceSha256']
        report.update(passed=True,previewShapeCounts=[int(re.search(r' shapes=(\d+)',s)[1]) for s in infos],sequenceEffectCounts=counts)
    except BaseException as error:
        report['error'] = repr(error)
        raise
    finally:
        if process and process.poll() is None:
            os.killpg(process.pid,signal.SIGTERM)
            try: process.wait(timeout=3)
            except subprocess.TimeoutExpired: os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=3)
        if guard and guard.poll() is None:
            try: guard.wait(timeout=6)
            except subprocess.TimeoutExpired: guard.terminate();guard.wait(timeout=3)
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'steps':report['steps'],'report':str(out/'result.json')}))


if __name__=='__main__': main()
