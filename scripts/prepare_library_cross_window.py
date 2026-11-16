#!/usr/bin/env python3
"""Prepare the next cross-sequencer toolbar route, without launching any app.

The output is intentionally NOT a ready foreground plan: first run its isolated
layout-only steps in a root-coordinated background slot and review both rasters.
No physical coordinates are accepted or executable input driver is generated.
"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
tree = lambda p: {str(f.relative_to(p)): sha(f) for f in sorted(p.rglob('*')) if f.is_file()}
local = lambda tag: tag.rsplit('}', 1)[-1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--app', type=Path, default=ROOT/'target/private-app-native-curve-tooltip-release/Vixen.app')
    args = parser.parse_args()
    out, app = args.out.resolve(), args.app.resolve()
    source = ROOT/'target/library-drag-fixture/profile'
    assert out.is_relative_to(ROOT/'target') and app.is_relative_to(ROOT/'target')
    assert not out.exists() and not any(p.is_symlink() for p in source.rglob('*'))
    clone = json.loads((app.parent/'private-app.json').read_text())
    info = plistlib.loads((app/'Contents/Info.plist').read_bytes())
    exe = app/'Contents/MacOS/Vixen'
    assert clone['passed'] and sha(exe) == clone['executableSha256']
    assert info['CFBundleIdentifier'] == clone['bundleIdentifier'] and info['CFBundleIdentifier'].startswith('com.vixenlights.verification.')
    config = ET.parse(source/'SystemData/SystemConfig.xml').getroot()
    assert not list(next(e for e in config if local(e.tag) == 'Controllers'))
    before = tree(source)
    out.mkdir(); shutil.copytree(source, out/'profile')
    profile = out/'profile'; (profile/'.lock').unlink(missing_ok=True)
    names = ['Source Library', 'Destination Library']
    for name in names:
        shutil.copy2(source/'Sequence/Library Drag.tim', profile/'Sequence'/(name+'.tim'))
    hook = lambda name, *args: {'action':'hook', 'name':name, 'path':list(args)}
    wait = lambda n: {'action':'wait', 'seconds':n}
    steps = [{'action':'window', 'id':'administration'}, wait(1)]
    # Each sequencer starts with its toolbar hidden. Wait for the active model's
    # command bindings to settle before observing or toggling its menu item;
    # otherwise the menu can still describe the previously active sequencer.
    for name in names:
        steps += [hook('admin.openSequence', str(profile/'Sequence'/(name+'.tim'))), wait(2)]
    steps += [{'action':'windows'}]
    for label, name in zip(['source','destination'], names):
        steps += [hook('sequencer.activate',name), {'action':'focus','window':name},
                  hook('sequencer.resize','1100','780'), hook('sequencer.panel','Curve Library'),
                  wait(.6), hook('sequencer.transport','state'), {'action':'menus','name':label+'-before-toolbar'},
                  {'action':'menu','path':['View','Toolbars','Library Curve']},wait(.6),
                  hook('sequencer.focusState'), {'action':'dump','name':label+'-controls'},
                  {'action':'menus','name':label+'-menus'}, {'action':'snapshot','name':label+'-ready'}]
    steps += [hook('timelineAllPlayback'), {'action':'windows'}, {'action':'quit'}]
    (out/'layout-steps.json').write_text(json.dumps({'steps':steps},indent=2)+'\n')
    shutil.copy2(__file__,out/'prepare_library_cross_window.py')
    pins = {str(p):sha(p) for p in [ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift',ROOT/'app/Sources/VixenSequencer/SequencerModel.swift',ROOT/'app/Sources/VixenKit/UITestHarness.swift',ROOT/'target/native-library-owner-review.json']}
    plan = dict(prepared=True,launched=False,foregroundReady=False,readinessReason='Both exact per-window current-release rasters, AX titles and toolbar visibility must be observed before finalizing coordinates or a physical driver.',
        app=str(app),appSha256=sha(exe),sourceExecutableSha256=clone['sourceExecutableSha256'],privateBundleIdentifier=clone['bundleIdentifier'],
        source=str(source),sourceHashes=before,preparedProfileHashes=tree(profile),layoutSteps=len(steps)-1,layoutStepsSha256=sha(out/'layout-steps.json'),
        preparerSha256=sha(__file__),reviewedSourcePins=pins,
        environment={'VIXEN_DATA':str(profile),'VIXEN_MUTE':'1','VIXEN_UI_TEST':str(out/'layout-steps.json'),'VIXEN_UI_TEST_OUT':str(out/'ui'),'VIXEN_SETTINGS_DIR':str(out/'ui'),'VIXEN_UI_TEST_TRACE':'1','VIXEN_LIBRARY_DRAG_TRACE':'1','TMPDIR':str(out/'runtime')+'/'},
        backgroundLayoutBounds={'processSeconds':35,'ownedCleanupSeconds':6,'globalInput':False,'foregroundActivation':False,'isolation':'Private copied controller-free profile, unique Cocoa app identity, explicit private settings/TMPDIR; same current clone may only run serially.'},
        physicalCases=[{'name':'cross-model-copy','route':'Source Library Curve A toolbar → Destination Library Curve B toolbar, no modifiers','expected':'Actual destination Curve name? sheet. Focused Unicode name Cross Window Curve, actual OK, live saved order A/B/C/D/Cross Window Curve and levels10/20/30/40/10, current and unlinked. No internal reorder.'},
                       {'name':'cross-model-linked-rejection','route':'Source Library Curve A toolbar → Destination Library Curve B toolbar with Command held during drag','expected':'Library source creates a linked payload; native destination rejects it, no name sheet, saved five-entry order and values unchanged. Observe actual drag-session ended operation0.'}],
        futureDriverBounds={'inputSeconds':40,'helperCleanupEndSeconds':48,'outerStageSeconds':75},
        geometryProposal={'status':'NOT CALIBRATED; never use before retained current screenshots/AX/CG frame review','windowPoints':[1100,780],'sourcePlacement':'Right/front','destinationPlacement':'Left/behind','reason':'Both toolbar origins remain exposed on an actual sufficiently wide screen; every posted point must belong to the intended exact owned window, not merely its PID.'},
        futureAcceptance=['Same owned private PID with two distinct exact AX windows and two sequencer models; no third target window ambiguity.','Read-only live ModuleStore checkpoints before/after each drag and exact saved values; both transports stopped.','No ValueDragTesting, libraryMove, receive, drag callback, or other mutation hook. Setup hooks only open/select/size existing UI.','Freeze source/helper/executable/profile/calibration hashes after layout observation. Match each screenshot dimensions/profile-normalized pixels, window title and actual CG frame.','Each input checks the absolute deadline, foreground private-app identity and exact expected top window at its point; actual naming field PID/focus and readback.','Abort on new foreground app, authorization surface, identity loss, unexpected sheet, uncertain drag state, or deadline; release owned held inputs safely before terminating own process.','Restore previous app/pointer only while still owning foreground. Record explicit input booleans, held state, owner preferences, source/app hashes, and reaped processes.'],
        exclusions=['Not editor→library or library→editor.','Does not cover Color/Gradient cross-window paths or arbitrary external applications.','Does not establish physical acceptance until an observed calibrated driver has run under a fresh root-coordinated desktop window.'])
    assert tree(source)==before
    (out/'prepared.json').write_text(json.dumps(plan,indent=2)+'\n')
    print(json.dumps({'prepared':str(out/'prepared.json'),'layoutSteps':plan['layoutSteps'],'foregroundReady':False,'launched':False}))

if __name__ == '__main__': main()
