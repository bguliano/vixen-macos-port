#!/usr/bin/env python3
"""Strict complete/partial storage capture evidence; no expected drawing output."""
from pathlib import Path
import argparse,json,sys
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'scripts'))
from raster_picture_common import Files,parse_json,require,digest,hash_value,drawing_defaults,BYTE_ORDER
from contract import cases,requested

def drawing_labels(case):
 return [e[10:] for e in case['events'] if e.startswith('operation:') and not e.startswith(('operation:original-grayscale','operation:reconstruct-'))]
def validate_case(files,case,host):
 prefix='cases/'+case['id']+'/';manifest=files.read(prefix+'manifest.json');r=parse_json(manifest)
 require(r.get('schema')==1 and r.get('status')=='complete' and r.get('id')==case['id'] and r.get('input')==case and r.get('byteOrder')==BYTE_ORDER,'case identity/schema')
 require(r.get('events')==case['events'],'exact operation/observer interleaving')
 require(r.get('hostSha256')==host and r.get('originalMethodsSha256')==digest((HERE/'OriginalPictureMethods.cs.in').read_bytes()),'capture source provenance')
 observations=r.get('observations');require(isinstance(observations,list) and len(observations)==len(case['observations']),'observer count')
 expected={'input-requested.argb':case['width']*case['height']*4};buffers={}
 for a,b in zip(observations,case['observations']):
  name=b['label']+'-'+b['api']+'.argb'
  require(all(a.get(k)==v for k,v in b.items()) and a.get('file')==name and a.get('pixelFormat')=='Format32bppArgb','observer identity/format')
  require(type(a.get('stride')) is int and (a['stride']==0 if b['api']=='getpixel' else abs(a['stride'])==b['width']*4),'observer stride')
  expected[name]=b['width']*b['height']*4
 require({n[len(prefix):] for n in files.raw if n.startswith(prefix)}==set(expected)|{'manifest.json'},'exact raw file set')
 records=r.get('files');require(isinstance(records,list) and len(records)==len(expected),'raw record count');pending=set(expected)
 for item in records:
  name=item.get('path');require(name in pending,'duplicate/extra raw record');pending.remove(name);raw=files.read(prefix+name)
  require(type(item.get('bytes')) is int and item['bytes']==len(raw)==expected[name] and item.get('sha256')==digest(raw),'raw length/hash');buffers[name]=raw
 require(buffers['input-requested.argb']==requested(case),'exact requested input digest')
 if case['id']=='storage-08-drawn-source':
  require(buffers['b-before-copy-getpixel.argb']==buffers['c-before-draw-getpixel.argb'],'exact GetPixel/SetPixel reconstruction before later draw')
 states=r.get('drawingStates');labels=drawing_labels(case);require(isinstance(states,list) and [s.get('label') for s in states]==labels,'exact actual drawing-state records')
 for item in states:
  state=item.get('state');drawing_defaults(state,2)
  wanted=dict(smoothing='AntiAlias' if item['label'].startswith('antialias-') else 'None',pixelOffset='Default',interpolation='Bilinear',compositingMode='SourceOver',compositingQuality='Default',pageUnit='Display',pageScale=1,transform=[1,0,0,1,0,0],clipIsInfinite=True,visibleClipBounds=[0,0,case['width'],case['height']])
  require(all(state.get(k)==v for k,v in wanted.items()),'actual fixture graphics settings')
 # No guessed changed/untouched-pixel policy: preserve any cross-API result as evidence.
 mismatches=[]
 for obs in case['observations']:
  if obs['api']=='lockbits-readonly':
   label=obs['label'];a=buffers[label+'-getpixel.argb'];b=buffers[label+'-lockbits-readonly.argb']
   if a!=b:mismatches.append(label)
 seed='gray-input-before' if case['width']==3 else 'seed-before'
 return dict(id=case['id'],status='validated',manifestSha256=digest(manifest),observations=len(observations),seedGetPixelExact=buffers[seed+'-getpixel.argb']==requested(case),seedLockBitsExact=buffers[seed+'-lockbits-readonly.argb']==requested(case),readbackDisagreements=mismatches,outputMathematicsCompared=False)

