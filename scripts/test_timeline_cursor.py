#!/usr/bin/env python3
"""Three private timeline hover cursor captures; no button/key events. Root alone launches."""
import argparse,hashlib,json,os,platform,shutil,struct,subprocess,time
from pathlib import Path
import xml.etree.ElementTree as ET
from test_editor_family_physical import candidate_binding
from test_prop_cross_window import terminate
from test_prop_drag_edges_foreground import owner_states,hashes
from vixen2_physical_contract import typed_xml
ROOT=Path(__file__).resolve().parents[1]
APP=ROOT/'target/private-app-counter-stats/Vixen.app';CANONICAL=ROOT/'target/counter-stats-release/Vixen.app';BUILD=ROOT/'target/counter-stats-integration/build-inputs.json'
APPROVED='06eaac7244f8cc392acf3fb565261f55583210c695108316087f6fb5e1a90b6c'
SOURCE=ROOT/'target/editor-family-fixture-v1/color';LAYOUT=ROOT/'target/editor-family-color-layout-01'
HELPER=ROOT/'target/timeline-cursor-helper-02/foreground-timeline-cursor';HELPER_SOURCE=ROOT/'scripts/foreground_timeline_cursor.swift';CAL=ROOT/'target/timeline-cursor-helper-01/calibration.json'
NAMES=['01-front-resize','02-back-resize','03-body-arrow'];POINTS=[[311,188],[505,188],[408,188]]
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
def remaining(end,n):assert end-time.monotonic()>=n,'Insufficient complete child and cleanup budget'
def timeline_contract(state):
 assert state['drag']=='Normal' and state['modified'] is False and state['editorState'] is False and state['transport']=='stopped'
 assert state['undo']==state['redo']==state['guides']==[] and state['selected']==[1] and state['positionMs']==0 and state['playbackLineX']==-1
 assert state['elements']==[dict(duration=20000000,id=1,rendered=True,rows=[0],selected=True,start=0,type='Set Level')]
 assert state['settings']==dict(drawMode=False,snap=True)
 assert state['visibleRows']==[dict(active=False,height=32,id=0,name='Private family drag element',open=False,selected=False,top=0)]
 assert state['view']==dict(canvasHeight=32,canvasWidth=300,cursor=0,marksBarHeight=0,rulerHeight=50,scrollX=0,timePerPixel=100000,total=30000000,verticalOffset=0,visibleStart=0,waveformHeight=0)
 assert state['gridSize']==[514,567] and state['renderStatus']==dict(failed=0,waiting=0)
 return state

def validate(out,m):
 assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
 assert all(sha(p)==v for p,v in m['pins'].items()),'Frozen input drift'
 assert hashes(SOURCE)==m['sourceHashes'] and hashes(out/'profile')==m['profileHashes']
 assert candidate_binding(APP,APPROVED,CANONICAL,BUILD)==m['candidate']
 assert [m[k] for k in ['inputDeadlineSeconds','cleanupDeadlineSeconds','allInDeadlineSeconds','harnessWaitSeconds']]==[20,28,30,18]
 config=ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot();assert all(len(config.find(n))==0 for n in ['Controllers','Patches','Previews'])
 store=typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());assert [e.text for e in store.iter() if e.tag.endswith('}IsEnabled') and 'WebServer' in e.tag]==['false']
 assert m['captureNames']==NAMES and m['points']==POINTS

def verify_profile(out,m,app_pid):
 after=hashes(out/'profile');before=m['profileHashes'];changes={k for k in before.keys()|after.keys() if before.get(k)!=after.get(k)}
 assert changes<={'.lock','logs/Application.Info.log'},changes
 assert (out/'profile/.lock').read_text()==str(app_pid)+'@'+platform.node()
 log=out/'profile/logs/Application.Info.log';assert log.stat().st_size<2_000_000
 old=SOURCE/'logs/Application.Info.log';assert log.read_bytes().startswith(old.read_bytes() if old.exists() else b'')
 assert not list((out/'settings').rglob('*')),'Unexpected private settings write'
 assert sha(out/'ui/ColorPalette.xml')==m['profileHashes']['ColorPalette.xml']
 return sorted(changes)

