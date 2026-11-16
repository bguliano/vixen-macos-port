#!/usr/bin/env python3
"""Render the fixed36 Picture cases through actual Rust helpers; no UI/audio/network."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
from raster_picture_common import ROOT, cases, digest, require, requested, difference, MAC_NAMES

SOURCES = ['Cargo.lock','crates/fx-pixel-graphics/src/lib.rs','crates/fx-pixel-graphics/src/picture.rs',
           'crates/fx-pixel-graphics/src/raster_capture.rs','crates/fx-pixel-graphics/src/count_down.rs',
           'crates/fx-pixel-graphics/src/textdraw.rs','crates/vixen-effects/src/gfx.rs',
           'scripts/capture_picture_rasters.py','scripts/raster_picture_common.py']

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--out',type=Path,required=True)
    p.add_argument('--cases',type=Path,default=ROOT/'tools/mac-raster/picture36/cases.json')
    p.add_argument('--target-dir',type=Path,default=ROOT/'target/fx-pixel-graphics')
    a=p.parse_args();out=a.out.resolve();require(not out.exists() and out.is_relative_to(ROOT/'target'),'fresh target output required')
    doc,case_hash=cases(a.cases)
    hashes={name:digest((ROOT/name).read_bytes()) for name in SOURCES}
    out.mkdir(parents=True)
    (out/'cases.json').write_bytes(a.cases.read_bytes())
    env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
    env.update(RUSTUP_HOME=str(ROOT/'.tools/rustup'),CARGO_HOME=str(ROOT/'.tools/cargo'),CARGO_TARGET_DIR=str(a.target_dir.resolve()),CARGO_BUILD_JOBS='4',CARGO_NET_OFFLINE='true')
    env['PATH']=str(ROOT/'.tools/cargo/bin')+os.pathsep+env.get('PATH','')
    command=[str(ROOT/'.tools/cargo/bin/cargo'),'test','--offline','--locked','-p','fx-pixel-graphics','--lib','--no-run','--message-format=json']
    with (out/'build.jsonl').open('w') as log,(out/'build.stderr').open('w') as err:
        subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=err,check=True,timeout=300)
    binaries=[]
    for line in (out/'build.jsonl').read_text().splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact' and row.get('executable') and row.get('profile',{}).get('test') and row.get('target',{}).get('name')=='fx_pixel_graphics':
            binaries.append(Path(row['executable']))
    require(len(binaries)==1,'expected exactly one test executable')
    binary=binaries[0];binary_hash=digest(binary.read_bytes())
    render=out/'render'
    env.update(VIXEN_RASTER_CASES=str((out/'cases.json').resolve()),VIXEN_RASTER_OUT=str(render))
    with (out/'render.log').open('w') as log:
        subprocess.run([str(binary),'--exact','raster_capture::capture_picture_cases','--ignored','--nocapture'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=30)
    rendered=json.loads((render/'render.json').read_text())
    require(len(rendered['cases'])==len(doc['cases']),'partial render')
    rows=[]
    for case,record in zip(doc['cases'],rendered['cases']):
        require(case==record['input'],'render case order/identity')
        folder=render/'cases'/case['id'];requested_raw=(folder/'input-requested.argb').read_bytes()
        require(requested_raw==requested(case),'Rust/Python requested input disagrees')
        record.update(status='complete',caseGridSha256=case_hash,files=[dict(path=n,bytes=(folder/n).stat().st_size,sha256=digest((folder/n).read_bytes())) for n in sorted(MAC_NAMES)])
        (folder/'manifest.json').write_text(json.dumps(record,indent=2)+'\n')
        baseline=difference(requested_raw,(folder/'input-getpixel.argb').read_bytes(),case['width'],case['height'])
        rows.append(dict(id=case['id'],family=case.get('family','picture'),alpha=case['alpha'],outputWidth=record['outputWidth'],outputHeight=record['outputHeight'],baselineRequestedVsObserved=baseline,manifestSha256=digest((folder/'manifest.json').read_bytes())))
    require(all(digest((ROOT/n).read_bytes())==h for n,h in hashes.items()),'source changed during build/render')
    require(digest(binary.read_bytes())==binary_hash,'test executable changed')
    for name,expected in hashes.items():
        saved=out/'source-snapshot'/name;saved.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/name,saved)
        require(digest(saved.read_bytes())==expected,'source snapshot mismatch')
    frozen_binary=out/'graphics-raster-test';shutil.copy2(binary,frozen_binary)
    require(digest(frozen_binary.read_bytes())==binary_hash,'frozen test executable mismatch')
    report=dict(schema=doc['schema'],status='mac-render-complete-windows-comparison-pending',passed=True,windowsParityVerified=False,byteOrder=doc['byteOrder'],caseGridSha256=case_hash,caseCount=len(rows),cases=rows,sourceHashes=hashes,sourceSnapshot=str(out/'source-snapshot'),testExecutable=str(frozen_binary),buildExecutable=str(binary),testExecutableSha256=binary_hash,host=dict(system=platform.system(),release=platform.release(),machine=platform.machine()),scope='Actual production Picture/gfx and schema2 CountDown helpers via a cfg(test)-only ignored diagnostic. CountDown is actual Canvas post-mask; primitives are direct gfx. No GDI, UI, audio, complete effect lifecycle or Windows parity claim.')
    (render/'capture-summary.json').write_text(json.dumps(report,indent=2)+'\n')
    (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(passed=True,cases=len(rows),windowsParityVerified=False,output=str(render))))
if __name__=='__main__':main()
