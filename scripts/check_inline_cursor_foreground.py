#!/usr/bin/env python3
"""Eight bounded cursor-inclusive crops using the isolated optimized cursor-repair host.
Preparation never launches a native host or captures a screen. Root owns execution.
"""
import argparse,hashlib,json,os,plistlib,shutil,signal,struct,subprocess,time
from pathlib import Path
from check_inline_gestures_foreground import preferences
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/inline-cursor-fix-01/host'
REFERENCE=ROOT/'target/inline-gestures-foreground-release-03'
SOURCE=ROOT/'scripts/inline_cursor_driver.swift'
DRIVER=ROOT/'target/inline-cursor-fix-01/inline-cursor-driver'
NAMES=['01-level-tooltip-cursor','02-point-hover-cursor','03-point-drag-tooltip-cursor','04-point-release-cursor','05-gradient-hover-cursor','06-gradient-drag-tooltip-cursor','07-gradient-release-cursor','08-exit-arrow-no-tooltip']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,m):Path(p).write_text(json.dumps(m,indent=2)+'\n')
def remaining(end,needed):assert end-time.monotonic()>=needed,'Insufficient complete native+cleanup budget'
def validate(out,m):
 assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['result.json','events.json','foreground.json','config.json'])
 assert all(sha(p)==h for p,h in m['pins'].items()),'Frozen source or host drift'
 assert m['inputDeadlineSeconds']==20 and m['cleanupDeadlineSeconds']==25 and m['allInDeadlineSeconds']==30
 assert m['executable']==json.loads((BASE/'prepared.json').read_text())['executable']
 assert m['bundleIdentifier'].startswith('com.vixenlights.verification.inlinegestures.')
 assert plistlib.loads((Path(m['executable']).parents[1]/'Info.plist').read_bytes())['CFBundleIdentifier']==m['bundleIdentifier']
def verify(out,m,fg,host):
 assert all(fg[k] for k in ['passed','normalFinish','targetExited','restoredPrevious','restoredPointer','noInputHeld'])
 assert not fg['ownershipLost'] and fg['pendingKeyUpAcknowledgements']==[]
 assert [fg[k] for k in ['mouseDowns','mouseUps','keyDowns','keyUps']]==[3,3,1,1]
 assert len(fg['observations'])==17 and len(fg['pointerEvents'])==20
 assert host['terminated'] and host['testMode'] is False and host['current']['commits']==dict(level=1,point=1,gradient=1) and host['current']['tooltips']==[]
 crops=fg['captures'];assert [x['name'] for x in crops]==NAMES
 for row,tip in zip(crops,['25',None,'70, 75',None,None,'40',None,None]):
  p=out/(row['name']+'.png');assert row['path']==str(p) and p.is_file() and not p.is_symlink() and p.stat().st_size<2_000_000 and sha(p)==row['sha256']
  data=p.read_bytes();assert data[:16]==b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR';size=list(struct.unpack('>II',data[16:24]));assert size==row['pixelSize']
  x,y,w,h=row['cropCG'];assert all(isinstance(v,(int,float)) for v in [x,y,w,h]) and 0<w<=300 and 0<h<=140
  fx,fy,fw,fh=host['windowFrame'];top=host['screenTop'];gy=top-fy-fh
  assert fx<=x and gy<=y and x+w<=fx+fw and y+h<=gy+fh
  scale=size[0]/w;assert scale in [1,2] and size[1]==h*scale
  assert row['cursorRequested'] is True and row['screenCapturePreflight'] is True and row['captureArguments']==['-x','-C',f'-R{int(x)},{int(y)},{int(w)},{int(h)}',str(p)]
  cx,cy=row['cursorCG'];assert x<=cx<x+w and y<=cy<y+h
  assert row['windowNumber']==host['windowNumber'] and 0<=row['elapsedSeconds']<=20
  texts=[s for panel in row['passiveState']['tooltips'] for s in panel['texts']];assert texts==([] if tip is None else [tip])
 assert [r['passiveState']['commits'] for r in crops]==[dict(level=0,point=0,gradient=0),dict(level=1,point=0,gradient=0),dict(level=1,point=0,gradient=0),dict(level=1,point=1,gradient=0),dict(level=1,point=1,gradient=0),dict(level=1,point=1,gradient=0),dict(level=1,point=1,gradient=1),dict(level=1,point=1,gradient=1)]
 return dict(captures=8,retainedGestureObservations=17,manualVisualReviewRequired=True,visualAppearanceAccepted=False)