def verify_hover(out,m,fg):
 assert all(fg[k] for k in ['passed','cleanupCompleted','inputCompleted','noInputHeld','restoredPrevious','restoredPointer'])
 assert fg['ownershipLost'] is False and [fg[k] for k in ['mouseDowns','mouseUps','keyEvents']]==[0,0,0]
 ids=fg['windowIDs'];assert set(ids)=={'editor'} and type(ids['editor']) is int
 f=fg['frames']['editor'];assert len(f)==4 and f[2:]==[1100,780]
 assert len(fg['events'])==3 and [x['kind'] for x in fg['events']]==[5,5,5]
 for e,p in zip(fg['events'],POINTS):assert e['window']==ids['editor'] and e['point']==[f[0]+p[0],f[1]+p[1]] and 0<=e['elapsedSeconds']<=fg['inputCutoffSeconds']<=20
 assert [x['name'] for x in fg['captures']]==NAMES
 for c,p in zip(fg['captures'],POINTS):
  path=out/(c['name']+'.png');assert c['path']==str(path) and path.is_file() and not path.is_symlink() and path.stat().st_size<2_000_000 and sha(path)==c['sha256']
  data=path.read_bytes();assert data[:16]==b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR';size=list(struct.unpack('>II',data[16:24]));assert size==c['pixelSize'] and size in [[80,56],[160,112]]
  x,y,w,h=c['cropCG'];assert [w,h]==[80,56] and x>=f[0] and y>=f[1] and x+w<=f[0]+f[2] and y+h<=f[1]+f[3]
  assert c['cursorCG']==[f[0]+p[0],f[1]+p[1]] and c['cropCG']==[f[0]+p[0]-36,f[1]+p[1]-24,80,56]
  assert c['cursorRequested'] is True and c['window']==ids['editor'] and c['arguments']==['-x','-C',f'-R{int(x)},{int(y)},80,56',str(path)] and 0<=c['elapsedSeconds']<=fg['inputCutoffSeconds']
 return dict(captures=3,pointerMoves=3,buttonEvents=0,keyEvents=0,manualVisualReviewRequired=True,visualAppearanceAccepted=False)

def verify(out,m,r):
 fg=json.loads((out/'foreground.json').read_text());v=verify_hover(out,m,fg)
 rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(row['ok'] for row in rows)
 states=[json.loads(row['detail']) for row in rows if '"activeRow"' in row.get('detail','')];assert len(states)==2
 before=timeline_contract(states[0]);after=timeline_contract(states[1]);assert before==after,'Any timeline value changed during hovering'
 assert json.loads((out/'before-timeline-state.json').read_text())==before and json.loads((out/'ui/timeline-state.json').read_text())==after
 assert sha(out/'before-timeline-state.json')==fg['beforeTimelineSha256']
 transports=[row['detail'] for row in rows if row.get('detail','').startswith('speed=')];assert len(transports)==2 and all('running=false' in t for t in transports)
 return dict(foreground=fg,**v,steps=len(rows),timelineBefore=before,timelineAfter=after,exactTimelineUnchanged=True,privateProfileChanges=verify_profile(out,m,r['appPID']),normalQuitClaimed=False)

