#!/usr/bin/env python3
"""Fault checks using explicit synthetic Windows-shaped data; never Windows parity evidence."""
import argparse
import copy
import json
from pathlib import Path
import shutil
import tempfile
import zipfile
from compare_picture_rasters import compare
from raster_picture_common import ROOT, Files, cases, digest, difference, MAC_NAMES


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--mac',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--cases',type=Path,default=ROOT/'tools/mac-raster/picture36/cases.json');p.add_argument('--source-contract',type=Path,default=ROOT/'tools/mac-raster/picture36/source-contract.json');a=p.parse_args()
    assert not a.out.exists();a.out.mkdir(parents=True)
    grid,_=cases(a.cases);source_contract=a.source_contract;contract=json.loads(source_contract.read_text());schema=grid['schema'];case_count=len(grid['cases'])
    sources=[ROOT/'scripts/raster_picture_common.py',ROOT/'scripts/compare_picture_rasters.py',Path(__file__).resolve(),a.cases.resolve(),source_contract.resolve()]
    source_hashes={str(path):digest(path.read_bytes()) for path in sources}
    checks=[]
    with tempfile.TemporaryDirectory(prefix='vixen-raster-consumer-',dir='/private/tmp') as temporary:
        root=Path(temporary);original=root/'synthetic';original.mkdir();host=b'MZ-SYNTHETIC-CONSUMER-TEST-NOT-EXECUTABLE';(original/'RasterCapture.exe').write_bytes(host)
        defaults=dict(smoothing='None',pixelOffset='Default',interpolation='Bilinear',compositingMode='SourceOver',compositingQuality='Default',pageUnit='Display',pageScale=1,dpiX=96,dpiY=96,transform=[1,0,0,1,0,0])
        if schema==2:defaults.update(clipBounds=[0,0,40,40],visibleClipBounds=[0,0,40,40],clipIsInfinite=True)
        preflight=dict(schema=1,status='available',os='Microsoft Windows NT SYNTHETIC',clr='4.0.TEST',hostSha256=digest(host),drawingIdentity='System.Drawing, Version=4.0.0.0, SYNTHETIC',drawingPath='SYNTHETIC',gdiplusPath='SYNTHETIC',gdiplusVersion='TEST',is64BitProcess=True,is64BitOperatingSystem=True,drawingSha256='1'*64,gdiplusSha256='2'*64,freshGraphicsDefaults=defaults,syntheticTestFixture=True)
        (original/'preflight.json').write_text(json.dumps(preflight));outcomes=[]
        for c in grid['cases']:
            src=a.mac/'cases'/c['id'];dest=original/'cases'/c['id'];dest.mkdir(parents=True)
            for n in MAC_NAMES:
                output=n.replace('to-rgba','fastpixel');(dest/output).write_bytes((src/n).read_bytes())
            r=json.loads((src/'manifest.json').read_text());r.update(inputStride=c['width']*4,outputStride=r['outputWidth']*4,inputPixelFormat='Format32bppArgb',outputPixelFormat='Format32bppArgb',freshGraphicsDefaults=defaults,hostSha256=digest(host),originalMethodsSha256=contract['generated']['OriginalPictureMethods.cs.in'],fastPixelSha256=contract['generated']['OriginalFastPixel.cs.in'])
            if schema==2:r.update(countDownMethodsSha256=contract['generated']['OriginalCountDownMethods.cs.in'],actualDrawingDefaults=defaults if c['family']!='picture' else None,drawingStateScope='actual-before-identity-transform-fixture' if c['family']!='picture' else 'fresh-defaults-only-original-picture-body-uninstrumented')
            r['files']=[dict(path=n.replace('to-rgba','fastpixel'),bytes=(dest/n.replace('to-rgba','fastpixel')).stat().st_size,sha256=digest((dest/n.replace('to-rgba','fastpixel')).read_bytes())) for n in sorted(MAC_NAMES)]
            (dest/'manifest.json').write_text(json.dumps(r));outcomes.append(dict(id=c['id'],status='captured',manifestSha256=digest((dest/'manifest.json').read_bytes())))
        stages=[]
        for name in ['integrity','compile-raster','preflight']+['case-'+c['id'] for c in grid['cases']]+['integrity-final']:
            stage=dict(name=name,status='completed',exitCode=0,timedOut=False,outputLimit=False,overallExpired=False,elapsedSeconds=0.01,budgetSeconds=10,pid=1,error=None)
            stages.append(stage);(original/(name+'.stage.json')).write_text(json.dumps(stage))
        summary=dict(schema=schema,version='raster-'+str(schema),status='captured-comparison-pending',expectedCases=case_count,completedCases=case_count,validWindowsRasterCaptured=True,comparisonPerformed=False,cases=outcomes,stages=stages,compilerPath='SYNTHETIC',compilerVersion='TEST',compilerSha256='3'*64)
        (original/'capture-summary.json').write_text(json.dumps(summary))
        def run(path):return compare(a.mac,path,a.cases,source_contract,synthetic_fixture=True)
        baseline=run(original);assert baseline['passed'] and not baseline['windowsParityVerified'];checks.append('exact synthetic fixture validates without Windows parity claim')
        try:compare(a.mac,original,a.cases,source_contract)
        except ValueError:checks.append('normal consumer refuses explicitly synthetic Windows evidence')
        else:raise AssertionError('synthetic evidence accepted as Windows')
        def mutate_case(directory,fn,index=0):
            spec=grid['cases'][index];folder=directory/'cases'/spec['id'];manifest=folder/'manifest.json';r=json.loads(manifest.read_text());fn(folder,r);manifest.write_text(json.dumps(r))
            s=json.loads((directory/'capture-summary.json').read_text());s['cases'][index]['manifestSha256']=digest(manifest.read_bytes());(directory/'capture-summary.json').write_text(json.dumps(s))
        def seal(folder,r):
            for item in r['files']:
                path=folder/item['path'];item.update(bytes=path.stat().st_size,sha256=digest(path.read_bytes()))
        def fault(name,change,expected='invalid'):
            folder=root/'case';shutil.copytree(original,folder)
            try:
                change(folder)
                try:report=run(folder)
                except Exception:
                    assert expected=='invalid',name
                else:
                    assert not report['passed'] and not report['windowsParityVerified'],name
                    assert report['status']==('different' if expected=='different' else 'partial-or-invalid'),(name,report['status'])
                checks.append(name)
            finally:shutil.rmtree(folder)
        first=grid['cases'][0]['id']
        fault('missing case manifest rejected',lambda d:(d/'cases'/first/'manifest.json').unlink())
        fault('missing raw file rejected',lambda d:(d/'cases'/first/'output-getpixel.argb').unlink())
        fault('corrupt raw hash rejected',lambda d:(d/'cases'/first/'output-getpixel.argb').write_bytes(b'bad'))
        fault('unexpected case file rejected',lambda d:(d/'cases'/first/'extra.argb').write_bytes(b'bad'))
        fault('malformed case JSON rejected',lambda d:(d/'cases'/first/'manifest.json').write_text('{'))
        fault('duplicate JSON key rejected',lambda d:(d/'cases'/first/'manifest.json').write_text('{"schema":1,"schema":1}'))
        fault('duplicate raw records rejected',lambda d:mutate_case(d,lambda f,r:r['files'].__setitem__(1,copy.deepcopy(r['files'][0]))))
        fault('traversal raw record rejected',lambda d:mutate_case(d,lambda f,r:r['files'][0].update(path='../outside.argb')))
        fault('wrong original method hash rejected',lambda d:mutate_case(d,lambda f,r:r.update(originalMethodsSha256='0'*64)))
        fault('nonpositive output dimensions rejected',lambda d:mutate_case(d,lambda f,r:r.update(outputWidth=0)))
        fault('wrong stride rejected',lambda d:mutate_case(d,lambda f,r:r.update(inputStride=1)))
        fault('wrong pixel format rejected',lambda d:mutate_case(d,lambda f,r:r.update(outputPixelFormat='Format32bppPArgb')))
        fault('wrong input case rejected',lambda d:mutate_case(d,lambda f,r:r['input'].update(alpha=True)))
        fault('oversized manifest rejected',lambda d:(d/'cases'/first/'manifest.json').write_text(' '*65537))
        fault('missing preflight rejected',lambda d:(d/'preflight.json').unlink())
        def rewrite(d,n,fn):
            path=d/n;r=json.loads(path.read_text());fn(r);path.write_text(json.dumps(r))
        fault('wrong runtime rejected',lambda d:rewrite(d,'preflight.json',lambda r:r.update(clr='10.0')))
        fault('missing runtime architecture rejected',lambda d:rewrite(d,'preflight.json',lambda r:r.pop('is64BitProcess')))
        fault('changed host rejected',lambda d:(d/'RasterCapture.exe').write_bytes(b'changed'))
        fault('duplicate summary cases rejected',lambda d:rewrite(d,'capture-summary.json',lambda r:r['cases'].__setitem__(1,copy.deepcopy(r['cases'][0]))))
        fault('missing summary outcome rejected',lambda d:rewrite(d,'capture-summary.json',lambda r:r['cases'].pop()))
        fault('false captured flag rejected',lambda d:rewrite(d,'capture-summary.json',lambda r:r.update(validWindowsRasterCaptured=False)))
        fault('missing compiler stage rejected',lambda d:rewrite(d,'capture-summary.json',lambda r:r['stages'].pop(1)))
        fault('duplicate stage rejected',lambda d:rewrite(d,'capture-summary.json',lambda r:r['stages'].__setitem__(1,copy.deepcopy(r['stages'][0]))))
        fault('stage file tampering rejected',lambda d:rewrite(d,'compile-raster.stage.json',lambda r:r.update(exitCode=1)))
        def failed_stage(d,name,**changes):
            rewrite(d,name+'.stage.json',lambda r:r.update(**changes))
            rewrite(d,'capture-summary.json',lambda r:next(s for s in r['stages'] if s['name']==name).update(**changes))
        fault('compiler timeout rejected',lambda d:failed_stage(d,'compile-raster',timedOut=True))
        fault('captured case with output limit rejected',lambda d:failed_stage(d,'case-'+first,outputLimit=True))
        final_failed=root/'final-failed';shutil.copytree(original,final_failed);failed_stage(final_failed,'integrity-final',status='failed',exitCode=1)
        final_report=run(final_failed)
        assert final_report['status']=='partial-or-invalid' and not final_report['windowsParityVerified'] and len(final_report['captureIssues'])==1 and all(r['status']=='exact' for r in final_report['cases'])
        checks.append('failed final integrity preserves cases diagnostically without parity')
        def partial(r):
            r.update(status='partial-or-diagnostics-only',completedCases=case_count-1);r['cases'][0].update(status='timed-out',error='synthetic timeout')
        fault('partial capture never accepted',lambda d:rewrite(d,'capture-summary.json',partial))
        partial_path=root/'partial';shutil.copytree(original,partial_path);rewrite(partial_path,'capture-summary.json',partial)
        partial_report=run(partial_path)
        assert partial_report['status']=='partial-or-invalid' and not partial_report['windowsParityVerified'] and sum(r['status']=='exact' for r in partial_report['cases'])==case_count-1
        checks.append('valid completed cases retained diagnostically in partial capture')
        def unknown(d):
            (d/'cases'/'unexpected').mkdir();(d/'cases'/'unexpected'/'x').write_bytes(b'')
        fault('unknown case directory rejected',unknown)
        def mismatch(f,r):
            for n in ['output-getpixel.argb','output-fastpixel.argb']:
                raw=bytearray((f/n).read_bytes());raw[1]^=1;(f/n).write_bytes(raw)
            seal(f,r)
        fault('valid changed color reports exact difference',lambda d:mutate_case(d,mismatch),expected='different')
        def alpha_mismatch(f,r):
            for n in ['input-getpixel.argb','input-fastpixel.argb']:
                (f/n).write_bytes((f/'input-requested.argb').read_bytes())
            seal(f,r)
        fault('hidden RGB at zero alpha remains a difference',lambda d:mutate_case(d,alpha_mismatch,index=9),expected='different')
        def dim_mismatch(f,r):
            r['outputWidth']+=1;r['outputStride']=r['outputWidth']*4
            for n in ['output-getpixel.argb','output-fastpixel.argb']:(f/n).write_bytes(bytes(r['outputWidth']*r['outputHeight']*4))
            seal(f,r)
        fault('valid changed dimensions reports difference',lambda d:mutate_case(d,dim_mismatch),expected='different')
        def requested_wrong(f,r):
            raw=bytearray((f/'input-requested.argb').read_bytes());raw[0]^=1;(f/'input-requested.argb').write_bytes(raw);seal(f,r)
        fault('rehash cannot hide changed requested input',lambda d:mutate_case(d,requested_wrong))
        def api_disagree(f,r):
            raw=bytearray((f/'output-fastpixel.argb').read_bytes());raw[0]^=1;(f/'output-fastpixel.argb').write_bytes(raw);seal(f,r)
        fault('FastPixel/GetPixel discrepancy retained as invalid',lambda d:mutate_case(d,api_disagree))
        if schema==2:
            fault('wrong CountDown source hash rejected',lambda d:mutate_case(d,lambda f,r:r.update(countDownMethodsSha256='0'*64),index=36))
            fault('missing effective nonpicture drawing state rejected',lambda d:mutate_case(d,lambda f,r:r.update(actualDrawingDefaults=None),index=36))
            fault('wrong nonpicture drawing scope rejected',lambda d:mutate_case(d,lambda f,r:r.update(drawingStateScope='fresh-only'),index=36))
            fault('Picture internal state claim rejected',lambda d:mutate_case(d,lambda f,r:r.update(actualDrawingDefaults=defaults)))
            fault('wrong Picture drawing scope rejected',lambda d:mutate_case(d,lambda f,r:r.update(drawingStateScope='actual-before-identity-transform-fixture')))
            fault('missing clipping state rejected',lambda d:mutate_case(d,lambda f,r:r['freshGraphicsDefaults'].pop('clipBounds'),index=36))
            fault('changed family rejected',lambda d:mutate_case(d,lambda f,r:r['input'].update(family='primitive'),index=36))
            fault('changed nonpicture blank input rejected',lambda d:mutate_case(d,requested_wrong,index=36))
            assert baseline['families']['countdown']['cases']==20 and baseline['families']['primitive']['cases']==18
            assert all(r['outputComparison']=='Windows raw GDI vs actual CountDown Canvas post-mask' for r in baseline['cases'] if r['family']=='countdown')
            checks.append('CountDown post-mask comparison and primitive family remain distinct')
        fault('symlink raw input rejected',lambda d:((d/'cases'/first/'output-getpixel.argb').unlink(),(d/'cases'/first/'output-getpixel.argb').symlink_to('/etc/hosts')))
        def archive(path,records=None,unsafe=None,missing=False):
            files={p.relative_to(original).as_posix():p.read_bytes() for p in original.rglob('*') if p.is_file()}
            manifest=dict(schema=1,files=[dict(path=n,bytes=len(b),sha256=digest(b)) for n,b in files.items()])
            if records:records(manifest)
            with zipfile.ZipFile(path,'w') as z:
                for n,b in files.items():z.writestr(n,b)
                if not missing:z.writestr('return-manifest.json',json.dumps(manifest))
                if unsafe:z.writestr(unsafe,b'bad')
        zip_path=root/'valid.zip';archive(zip_path);zr=run(zip_path);assert zr['passed'] and zr['windowsReturnIntegrityVerified'] and not zr['windowsParityVerified'];checks.append('read-only ZIP return integrity validates')
        for name,kwargs in [('archive missing manifest',dict(missing=True)),('archive traversal',dict(unsafe='../outside')),('archive unlisted file',dict(unsafe='extra')),('archive duplicate record',dict(records=lambda m:m['files'].append(m['files'][0]))),('archive corrupt hash',dict(records=lambda m:m['files'][0].update(sha256='0'*64)))]:
            path=root/'invalid.zip';archive(path,**kwargs)
            try:run(path)
            except Exception:checks.append(name+' rejected')
            else:raise AssertionError(name)
            path.unlink()
        sample=difference(bytes([0,1,2,3,255,2,3,4]),bytes([1,2,3,4,255,2,3,4]),2,1)
        assert sample['differingPixels']==1 and sample['differingChannels']==4 and sample['channels']==dict(A=1,R=1,G=1,B=1) and sample['maxChannelDelta']==1 and sample['mismatchBounds']==dict(minX=0,minY=0,maxX=0,maxY=0)
        checks.append('ARGB channel counts coordinates bounds and deltas are exact')
    for path,expected in source_hashes.items():
        assert digest(Path(path).read_bytes())==expected,'consumer source changed during checks'
    report=dict(passed=True,checks=checks,count=len(checks),schemaUnderTest=schema,caseCount=case_count,sourceHashes=source_hashes,macCaptureSummarySha256=digest((a.mac/'capture-summary.json').read_bytes()),windowsDrawingExecuted=False,windowsParityVerified=False,syntheticFixturesOnly=True)
    (a.out/'result.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(passed=True,checks=len(checks),windowsParityVerified=False)))
if __name__=='__main__':main()
