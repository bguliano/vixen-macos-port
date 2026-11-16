#!/usr/bin/env python3
"""Pin optimized production objects and run one private offscreen inline-strip probe."""
import argparse, hashlib, json, os, plistlib, signal, subprocess, time, uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def digest(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--build',type=Path,default=ROOT/'target/build-release');p.add_argument('--out',type=Path,required=True)
    p.add_argument('--prepare-only',action='store_true');p.add_argument('--run-prepared',action='store_true')
    a=p.parse_args();out=a.out.resolve();build=a.build.resolve()
    assert out.is_relative_to(ROOT/'target') and build.is_relative_to(ROOT/'target')
    app=out/'InlineStrip.app';exe=app/'Contents/MacOS/InlineStrip';manifest=out/'prepared.json'
    if not a.run_prepared:
        assert not out.exists();exe.parent.mkdir(parents=True)
        frozen=out/'compiled-inputs';modules=frozen/'modules';modules.mkdir(parents=True)
        products=build/'swift/out/Products/Release';pins={};originals={}
        def pin(src,dst):
            dst.parent.mkdir(parents=True,exist_ok=True)
            subprocess.run(['/bin/cp','-c',str(src),str(dst)],check=True)
            assert digest(src)==digest(dst)
            pins[str(dst)]=digest(dst);originals[str(src)]=digest(src);return dst
        for module in ['VixenKit','VixenBridge']:
            for src in (products/(module+'.swiftmodule')).rglob('*'):
                if src.is_file():pin(src,modules/(module+'.swiftmodule')/src.relative_to(products/(module+'.swiftmodule')))
        objects=[pin(products/(name+'.o'),frozen/(name+'.o')) for name in ['VixenKit','VixenBridge','vixen_ffiFFI']]
        lib=pin(build/'pkg/lib/libvixen_ffi.a',frozen/'libvixen_ffi.a')
        header=pin(build/'pkg/Sources/vixen_ffiFFI/include/vixen_ffiFFI.h',frozen/'include/vixen_ffiFFI.h')
        pin(build/'pkg/Sources/VixenBridge/vixen_ffi.swift',frozen/'vixen_ffi.swift')
        modulemap=frozen/'vixen_ffiFFI.modulemap';modulemap.write_text('module vixen_ffiFFI { header "'+str(header)+'" export * }\n');pins[str(modulemap)]=digest(modulemap)
        source=pin(ROOT/'scripts/check_inline_strip_native.swift',out/'fixture-source/check_inline_strip_native.swift')
        pin(Path(__file__).resolve(),out/'fixture-source/check_inline_strip_native.py')
        sourcepaths=['app/Sources/VixenKit/CurveStripView.swift','app/Sources/VixenKit/KitSupport.swift','app/Sources/VixenKit/GradientStripView.swift',
            'crates/vixen-session/src/effect_editor/curve_strip.rs','crates/vixen-session/src/effect_editor/gradient_strip.rs','upstream/Vixen/src/Vixen.Modules/Editor/EffectEditor/Controls/SliderPoint.cs',
            'upstream/Vixen/src/Vixen.Modules/Editor/EffectEditor/Controls/BaseInlineCurveEditor.cs','upstream/Vixen/src/Vixen.Modules/Editor/EffectEditor/Controls/BaseInlineGradientEditor.cs','docs/spec/effect-editor.md','docs/parity/effect-editor.md']
        for name in sourcepaths:pin(ROOT/name,out/'source-evidence'/name)
        cmd=['/usr/bin/swiftc','-module-cache-path',str(out/'clang-cache'),'-parse-as-library','-swift-version','6',
            '-default-isolation','MainActor','-target','arm64-apple-macos26.0','-O','-I',str(modules),'-Xcc',
            '-fmodule-map-file='+str(modulemap),'-I',str(header.parent),str(source),*map(str,objects),str(lib),'-lc++','-lcurl']
        for framework in ['AppKit','SwiftUI','AudioToolbox','AudioUnit','CoreAudio','CoreFoundation','CoreGraphics','CoreText','Foundation','IOKit','Metal','QuartzCore','Security','SystemConfiguration']:
            cmd+=['-framework',framework]
        cmd+=['-o',str(exe)]
        with (out/'compile.log').open('w') as log:subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
        bundle='com.vixenlights.verification.inlinestrip.'+uuid.uuid4().hex
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':bundle,'CFBundleExecutable':'InlineStrip','CFBundleName':'Private Inline Strip Handler Check','CFBundlePackageType':'APPL','LSUIElement':True,'LSMinimumSystemVersion':'26.0'}))
        subprocess.run(['/usr/bin/codesign','--force','--sign','-',str(app)],check=True,capture_output=True,timeout=30)
        subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(app)],check=True,capture_output=True,timeout=30)
        m=dict(build=str(build),configuration='release',executable=str(exe),executableSha256=digest(exe),bundleIdentifier=bundle,
            pinnedInputHashes=pins,originalInputHashes=originals,compileCommand=cmd,timeoutSeconds=15,scope=__doc__)
        manifest.write_text(json.dumps(m,indent=2)+'\n')
    if a.prepare_only:print(manifest);return
    m=json.loads(manifest.read_text());assert digest(exe)==m['executableSha256']
    assert all(digest(k)==v for k,v in m['pinnedInputHashes'].items())
    assert not (out/'native-result.json').exists()
    prefs=[Path.home()/'Library/Preferences/io.github.bguliano.vixen-macos-port.plist',Path.home()/'Library/Application Support/Vixen/Settings.xml']
    before={str(p):digest(p) if p.exists() else None for p in prefs}
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    settings=out/'private-settings';settings.mkdir(exist_ok=True)
    env.update(VIXEN_MUTE='1',VIXEN_SETTINGS_DIR=str(settings))
    process=None;started=time.monotonic();status={}
    try:
        with (out/'process.log').open('w') as log:
            process=subprocess.Popen([str(exe),str(out/'native-result.json')],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            status['pid']=process.pid
            status['exitCode']=process.wait(timeout=15)
    finally:
        if process is not None and process.poll() is None:
            os.killpg(process.pid,signal.SIGTERM)
            try:process.wait(timeout=2)
            except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=2)
        status.update(elapsedSeconds=time.monotonic()-started,processExited=process is not None and process.poll() is not None,
            ownerPreferencesBefore=before,ownerPreferencesAfter={str(p):digest(p) if p.exists() else None for p in prefs})
        status['ownerPreferencesUnchanged']=status['ownerPreferencesBefore']==status['ownerPreferencesAfter']
        status['inputsUnchanged']=all(digest(k)==v for k,v in m['pinnedInputHashes'].items())
        status['executableUnchanged']=digest(exe)==m['executableSha256']
        (out/'run.json').write_text(json.dumps(status,indent=2)+'\n')
    assert status['exitCode']==0 and status['ownerPreferencesUnchanged'] and status['inputsUnchanged'] and status['executableUnchanged'],status
    result=json.loads((out/'native-result.json').read_text());result['run']=status;result['preparedSha256']=digest(manifest)
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':result['passed'],'checks':len(result['checks']),'failures':[c['name'] for c in result['checks'] if not c['passed']], 'elapsedSeconds':status['elapsedSeconds']}))
if __name__=='__main__':main()