def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists()
 c=candidate_binding(APP,APPROVED,CANONICAL,BUILD);layout=json.loads((LAYOUT/'prepared.json').read_text());assert hashes(SOURCE)==layout['sourceHashes']
 assert json.loads((LAYOUT/'independent-layout-acceptance.json').read_text())['passed']
 timeline_contract(json.loads((LAYOUT/'ui/timeline-state.json').read_text()))
 cal=json.loads(CAL.read_text());assert cal['points']==POINTS and cal['captureNames']==NAMES and cal['referenceSha256']==sha(LAYOUT/'ui/ready.png')
 for p in [APP,HELPER]:subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(p)],capture_output=True,check=True,timeout=3)
 out.mkdir();shutil.copytree(SOURCE,out/'profile');(out/'profile/.lock').unlink(missing_ok=True);(out/'settings').mkdir();shutil.copy2(CAL,out/'calibration.json')
 steps=json.loads((LAYOUT/'layout-steps.json').read_text())['steps'];sequence=out/'profile/Sequence/Editor Color.tim'
 for row in steps:
  if row.get('name')=='admin.openSequence':row['path']=[str(sequence)]
 index=next(i for i,row in enumerate(steps) if row.get('action')=='snapshot' and row.get('name')=='ready')
 steps[index:index]=[dict(action='hook',name='timelineState',path=[])]
 index+=1;steps.insert(index+1,dict(action='wait',seconds=18));steps.insert(-1,dict(action='hook',name='sequencer.transport',path=['state']))
 save(out/'steps.json',dict(steps=steps))
 inputs=[Path(__file__).resolve(),HELPER_SOURCE,HELPER,CAL,BUILD,APP/'Contents/MacOS/Vixen',APP/'Contents/Info.plist',APP.parent/'private-app.json',CANONICAL/'Contents/MacOS/Vixen',out/'steps.json',out/'calibration.json',LAYOUT/'prepared.json',LAYOUT/'independent-layout-acceptance.json',LAYOUT/'ui/ready.png',LAYOUT/'ui/timeline-state.json',ROOT/'scripts/test_editor_family_physical.py',ROOT/'scripts/test_prop_cross_window.py',ROOT/'scripts/test_prop_drag_edges_foreground.py',ROOT/'scripts/prepare_private_test_app.py',ROOT/'scripts/vixen2_physical_contract.py',ROOT/'scripts/test_timeline_cursor_controls.py',ROOT/'target/timeline-cursor-helper-02/pure-controls.json',ROOT/'target/timeline-cursor-helper-01/raster-controls.json',ROOT/'target/timeline-cursor-helper-02/consumer-controls.json']
 m=dict(app=str(APP/'Contents/MacOS/Vixen'),appSha256=sha(APP/'Contents/MacOS/Vixen'),bundleIdentifier=c['bundleIdentifier'],candidate=c,helper=str(HELPER),source=str(SOURCE),sourceHashes=hashes(SOURCE),profileHashes=hashes(out/'profile'),steps=len(steps)-1,points=POINTS,captureNames=NAMES,pins={str(p):sha(p) for p in inputs},inputDeadlineSeconds=20,cleanupDeadlineSeconds=28,allInDeadlineSeconds=30,harnessWaitSeconds=18,launchPerformed=False,scope='Three actual Timeline F65 hover cursor crops: front/back resize and body arrow. Zero button/key events; exact unchanged timeline and source/profile bytes; no mutation hooks or normal-Quit claim.',calibrationSourceExecutableSha256=layout['sourceExecutableSha256'],calibrationScope='Retained945941 one-effect geometry/complete rectangle. Current cursor candidate must independently match exact raster before input; no old visual result reattributed.')
 save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));(out/'used-sources').mkdir()
 for p in [Path(__file__).resolve(),HELPER_SOURCE,ROOT/'scripts/test_timeline_cursor_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],pins=len(m['pins']),launched=False)))

def run(out,m):
 start=time.monotonic();end=start+30;app=helper=None;before=None;r=dict(passed=False,scope=m['scope'],manualVisualReviewRequired=True,visualAppearanceAccepted=False)
 try:
  validate(out,m);before=owner_states()
  ps=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=1);assert not any('/Contents/MacOS/Vixen' in s or '/foreground-' in s or 'inline-cursor-driver' in s for s in ps.splitlines()),'Another native verification process exists'
  remaining(end,28.5);subprocess.run([m['helper'],'--desktop',str(out/'desktop-before.json')],check=True,timeout=1)
  desktop=json.loads((out/'desktop-before.json').read_text());assert all(desktop[k] for k in ['accessibility','eventPosting','screenCapture']) and not desktop['permissionRequested']
  assert desktop['previous']['pid']>1 and desktop['previous']['executable'] and desktop['previous']['bundle'] and 'securityagent' not in desktop['previous']['bundle'].lower()
  (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
  env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_MUTE='1',VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
  with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as hlog:
   remaining(end,28);app=subprocess.Popen([m['app'],'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);r['appPID']=app.pid
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid));remaining(end,27.5)
   helper=subprocess.Popen([m['helper'],str(app.pid),str(out),m['app']],stdout=hlog,stderr=subprocess.STDOUT,start_new_session=True)
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid,helperPID=helper.pid,helperExecutable=m['helper'],helperProcessGroup=helper.pid))
   helper.wait(timeout=min(27,max(.01,end-time.monotonic()-1)));assert helper.returncode==0
   app.wait(timeout=min(.5,max(.01,end-time.monotonic())));assert app.returncode==0
  r.update(verify(out,m,r),passed=True)
 except Exception as error:r['error']=repr(error)
 finally:
  errors=[]
  if helper is not None and helper.poll() is None:
   try:save(out/'abort-request.json',dict(abort=True));helper.wait(timeout=min(.75,max(.01,end-time.monotonic()-.5)))
   except subprocess.TimeoutExpired:pass
   except Exception as error:errors.append(repr(error))
  for child in [helper,app]:
   try:terminate(child,end)
   except Exception as error:errors.append(repr(error))
  try:
   if (out/'foreground.json').exists():r['foreground']=json.loads((out/'foreground.json').read_text())
  except Exception as error:errors.append(repr(error))
  try:r['ownerPreferencesUnchanged']=before is not None and before==owner_states();r['inputsUnchanged']=all(sha(p)==h for p,h in m['pins'].items()) and hashes(SOURCE)==m['sourceHashes']
  except Exception as error:r.update(ownerPreferencesUnchanged=False,inputsUnchanged=False);errors.append(repr(error))
  r.update(cleanupErrors=errors,appExited=app is None or app.poll() is not None,helperExited=helper is None or helper.poll() is not None,elapsedSeconds=time.monotonic()-start,withinDeadline=time.monotonic()<=end)
  r['passed']=r['passed'] and not errors and all(r[k] for k in ['appExited','helperExited','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline']);save(out/'result.json',r)
 print(json.dumps({k:r[k] for k in ['passed','elapsedSeconds']}));return 0 if r['passed'] else 1
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();o=a.out.resolve()
 if a.prepare_only:prepare(o)
 else:raise SystemExit(run(o,json.loads((o/'prepared.json').read_text())))
