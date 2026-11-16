#!/usr/bin/env python3
"""Synthetic schema controls only. Pixels are arbitrary, never Windows evidence."""
from pathlib import Path
import argparse,copy,json,sys,zipfile
from contract import cases,requested
from validate_return import validate,drawing_labels,Files,digest

def encoded(v):return json.dumps(v,separators=(',',':')).encode()
def defaults(w,h,aa=False):return dict(smoothing='AntiAlias' if aa else 'None',pixelOffset='Default',interpolation='Bilinear',compositingMode='SourceOver',compositingQuality='Default',pageUnit='Display',pageScale=1,dpiX=96,dpiY=96,transform=[1,0,0,1,0,0],clipBounds=[-4194304,-4194304,8388608,8388608],visibleClipBounds=[0,0,w,h],clipIsInfinite=True)
def fixture():
 raw={'RasterCapture.exe':b'SYNTHETIC TEST ONLY - NOT WINDOWS CODE'};host=digest(raw['RasterCapture.exe']);grid=cases()['cases'];outcomes=[]
 for c in grid:
  prefix='cases/'+c['id']+'/';records=[];obs=[];raw[prefix+'input-requested.argb']=requested(c)
  for o in c['observations']:
   name=o['label']+'-'+o['api']+'.argb';raw[prefix+name]=requested(c)[:o['width']*o['height']*4];obs.append(dict(**o,file=name,stride=0 if o['api']=='getpixel' else o['width']*4,pixelFormat='Format32bppArgb'))
  for name,data in list(raw.items()):
   if name.startswith(prefix):records.append(dict(path=name[len(prefix):],bytes=len(data),sha256=digest(data)))
  r=dict(schema=1,status='complete',id=c['id'],input=c,byteOrder='ARGB8-row-major-top-down',hostSha256=host,originalMethodsSha256=digest(Path(__file__).with_name('OriginalPictureMethods.cs.in').read_bytes()),events=c['events'],observations=obs,drawingStates=[dict(label=n,state=defaults(c['width'],c['height'],n.startswith('antialias-'))) for n in drawing_labels(c)],files=records)
  raw[prefix+'manifest.json']=encoded(r);outcomes.append(dict(id=c['id'],status='captured',error=None,manifestSha256=digest(raw[prefix+'manifest.json'])))
 stages=[]
 for name in ['integrity','compile-raster','preflight']+['case-'+c['id'] for c in grid]+['integrity-final']:
  s=dict(name=name,status='completed',error=None,pid=123,exitCode=0,elapsedSeconds=.01,budgetSeconds=5,timedOut=False,overallExpired=False,outputLimit=False);stages.append(s);raw[name+'.stage.json']=encoded(s)
 raw['preflight.json']=encoded(dict(schema=1,status='available',os='Windows synthetic test',clr='4.0.synthetic',hostSha256=host,drawingIdentity='System.Drawing, Version=4.0.0.0, synthetic',drawingPath='C:\\synthetic\\System.Drawing.dll',drawingSha256='1'*64,gdiplusPath='C:\\synthetic\\gdiplus.dll',gdiplusVersion='synthetic',gdiplusSha256='2'*64,is64BitProcess=True,is64BitOperatingSystem=True,freshGraphicsDefaults=defaults(2,2),syntheticTestFixture=True))
 raw['capture-summary.json']=encoded(dict(schema=1,version='bitmap-storage-1',status='captured-comparison-pending',expectedCases=8,completedCases=8,validWindowsRasterCaptured=True,comparisonPerformed=False,cases=outcomes,stages=stages,compilerSha256='3'*64,compilerPath='C:\\synthetic\\csc.exe',compilerVersion='synthetic'))
 return raw
def container(raw):
 f=object.__new__(Files);f.raw=raw;f.return_integrity=True;return f
def update(raw,path,fn):d=json.loads(raw[path]);fn(d);raw[path]=encoded(d)
def case_update(raw,index,fn):
 id=cases()['cases'][index]['id'];p='cases/'+id+'/manifest.json';update(raw,p,fn);update(raw,'capture-summary.json',lambda d:d['cases'][index].update(manifestSha256=digest(raw[p])))
