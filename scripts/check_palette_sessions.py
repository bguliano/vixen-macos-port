#!/usr/bin/env python3
"""Verify real palette edit/mutation ordering with private copied inputs."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
from check_color_sessions import ROOT, digest


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build',required=True,type=Path)
    p.add_argument('--out',required=True,type=Path)
    p.add_argument('--profile',type=Path,default=ROOT/'target/library-drag-fixture/profile')
    p.add_argument('--baseline',action='store_true')
    p.add_argument('--payload-baseline',action='store_true')
    p.add_argument('--prepare-only',action='store_true')
    p.add_argument('--run-prepared',action='store_true')
    args = p.parse_args()
    if args.payload_baseline: args.baseline=True
    build,out,profile=args.build.resolve(),args.out.resolve(),args.profile.resolve()
    assert all(x.is_relative_to(ROOT/'target') for x in [build,out,profile])
    products=build/'swift/out/Products/Debug'; source=ROOT/'scripts/check_palette_sessions.swift'
    app=out/'PaletteSessions.app';exe=app/'Contents/MacOS/PaletteSessions'
    objects=sorted(products.glob('*.o'))
    inputs=[source,*objects,build/'pkg/lib/libvixen_ffi.a',build/'pkg/Sources/VixenBridge/vixen_ffi.swift']
    for name in ['VixenKit','VixenSequencer']:
        inputs += [x for x in (products/f'{name}.swiftmodule').glob('*') if x.is_file()]
    if not args.run_prepared:
        assert not out.exists();exe.parent.mkdir(parents=True)
        cmd=['/usr/bin/swiftc','-module-cache-path',str(out/'clang-cache'),'-parse-as-library','-swift-version','6',
             '-default-isolation','MainActor','-target','arm64-apple-macos26.0','-I',str(products),'-Xcc',
             '-fmodule-map-file='+str(build/'swift/out/Intermediates.noindex/GeneratedModuleMaps/vixen_ffiFFI.modulemap'),
             '-I',str(build/'pkg/Sources/vixen_ffiFFI/include'),str(source),*(str(x) for x in objects),
             '-L',str(build/'pkg/lib'),'-lvixen_ffi','-lc++','-lcurl']
        for framework in ['AppKit','SwiftUI','AudioToolbox','AudioUnit','CoreAudio','CoreFoundation','CoreGraphics',
                          'CoreText','Foundation','IOKit','Metal','QuartzCore','Security','SystemConfiguration']:
            cmd += ['-framework',framework]
        cmd += ['-o',str(exe)]
        with (out/'compile.log').open('w') as log:
            subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        bundle='com.vixenlights.verification.palettesessions.'+digest(exe)[:16]
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':bundle,'CFBundleExecutable':'PaletteSessions',
            'CFBundleName':'Vixen Palette Verification','CFBundlePackageType':'APPL','LSUIElement':True,'LSMinimumSystemVersion':'26.0'}))
        subprocess.run(['/usr/bin/codesign','--force','--sign','-',str(app)],check=True,capture_output=True,timeout=30)
        compiled={str(x):digest(x) for x in inputs};frozen={};mapping={}
        for n,(name,wanted) in enumerate(compiled.items()):
            dst=out/'input-snapshot'/f'{n:03d}-{Path(name).name}';dst.parent.mkdir(exist_ok=True)
            subprocess.run(['/bin/cp','-c',name,str(dst)],check=True)
            assert digest(dst)==wanted;frozen[str(dst)]=wanted;mapping[name]=str(dst)
        source_hashes={str(x.relative_to(profile)):digest(x) for x in profile.rglob('*') if x.is_file()}
        shutil.copytree(profile,out/'profile');(out/'profile/.lock').unlink(missing_ok=True);(out/'ui').mkdir()
        manifest={'baseline':args.baseline,'payloadBaseline':args.payload_baseline,'expectedChecks':16,'compileCommand':cmd,'compiledInputHashes':compiled,'inputHashes':frozen,'inputSnapshotMap':mapping,
                  'executableSha256':digest(exe),'bundleIdentifier':bundle,'sourceProfile':str(profile),'sourceProfileHashes':source_hashes,
                  'librarySourceSha256':digest(ROOT/'app/Sources/VixenSequencer/LibraryPanels.swift')}
        (out/'prepared.json').write_text(json.dumps(manifest,indent=2)+'\n')
    if args.prepare_only:print(out/'prepared.json');return
    m=json.loads((out/'prepared.json').read_text())
    assert m['baseline']==args.baseline and m.get('payloadBaseline',False)==args.payload_baseline and digest(exe)==m['executableSha256']
    assert all(digest(Path(k))==v for k,v in m['inputHashes'].items())
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(VIXEN_UI_TEST='private-callbacks',VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_DATA=str(out/'profile'),
               VIXEN_TEST_FILE=str(out/'exchange.vfc'),VIXEN_MUTE='1')
    with (out/'process.log').open('w') as log:
        process=subprocess.run([str(exe),str(out/'native-result.json')],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=35)
    assert process.returncode==0,(process.returncode,out/'process.log')
    report=json.loads((out/'native-result.json').read_text())
    assert len(report['checks'])==m.get('expectedChecks',15) and not report['appActive'] and report['frontmostPID']!=report['ownPID']
    failures=[x['name'] for x in report['checks'] if not x['passed']]
    if args.payload_baseline:
        assert failures==['pending_edit_drag_payload_uses_committed_color'],failures
    elif args.baseline:
        assert {'pending_edit_single_reorder','pending_edit_remove_edited','pending_edit_receive_before_target'} <= set(failures),failures
    else:assert not failures,failures
    assert digest(exe)==m['executableSha256'] and all(digest(Path(k))==v for k,v in m['inputHashes'].items())
    assert {str(x.relative_to(profile)):digest(x) for x in profile.rglob('*') if x.is_file()}==m['sourceProfileHashes']
    report.update(expectedOutcomeObserved=True,baseline=args.baseline,inputsUnchanged=True,executableSha256=m['executableSha256'])
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'expectedOutcomeObserved':True,'baseline':args.baseline,'checks':len(report['checks']),'failures':failures}))


if __name__=='__main__':main()