def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists()
 candidate=json.loads((BASE/'prepared.json').read_text());old=json.loads((REFERENCE/'prepared.json').read_text());accepted=json.loads((REFERENCE/'result.json').read_text())
 assert candidate['passed'] and candidate['launched'] is False and accepted['passed'] and accepted['observations']==17
 pins=dict(candidate['pinnedInputHashes']);pins.update({str(BASE/'prepared.json'):sha(BASE/'prepared.json'),str(REFERENCE/'prepared.json'):sha(REFERENCE/'prepared.json'),str(REFERENCE/'result.json'):sha(REFERENCE/'result.json')})
 for p,row in candidate['productionHashes'].items():assert sha(p)==row['sha256'];pins[p]=row['sha256']
 for p,h in candidate['originalInputHashes'].items():assert sha(p)==h; pins[p]=h
 assert sha(BASE/'host-source.swift')==sha(REFERENCE/'host-source.swift')
 for path in [Path(candidate['executable']).parents[2],DRIVER]:subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(path)],capture_output=True,check=True,timeout=3)
 for p in [Path(__file__).resolve(),SOURCE,DRIVER,ROOT/'scripts/check_inline_gestures_foreground.py',ROOT/'scripts/test_inline_cursor_controls.py',ROOT/'target/inline-cursor-fix-01/pure-controls.json',ROOT/'target/inline-cursor-fix-01/consumer-controls.json',ROOT/'target/inline-cursor-fix-01/cursor.patch',ROOT/'target/inline-cursor-fix-01/independent-source-review.json']:
  pins[str(p)]=sha(p)
 out.mkdir();(out/'used-sources').mkdir()
 for p in [SOURCE,Path(__file__).resolve(),ROOT/'scripts/test_inline_cursor_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 m=dict(executable=candidate['executable'],executableSha256=candidate['executableSha256'],driver=str(DRIVER),bundleIdentifier=candidate['bundleIdentifier'],pins=pins,inputDeadlineSeconds=20,cleanupDeadlineSeconds=25,allInDeadlineSeconds=30,launchPerformed=False,captures=NAMES,expectedCursors=['horizontal-resize','crosshair','crosshair','crosshair','horizontal-resize','horizontal-resize','horizontal-resize','arrow'],scope='Eight small cursor-inclusive crops of isolated optimized cursor-repair views: hover, captured drag, release and exit. Same17 semantic observations/three gestures. Exact accepted Rust/FFI objects reused; not full-current-application appearance or new Rust behavior verification.',permissionPolicy='Existing ScreenCapture/AX/event preflight only; fail before host launch when absent. No requests/prompts.',reuseProof='Only production cursor changes in two views;38 current VixenKit sources compiled optimized in isolated folder; passive host source byte-identical and original compiled Rust/Bridge/FFI dependencies byte-identical.')
 save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));print(json.dumps(dict(prepared=str(out/'prepared.json'),pins=len(pins),launched=False)))
def terminate_owned(child,end,group=False):
 if child is None:return
 if group:
  # Only the session/group created for this exact helper Popen; its capture child
  # inherits it. Kill remaining descendants even if the helper exits on TERM.
  try:os.killpg(child.pid,signal.SIGTERM)
  except ProcessLookupError:pass
  try:child.wait(timeout=min(.2,max(.01,end-time.monotonic())))
  except subprocess.TimeoutExpired:pass
  finally:
   try:os.killpg(child.pid,signal.SIGKILL)
   except ProcessLookupError:pass
  child.wait(timeout=.3)
 else:
  if child.poll() is not None:return
  child.terminate()
  try:child.wait(timeout=min(.3,max(.01,end-time.monotonic())))
  except subprocess.TimeoutExpired:child.kill();child.wait(timeout=.3)
