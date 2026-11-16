#!/usr/bin/env python3
"""Synthetic, isolated validation tests; never produces actual Windows evidence."""
import argparse
import copy
import json
from pathlib import Path
import zipfile
from validate_raster_detail_return import SOURCE, grid, validate, requested
from raster_picture_common import Files, ROOT, digest, parse_json, require


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--mock-results',type=Path,required=True);parser.add_argument('--out',type=Path,required=True);a=parser.parse_args();out=a.out.resolve()
    require(out.is_relative_to(ROOT/'target') and not out.exists(),'fresh target output');out.mkdir(parents=True)
    raw={p.relative_to(a.mock_results).as_posix():p.read_bytes() for p in a.mock_results.rglob('*') if p.is_file()}
    def put(name,value):raw[name]=(json.dumps(value,separators=(',',':'))+'\n').encode()
    def state(w=40,h=40):return dict(smoothing='None',pixelOffset='Default',interpolation='Bilinear',compositingMode='SourceOver',compositingQuality='Default',pageUnit='Display',pageScale=1,dpiX=96,dpiY=96,transform=[1,0,0,1,0,0],clipBounds=[0,0,w,h],visibleClipBounds=[0,0,w,h],clipIsInfinite=True)
    runtime=parse_json(raw['preflight.json']);runtime.update(syntheticTestFixture=True,os='Microsoft Windows NT synthetic consumer fixture',clr='4.0.30319.42000',is64BitProcess=True,is64BitOperatingSystem=True,drawingIdentity='System.Drawing, Version=4.0.0.0, synthetic',drawingPath='C:\\synthetic\\System.Drawing.dll',gdiplusPath='C:\\synthetic\\gdiplus.dll',gdiplusVersion='synthetic',freshGraphicsDefaults=state(8,8));put('preflight.json',runtime)
    summary=parse_json(raw['capture-summary.json'],2097152)
    cases=grid()[0]['cases'];outcomes={c['id']:c for c in summary['cases']}
    for case in cases:
        prefix='cases/'+case['id']+'/';manifest=parse_json(raw[prefix+'manifest.json']);width,height=(case['maxWidth'],case['maxHeight']) if case['operation']=='resize-exact' else (case['width'],case['height']);manifest['freshGraphicsDefaults']=state(width,height)
        if case['operation']!='grayscale':manifest['actualDrawingDefaults']=state(width,height)
        if case['operation']=='ellipse':
            trace=parse_json(raw[prefix+'path-trace.json'])
            for item in trace['paths']:item['drawingDefaults']=state()
            put(prefix+'path-trace.json',trace)
            for record in manifest['files']:
                if record['path']=='path-trace.json':record.update(bytes=len(raw[prefix+record['path']]),sha256=digest(raw[prefix+record['path']]))
        put(prefix+'manifest.json',manifest);outcomes[case['id']]['manifestSha256']=digest(raw[prefix+'manifest.json'])
    summary['compilerVersion']='synthetic';put('capture-summary.json',summary)
    # The consumer reads snapshots, not the changing live summary/logs.
    raw={name:value for name,value in raw.items() if name!='summary.json'}
    put('return-manifest.json',dict(schema=1,files=[dict(path=n,bytes=len(v),sha256=digest(v)) for n,v in sorted(raw.items())]))
    archive=out/'synthetic-only.zip'
    with zipfile.ZipFile(archive,'x',zipfile.ZIP_DEFLATED) as z:
        for name,value in raw.items():z.writestr(name,value)
    files=Files(archive);checks=[]
    def check(name,value):require(value,name);checks.append(name)
    def rejected(f):
        try:return not validate(f,synthetic_fixture=True)['passed']
        except (ValueError,KeyError,TypeError,OverflowError):return True
    report=validate(files,synthetic_fixture=True);check('full80 synthetic detail accepted only as synthetic',report['passed'] and not report['validCompleteWindowsCapture'] and not report['windowsParityVerified'])
    try:validate(files);check('synthetic fixture cannot claim Windows evidence',False)
    except ValueError:check('synthetic fixture cannot claim Windows evidence',True)
    check('80 independent requested buffers match C# generator',all(files.read('cases/'+c['id']+'/input-requested.argb')==requested(c) for c in cases))
    for c in cases[60:64]:
        buf=requested(c);check('explicit alpha rows '+c['id'],[buf[y*64] for y in range(3)]==[0,c['constantAlpha'],255])
    first=cases[0]['id'];ellipse=cases[64]['id'];gray=cases[60]['id']
    def mutate_json(f,name,fn):
        value=f.json(name,2097152);fn(value);f.raw[name]=json.dumps(value).encode()
    tests=[
        ('summary count','capture-summary.json',lambda d:d.update(completedCases=79)),
        ('summary comparison claim','capture-summary.json',lambda d:d.update(comparisonPerformed=True)),
        ('duplicate outcome','capture-summary.json',lambda d:d['cases'].__setitem__(1,d['cases'][0])),
        ('missing case stage','capture-summary.json',lambda d:d['stages'].pop(3)),
        ('preflight host','preflight.json',lambda d:d.update(hostSha256='0'*64)),
        ('wrong output dimensions','cases/'+first+'/manifest.json',lambda d:d.update(outputWidth=7)),
        ('readback flag false','cases/'+first+'/manifest.json',lambda d:d.update(outputReadbacksEqual=False)),
        ('resize state missing','cases/'+first+'/manifest.json',lambda d:d.update(actualDrawingDefaults=None)),
        ('gray false state scope','cases/'+gray+'/manifest.json',lambda d:d.update(drawingStateScope='actual-before-identity-transform-fixture')),
        ('source hash','cases/'+first+'/manifest.json',lambda d:d.update(originalMethodsSha256='0'*64)),
        ('raw duplicate','cases/'+first+'/manifest.json',lambda d:d['files'].__setitem__(1,d['files'][0])),
        ('path point bits','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['pointBits'].__setitem__(0,1)),
        ('path reserved type2','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(1,2)),
        ('path missing Start','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(0,1)),
        ('path later Start','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(1,0)),
        ('path early closure','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(1,129)),
        ('path no final closure','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(3,1)),
        ('flattened Bezier','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][1].update(types=[0,3,3,131])),
        ('original incomplete Bezier','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(1,3)),
        ('path unknown type','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['types'].__setitem__(0,99)),
        ('path pen state','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0].update(penWidth=2)),
        ('path graphics state','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0].update(drawingDefaults=None)),
        ('path nonidentity transform','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][0]['drawingDefaults'].update(transform=[1,0,0,1,1,0])),
        ('resize interpolation','cases/'+first+'/manifest.json',lambda d:d['actualDrawingDefaults'].update(interpolation='NearestNeighbor')),
        ('resize clip','cases/'+first+'/manifest.json',lambda d:d['actualDrawingDefaults'].update(visibleClipBounds=[0,0,1,1])),
        ('path flatness','cases/'+ellipse+'/path-trace.json',lambda d:d['paths'][1].update(flatness=.1)),
    ]
    for name,path,fn in tests:
        f=copy.deepcopy(files);mutate_json(f,path,fn)
        # Update case-file and outcome hashes so semantic negatives reach their
        # actual field checks, not only the enclosing checksum check.
        if path.startswith('cases/'):
            parts=path.split('/');prefix='/'.join(parts[:2])+'/'
            if parts[-1]!='manifest.json':
                def update(d):
                    for rec in d['files']:
                        if rec['path']==parts[-1]:rec.update(bytes=len(f.raw[path]),sha256=digest(f.raw[path]))
                mutate_json(f,prefix+'manifest.json',update)
            def update_outcome(d):
                for row in d['cases']:
                    if row['id']==parts[1]:row['manifestSha256']=digest(f.raw[prefix+'manifest.json'])
            mutate_json(f,'capture-summary.json',update_outcome)
        check('reject '+name,rejected(f))
    f=copy.deepcopy(files);del f.raw['cases/'+ellipse+'/path-original-getpixel.argb'];check('reject missing native path pixels',rejected(f))
    f=copy.deepcopy(files);f.raw['cases/unknown/manifest.json']=b'{}';check('reject unknown case',rejected(f))
    f=copy.deepcopy(files);f.return_integrity=False;check('reject unverified return manifest',rejected(f))
    # Tamper an archived byte without rewriting its manifest: exercise the real
    # ZIP/return-manifest reader separately from semantic mutation tests.
    bad=out/'synthetic-bad-checksum.zip'
    with zipfile.ZipFile(bad,'x',zipfile.ZIP_DEFLATED) as z:
        for name,value in raw.items():z.writestr(name,value+b' ' if name=='preflight.json' else value)
    try:Files(bad);check('reject archive checksum mismatch',False)
    except ValueError:check('reject archive checksum mismatch',True)
    result=dict(passed=True,checks=checks,sourceHashes={str(p.relative_to(ROOT)):digest(p.read_bytes()) for p in [Path(__file__),ROOT/'scripts/validate_raster_detail_return.py',ROOT/'scripts/raster_picture_common.py',SOURCE/'cases.json',SOURCE/'generate_cases.py',SOURCE/'source-contract.json']},syntheticOnly=True,actualWindowsEvidence=False,scope='One real synthetic ZIP snapshot plus in-memory semantic negatives; original schema2 consumer unchanged.')
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(dict(passed=True,checks=len(checks),result=str(out/'result.json'))))

if __name__=='__main__':main()