def validate(files,synthetic_fixture=False):
 require(parse_json((HERE/'cases.json').read_bytes())==cases(),'local contract drift')
 if 'capture-summary.json' not in files.raw:
  return dict(passed=False,status='diagnostics-only',validCompleteWindowsCapture=False,windowsParityVerified=False,filesRetained=len(files.raw),scope='No final eight-case capture summary; partial files retained without success claim.')
 summary=files.json('capture-summary.json');grid=cases()['cases'];outcomes=summary.get('cases')
 require(summary.get('schema')==1 and summary.get('version')=='bitmap-storage-1' and summary.get('expectedCases')==8,'summary schema')
 require(isinstance(outcomes,list) and len(outcomes)==8 and [x.get('id') for x in outcomes]==[c['id'] for c in grid],'exact eight ordered outcomes')
 count=sum(o.get('status')=='captured' for o in outcomes);require(type(summary.get('completedCases')) is int and summary['completedCases']==count and summary.get('validWindowsRasterCaptured') is (count>0) and summary.get('comparisonPerformed') is False,'capture counts/claims')
 stages=summary.get('stages');require(isinstance(stages,list) and len(stages)<=12,'stage count')
 expected_order=['integrity','compile-raster','preflight']+['case-'+c['id'] for c in grid]+['integrity-final'];stage_map={}
 for stage in stages:
  name=stage.get('name');require(name in expected_order and name not in stage_map,'duplicate/unknown stage');stage_map[name]=stage;require(files.json(name+'.stage.json')==stage,'stage sidecar mismatch')
 require([s['name'] for s in stages]==[n for n in expected_order if n in stage_map],'stage order')
 def complete(name):
  s=stage_map.get(name,{});return s.get('status')=='completed' and type(s.get('exitCode')) is int and s['exitCode']==0 and s.get('error') is None and all(s.get(k) is False for k in ['timedOut','overallExpired','outputLimit'])
 ready=all(complete(n) for n in ['integrity','compile-raster','preflight']);runtime=None
 if ready:
  runtime=files.json('preflight.json');require(not runtime.get('syntheticTestFixture') or synthetic_fixture,'synthetic fixture is not Windows evidence')
  require(runtime.get('schema')==1 and runtime.get('status')=='available' and 'Windows' in runtime.get('os','') and runtime.get('clr','').startswith('4.') and runtime.get('drawingIdentity','').startswith('System.Drawing, Version=4.0.0.0,'),'actual Windows Framework provenance')
  for key in ['hostSha256','drawingSha256','gdiplusSha256']:hash_value(runtime.get(key))
  for key in ['drawingPath','gdiplusPath','gdiplusVersion']:require(isinstance(runtime.get(key),str) and 0<len(runtime[key])<=2048,'runtime paths/version')
  require(type(runtime.get('is64BitProcess')) is bool and type(runtime.get('is64BitOperatingSystem')) is bool,'runtime architecture flags')
  hash_value(summary.get('compilerSha256'))
  for key in ['compilerPath','compilerVersion']:require(isinstance(summary.get(key),str) and 0<len(summary[key])<=2048,'compiler provenance')
  require(runtime['hostSha256']==digest(files.read('RasterCapture.exe')),'compiled capture identity');drawing_defaults(runtime.get('freshGraphicsDefaults'),2)
 elif count:raise ValueError('Captured cases lack successful dependency stages')
 rows=[]
 ids={c['id'] for c in grid}
 for name in files.raw:
  if name.startswith('cases/'):
   parts=name.split('/');require(len(parts)==3 and parts[1] in ids,'unknown/nested case path')
   case=next(c for c in grid if c['id']==parts[1]);allowed={'manifest.json','input-requested.argb'}|{o['label']+'-'+o['api']+'.argb' for o in case['observations']}
   require(parts[2] in allowed,'unexpected complete/partial raw file')
 for case,outcome in zip(grid,outcomes):
  if outcome.get('status')=='captured':
   require(ready and complete('case-'+case['id']),'claimed capture stage failed');row=validate_case(files,case,runtime['hostSha256']);require(row['manifestSha256']==outcome.get('manifestSha256'),'outcome manifest hash');rows.append(row)
  else:
   require(outcome.get('status') in ['skipped-dependency','timed-out','failed','output-limit','skipped-overall-deadline','invalid-or-incomplete'],'unknown partial outcome')
   rows.append(dict(id=case['id'],status=outcome['status'],partialFiles=sum(n.startswith('cases/'+case['id']+'/') for n in files.raw)))
 full=count==8 and ready and complete('integrity-final') and summary.get('status')=='captured-comparison-pending' and files.return_integrity
 require(summary.get('status') in ['captured-comparison-pending','partial-or-diagnostics-only'],'summary status')
 require(summary.get('status')!='captured-comparison-pending' or count==8 and complete('integrity-final'),'false complete summary')
 return dict(passed=full,status='validated-capture-analysis-pending' if full else 'partial-or-diagnostics-only',validCompleteWindowsCapture=full and not synthetic_fixture,windowsParityVerified=False,syntheticConsumerTest=synthetic_fixture,returnIntegrityVerified=files.return_integrity,cases=rows,runtime=runtime,scope='Strict ordered raw capture evidence only; no expected raster values, tolerance, image normalization or Mac parity.')
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--windows',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();require(not a.out.exists(),'fresh report required')
 try:r=validate(Files(a.windows))
 except Exception as e:r=dict(passed=False,status='invalid-evidence',validCompleteWindowsCapture=False,windowsParityVerified=False,error=str(e))
 a.out.parent.mkdir(parents=True,exist_ok=True);a.out.write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:v for k,v in r.items() if k not in ['cases','runtime']}));return 0 if r['passed'] else 2
if __name__=='__main__':raise SystemExit(main())