def run(out,m):
 began=time.monotonic();end=began+30;host=driver=None;before=None
 result=dict(passed=False,scope=m['scope'],manualVisualReviewRequired=True,visualAppearanceAccepted=False)
 try:
  validate(out,m);before=preferences()
  inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=1)
  assert not any('/Contents/MacOS/Vixen' in s or 'inline-gesture' in s or 'inline-visual-driver' in s or 'inline-cursor-driver' in s or '/foreground-' in s for s in inventory.splitlines()),'Another verification process exists'
  remaining(end,28)
  subprocess.run([m['driver'],'--desktop',str(out/'desktop-before.json')],check=True,timeout=1)
  desktop=json.loads((out/'desktop-before.json').read_text());result['preflight']=desktop
  assert all(desktop[k] for k in ['accessibilityTrusted','eventPostingPreflight','screenCapturePreflight']),'Existing permission absent; no prompt requested'
  previous=desktop['application'];assert previous['pid']>1 and previous['executable'] and previous['bundleIdentifier'] and 'securityagent' not in previous['bundleIdentifier'].lower()
  env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env['VIXEN_SETTINGS_DIR']=str(out/'private-settings')
  with (out/'host.log').open('w') as log,(out/'driver.log').open('w') as dlog:
   remaining(end,27)
   host=subprocess.Popen([m['executable'],str(out/'events.json')],env=env,stdout=log,stderr=subprocess.STDOUT)
   save(out/'owned-processes.json',dict(hostPID=host.pid,hostExecutable=m['executable']))
   config=dict(m,pid=host.pid,output=str(out/'foreground.json'),events=str(out/'events.json'),previousApplication=previous);save(out/'config.json',config)
   remaining(end,26)
   driver=subprocess.Popen([m['driver'],str(out/'config.json')],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
   save(out/'owned-processes.json',dict(hostPID=host.pid,hostExecutable=m['executable'],driverPID=driver.pid,driverExecutable=m['driver'],driverProcessGroup=driver.pid))
   driver.wait(timeout=min(26,max(.01,end-time.monotonic()-1)))
   assert driver.returncode==0
   fg=json.loads((out/'foreground.json').read_text());result['foreground']=fg
   assert fg['passed'],fg.get('error')
   host.wait(timeout=min(1,max(.01,end-time.monotonic())))
   assert host.returncode==0
   result.update(verify(out,m,fg,json.loads((out/'events.json').read_text())),passed=True)
 except Exception as error:result['error']=repr(error)
 finally:
  errors=[]
  if driver is not None and driver.poll() is None:
   try:save(out/'abort-request.json',dict(abort=True));driver.wait(timeout=min(3,max(.01,end-time.monotonic()-1)))
   except subprocess.TimeoutExpired:pass
   except Exception as error:errors.append(repr(error))
  for child,group in [(driver,True),(host,False)]:
   try:terminate_owned(child,end,group)
   except Exception as error:errors.append(repr(error))
  result.update(cleanupErrors=errors,hostExited=host is None or host.poll() is not None,driverExited=driver is None or driver.poll() is not None,hostExitCode=host.returncode if host else None,driverExitCode=driver.returncode if driver else None)
  try:
   if (out/'foreground.json').exists():result['foreground']=json.loads((out/'foreground.json').read_text())
  except Exception as error:result.update(passed=False,foregroundReadError=repr(error))
  try:result['ownerPreferencesUnchanged']=before==preferences();result['inputsUnchanged']=all(sha(p)==h for p,h in m['pins'].items())
  except Exception as error:result.update(ownerPreferencesUnchanged=False,inputsUnchanged=False,finalEvidenceError=repr(error))
  result['elapsedSeconds']=time.monotonic()-began;result['withinDeadline']=time.monotonic()<=end
  result['passed']=result['passed'] and not errors and all(result[k] for k in ['hostExited','driverExited','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline'])
  save(out/'result.json',result)
 print(json.dumps({k:result[k] for k in ['passed','elapsedSeconds']},indent=2));return 0 if result['passed'] else 1
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();o=a.out.resolve()
 if a.prepare_only:prepare(o)
 else:raise SystemExit(run(o,json.loads((o/'prepared.json').read_text())))
