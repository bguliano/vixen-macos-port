#!/usr/bin/env python3
"""Four bounded cursor-inclusive crops using the immutable accepted native control host.
Preparation never launches a native host or captures a screen. Root owns execution.
"""
import argparse,hashlib,json,os,plistlib,shutil,signal,struct,subprocess,time
from pathlib import Path
from check_inline_gestures_foreground import preferences
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/inline-gestures-foreground-release-03'
SOURCE=ROOT/'scripts/inline_visual_driver.swift'
DRIVER=ROOT/'target/inline-visual-preparation-04/inline-visual-driver'
NAMES=['01-level-tooltip-cursor','02-point-tooltip-cursor','03-gradient-tooltip-cursor','04-exit-arrow-no-tooltip']
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
 for row,tip in zip(crops,['25','70, 75','40',None]):
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
 assert [r['passiveState']['commits'] for r in crops]==[dict(level=0,point=0,gradient=0),dict(level=1,point=0,gradient=0),dict(level=1,point=1,gradient=0),dict(level=1,point=1,gradient=1)]
 return dict(captures=4,retainedGestureObservations=17,manualVisualReviewRequired=True,visualAppearanceAccepted=False)
def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists()
 old=json.loads((BASE/'prepared.json').read_text());accepted=json.loads((BASE/'result.json').read_text());assert accepted['passed'] and accepted['normalFinish'] and accepted['observations']==17
 pins=dict(old['pinnedInputHashes']);pins.update({str(BASE/'prepared.json'):sha(BASE/'prepared.json'),str(BASE/'result.json'):sha(BASE/'result.json')})
 for p,row in old['productionHashes'].items():assert sha(p)==row['sha256'];pins[p]=row['sha256']
 original=json.loads((BASE/'source-build-inputs.json').read_text())['sources']
 relevant=['crates/vixen-ffi/src/kit.rs','crates/vixen-session/src/effect_editor/curve_strip.rs','crates/vixen-session/src/effect_editor/gradient_strip.rs','crates/vixen-core/src/curve.rs','crates/vixen-core/src/gradient.rs']
 for rel in relevant:assert sha(ROOT/rel)==original[rel];pins[str(ROOT/rel)]=original[rel]
 assert sha(ROOT/'scripts/inline_gesture_host.swift')==sha(BASE/'host-source.swift')
 assert sha(old['executable'])==old['executableSha256']
 for path in [Path(old['executable']).parents[2],DRIVER]:subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(path)],capture_output=True,check=True,timeout=3)
 for p in [Path(__file__).resolve(),SOURCE,DRIVER,Path(old['executable']),Path(old['executable']).parents[1]/'Info.plist',ROOT/'scripts/check_inline_gestures_foreground.py',ROOT/'scripts/test_inline_visual_controls.py',ROOT/'target/inline-visual-preparation-04/pure-controls.json',ROOT/'target/inline-visual-preparation-04/consumer-controls.json']:
  pins[str(p)]=sha(p)
 out.mkdir();(out/'used-sources').mkdir()
 for p in [SOURCE,Path(__file__).resolve(),ROOT/'scripts/test_inline_visual_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 m=dict(executable=old['executable'],executableSha256=old['executableSha256'],driver=str(DRIVER),bundleIdentifier=old['bundleIdentifier'],pins=pins,inputDeadlineSeconds=20,cleanupDeadlineSeconds=25,allInDeadlineSeconds=30,launchPerformed=False,captures=NAMES,oldHostReleaseSha256=old['sourceReleaseSha256'],scope='Four cursor-inclusive small screen crops of unchanged optimized Curve/Gradient native controls in the accepted isolated490x330 host; retains prior17 gesture observations. No full current application, Core, show or model-transfer claim. Visual readability/cursor shape requires actual image review.',permissionPolicy='CGPreflightScreenCaptureAccess/AXIsProcessTrusted/CGPreflightPostEventAccess only; absent permission stops before host launch; no requests or prompts.',reuseProof='Same immutable signed host and full compiled-input/ABI pins; current3native views+5Rust value/gesture/FFI sources identical to accepted source build.')
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
  assert not any('/Contents/MacOS/Vixen' in s or 'inline-gesture' in s or 'inline-visual-driver' in s or '/foreground-' in s for s in inventory.splitlines()),'Another verification process exists'
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
