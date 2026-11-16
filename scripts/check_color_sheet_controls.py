#!/usr/bin/env python3
"""Exercise real gradient/collection SwiftUI controls in a private muted host.

Uses matching cached debug objects; never posts WindowServer input. Coordinate
with foreground checks before --run-prepared because Cocoa creates hidden UI.
"""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
from check_color_sessions import ROOT, digest


def plan():
    hook = lambda name, *args: {'action':'hook', 'name':name, 'path':list(args)}
    wait = {'action':'wait', 'seconds':.4}
    click = lambda label: {'action':'click', 'label':label}
    state = lambda name: hook('controls.'+name)
    steps = []
    for revert in [False, True]:
        steps += [state('collections'), wait, click('Add Color')]
        if revert:
            steps += [hook('color.choose','FFFF0000'), hook('color.choose','FF0000FF')]
        steps += [hook('color.finish'), state('collectionState')]
    steps += [state('collections'), wait, click('Add Color'), hook('color.choose','FFFF0000'),
              hook('controls.chooseCollection','Collection B'), wait,
              hook('color.finish'), state('collectionState'),
              hook('controls.chooseCollection','Collection A'), wait, state('collectionState')]
    steps += [state('collections'), wait, click('Add Color'), hook('color.choose','FFFF0000'),
              click('Add Color'), hook('color.finish'), state('collectionState'), state('clientRevert')]
    steps += [state('gradient'), wait, hook('controls.fader','0','edit'), wait,
              hook('color.choose','FFFF0000'), click('Load Preset'), wait, state('gradientState'),
              click('Cancel'), wait, hook('color.finish'), state('gradientState'), click('Cancel')]
    steps += [state('linkedGradient'), wait, click('Unlink Gradient'), wait, state('gradientState'), click('Cancel')]
    steps += [state('gradient'), wait, hook('controls.fader','0','edit'), wait,
              hook('color.choose','FFFF0000'), click('Save Preset'), wait, state('gradientState'),
              {'action':'type','text':'Pending Save'}, click('OK'), wait, hook('color.finish'),
              state('savedPreset'), click('Cancel')]
    return {'steps': steps + [{'action':'quit'}]}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build', required=True, type=Path)
    p.add_argument('--out', required=True, type=Path)
    p.add_argument('--profile', type=Path, default=ROOT/'target/library-drag-fixture/profile')
    p.add_argument('--baseline', action='store_true')
    p.add_argument('--prepare-only', action='store_true')
    p.add_argument('--run-prepared', action='store_true')
    args = p.parse_args()
    build, out, profile = args.build.resolve(), args.out.resolve(), args.profile.resolve()
    assert all(path.is_relative_to(ROOT/'target') for path in [build,out,profile])
    products = build/'swift/out/Products/Debug'
    source = ROOT/'scripts/check_color_sheet_controls.swift'
    app = out/'ColorSheetControls.app'
    executable = app/'Contents/MacOS/ColorSheetControls'
    objects = sorted(products.glob('*.o'))
    inputs = [source, *objects, build/'pkg/lib/libvixen_ffi.a', build/'pkg/Sources/VixenBridge/vixen_ffi.swift']
    for name in ['VixenKit','VixenSequencer']:
        inputs += [path for path in (products/f'{name}.swiftmodule').glob('*') if path.is_file()]
    if not args.run_prepared:
        assert not out.exists()
        executable.parent.mkdir(parents=True)
        cmd = ['/usr/bin/swiftc','-module-cache-path',str(out/'clang-cache'),'-parse-as-library',
               '-swift-version','6','-default-isolation','MainActor','-target','arm64-apple-macos26.0',
               '-I',str(products),'-Xcc','-fmodule-map-file='+str(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'),
               '-I',str(build/'pkg/Sources/vixen_ffiFFI/include'),str(source),*(str(path) for path in objects),
               '-L',str(build/'pkg/lib'),'-lvixen_ffi','-lc++','-lcurl']
        for framework in ['AppKit','SwiftUI','AudioToolbox','AudioUnit','CoreAudio','CoreFoundation',
                          'CoreGraphics','CoreText','Foundation','IOKit','Metal','QuartzCore','Security','SystemConfiguration']:
            cmd += ['-framework',framework]
        cmd += ['-o',str(executable)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        bundle = 'com.vixenlights.verification.colorsheets.'+digest(executable)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':bundle,
            'CFBundleExecutable':'ColorSheetControls','CFBundleName':'Vixen Colour Sheet Verification',
            'CFBundlePackageType':'APPL','LSUIElement':True,'LSMinimumSystemVersion':'26.0'}))
        subprocess.run(['/usr/bin/codesign','--force','--sign','-',str(app)],capture_output=True,check=True,timeout=30)
        before = {str(path.relative_to(profile)):digest(path) for path in profile.rglob('*') if path.is_file()}
        shutil.copytree(profile,out/'profile'); (out/'profile/.lock').unlink(missing_ok=True)
        (out/'ui').mkdir()
        (out/'steps.json').write_text(json.dumps(plan(),indent=2)+'\n')
        manifest = {'baseline':args.baseline,'inputHashes':{str(path):digest(path) for path in inputs},
                    'executableSha256':digest(executable),'bundleIdentifier':bundle,
                    'profile':str(profile),'profileHashes':before,'compileCommand':cmd}
        (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
    if args.prepare_only: print(out/'prepared.json'); return
    manifest = json.loads((out/'prepared.json').read_text())
    assert manifest['baseline'] == args.baseline and digest(executable)==manifest['executableSha256']
    assert all(digest(Path(path))==expected for path,expected in manifest['inputHashes'].items())
    env = {k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',
               VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process = subprocess.run([str(executable)],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=60)
    assert process.returncode==0, (process.returncode,out/'process.log')
    rows = json.loads((out/'ui/result.json').read_text())
    steps = json.loads((out/'steps.json').read_text())['steps'][:-1]
    assert len(rows)==len(steps) and all(row['ok'] for row in rows), rows
    states = lambda name: [json.loads(row['detail']) for step,row in zip(steps,rows) if step.get('name')==name]
    collections = states('controls.collectionState')
    gradients = states('controls.gradientState')
    presets = states('controls.savedPreset')
    clients = states('controls.clientRevert')[0]
    checks = [
        ('collection_unchanged_add', collections[0]['colors']==[0xff00ff00,0xff0000ff]),
        ('collection_reverted_add', collections[1]['colors']==[0xff00ff00,0xff0000ff]),
        ('collection_pending_switch_preserves_B', collections[2]['selected']=='Collection B' and collections[2]['colors']==[0xff0000ff]),
        ('collection_pending_switch_appends_A', collections[3]['selected']=='Collection A' and collections[3]['colors']==[0xff00ff00,0xffff0000]),
        ('collection_repeated_add_starts_from_latest_committed_color', collections[4]['colors']==[0xff00ff00,0xffff0000,0xffff0000]),
        ('effect_reverted_preview_cleared_without_commit', clients['effectLiveCleared'] and not clients['effectCommits']),
        ('marks_reverted_preview_cleared_without_edit', clients['marksLiveCleared'] and clients['marksRevisionUnchanged']),
        ('gradient_load_preset_commits_original_first', gradients[0]['colors']==[0xffff0000,0xff0000ff]),
        ('gradient_load_preset_cancel_keeps_pending_color', gradients[1]['colors']==[0xffff0000,0xff0000ff]),
        ('gradient_unlink_preserves_preset', gradients[2]['colors']==[0xffffff00] and not gradients[2]['link']),
        ('gradient_save_preset_commits_original_first', gradients[3]['colors']==[0xffff0000,0xff0000ff]),
        ('gradient_saved_preset_contains_pending_color', presets==[[{'name':'Pending Save','colors':[0xffff0000,0xff0000ff]}]]),
    ]
    assert all(digest(Path(path))==expected for path,expected in manifest['inputHashes'].items())
    assert {str(path.relative_to(profile)):digest(path) for path in profile.rglob('*') if path.is_file()} == manifest['profileHashes']
    result = {'passed':all(passed for _,passed in checks),'checks':[{'name':name,'passed':passed} for name,passed in checks],
              'baseline':args.baseline,'steps':len(rows),'collections':collections,'gradients':gradients,'presets':presets,'clients':clients,
              'executableSha256':manifest['executableSha256'],'inputsUnchanged':True,
              'scope':'Actual mounted GradientEditorDialog and ColorCollectionsSheet buttons, compiled production collection Picker Binding setter obtained from view body, real Rust editors and effect/Marks clients. Colour chosen through existing test setter. No WindowServer menu/input or physical system colour-panel claim.'}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    failures = [name for name,passed in checks if not passed]
    if args.baseline:
        assert {'collection_unchanged_add','collection_pending_switch_preserves_B','gradient_saved_preset_contains_pending_color'} <= set(failures), failures
    else: assert result['passed'], failures
    print(json.dumps({'checks':len(checks),'baseline':args.baseline,'failures':failures,'expectedOutcomeObserved':True}))


if __name__=='__main__': main()