def main():
 p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);a=p.parse_args();assert not a.out.exists();a.out.mkdir(parents=True);checks=[];base=fixture()
 def expect(name,raw,passes=False,partial=False):
  try:r=validate(container(raw),True)
  except (ValueError,KeyError,TypeError):assert not passes and not partial;checks.append(name);return
  assert r['passed'] is passes and not r['validCompleteWindowsCapture'] and not r['windowsParityVerified'];assert passes or partial,name;checks.append(name)
 expect('synthetic exact complete capture accepted only as synthetic',base,True)
 try:validate(container(base))
 except ValueError:checks.append('synthetic runtime refused as actual Windows')
 else:raise AssertionError('synthetic accepted')
 mutations={
 'empty outcomes':lambda r:update(r,'capture-summary.json',lambda d:d.update(cases=[])),
 'truncated outcomes':lambda r:update(r,'capture-summary.json',lambda d:d['cases'].pop()),
 'extra outcome':lambda r:update(r,'capture-summary.json',lambda d:d['cases'].append(d['cases'][0])),
 'duplicate outcome':lambda r:update(r,'capture-summary.json',lambda d:d['cases'].__setitem__(1,d['cases'][0])),
 'observer reordered':lambda r:case_update(r,7,lambda d:d['events'].reverse()),
 'LockBits inserted before direct A draw':lambda r:case_update(r,7,lambda d:d['events'].insert(4,'read:a-before-draw:lockbits-readonly')),
 'missing observer':lambda r:case_update(r,0,lambda d:d['observations'].pop()),
 'wrong raw hash':lambda r:case_update(r,0,lambda d:d['files'][0].update(sha256='0'*64)),
 'duplicate raw record':lambda r:case_update(r,0,lambda d:d['files'].__setitem__(1,d['files'][0])),
 'bad source provenance':lambda r:case_update(r,0,lambda d:d.update(originalMethodsSha256='0'*64)),
 'unexpected smoothing':lambda r:case_update(r,6,lambda d:d['drawingStates'][0]['state'].update(smoothing='None')),
 'missing drawing state':lambda r:case_update(r,7,lambda d:d.update(drawingStates=[])),
 'comparison claim':lambda r:update(r,'capture-summary.json',lambda d:d.update(comparisonPerformed=True)),
 'wrong count':lambda r:update(r,'capture-summary.json',lambda d:d.update(completedCases=7)),
 'new nested raw path':lambda r:r.update({'cases/'+cases()['cases'][0]['id']+'/nested/x':b'x'}),
 'stage sidecar drift':lambda r:update(r,'preflight.stage.json',lambda d:d.update(exitCode=9))}
 for name,fn in mutations.items():r=copy.deepcopy(base);fn(r);expect(name,r)
 r=copy.deepcopy(base);id=cases()['cases'][2]['id'];r.pop('cases/'+id+'/manifest.json');update(r,'capture-summary.json',lambda d:(d.update(status='partial-or-diagnostics-only',completedCases=7),d['cases'][2].update(status='timed-out',manifestSha256=None)))
 expect('partial timeout preserves seven complete cases and raw fragments',r,partial=True)
 r=copy.deepcopy(base);update(r,'capture-summary.json',lambda d:(d.update(status='partial-or-diagnostics-only',completedCases=0,validWindowsRasterCaptured=False,stages=d['stages'][:1]+d['stages'][-1:]),[x.update(status='skipped-dependency',manifestSha256=None) for x in d['cases']]))
 expect('dependency failure has eight explicit skipped outcomes',r,partial=True)
 expect('early bootstrap without summary is diagnostics only',{'bootstrap.txt':b'compiler absent'},partial=True)
 # Cross-API differences are retained, not coerced or hidden by capture validation.
 r=copy.deepcopy(base);c=cases()['cases'][1];p='cases/'+c['id']+'/after-fill-lockbits-readonly.argb';r[p]=b'\x01'+r[p][1:]
 case_update(r,1,lambda d:next(f for f in d['files'] if f['path']=='after-fill-lockbits-readonly.argb').update(sha256=digest(r[p])));expect('actual cross-API discrepancy remains valid evidence',r,True)
 # Exercise the real bounded ZIP reader and its integrity/duplicate checks once.
 records=[dict(path=n,bytes=len(v),sha256=digest(v)) for n,v in sorted(base.items())];archive=a.out/'synthetic-return.zip'
 with zipfile.ZipFile(archive,'x') as z:
  for n,v in base.items():z.writestr(n,v)
  z.writestr('return-manifest.json',encoded(dict(schema=1,files=records)))
 actual=validate(Files(archive),True);assert actual['passed'] and not actual['validCompleteWindowsCapture'];checks.append('actual strict ZIP reader verifies complete synthetic return')
 (a.out/'result.json').write_text(json.dumps(dict(passed=True,checks=checks,count=len(checks),windowsExecuted=False,syntheticOnly=True),indent=2)+'\n');print(json.dumps(dict(passed=True,checks=len(checks))))
if __name__=='__main__':main()
