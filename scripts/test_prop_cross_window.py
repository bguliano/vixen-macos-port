#!/usr/bin/env python3
"""Prepare/run one private two-window Prop Order rejection; no production changes."""
import argparse,hashlib,json,os,platform,shutil,signal,subprocess,time,zipfile
from pathlib import Path
import xml.etree.ElementTree as ET
from test_editor_family_physical import candidate_binding
from test_prop_drag_edges_foreground import canonical,package,owner_states,hashes
from vixen2_physical_contract import typed_xml
ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'target/native-panels-quit-menu-fixture/profile'
FIXTURE=ROOT/'target/prop-drag-foreground/fixture.prp'
APP=ROOT/'target/private-app-counter-stats/Vixen.app'
CANONICAL=ROOT/'target/counter-stats-release/Vixen.app'
BUILD=ROOT/'target/counter-stats-integration/build-inputs.json'
APPROVED='06eaac7244f8cc392acf3fb565261f55583210c695108316087f6fb5e1a90b6c'
HELPER=ROOT/'target/foreground-prop-cross-window-v3'
HELPER_SOURCE=ROOT/'scripts/foreground_prop_cross_window.swift'
NAMES=['Pixel 1','Pixel 2','Pixel 3']
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
def remaining(end,n):assert end-time.monotonic()>=n,'Insufficient full helper+cleanup budget'
def validate(out,m):
 assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
 assert all(sha(p)==h for p,h in m['pins'].items()),'Frozen input drift'
 assert hashes(SOURCE)==m['sourceHashes'] and hashes(out/'profile')==m['profileHashes']
 assert candidate_binding(APP,APPROVED,CANONICAL,BUILD)==m['candidate']
 assert [m[k] for k in ['inputDeadlineSeconds','cleanupDeadlineSeconds','allInDeadlineSeconds','harnessWaitSeconds']]==[28,40,45,18]
 assert (out/'source.prp').read_bytes()==(out/'destination.prp').read_bytes()==FIXTURE.read_bytes()
 assert canonical(package(out/'source.prp'))==m['original']
 config=ET.parse(out/'profile/SystemData/SystemConfig.xml').getroot();assert all(len(config.find(n))==0 for n in ['Controllers','Patches'])
 store=typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());assert [e.text for e in store.iter() if e.tag.endswith('}IsEnabled') and 'WebServer' in e.tag]==['false']
def verify_packages(out,m):
 results={}
 with zipfile.ZipFile(FIXTURE) as z:background=z.read('background.jpg')
 for label in ['source','destination']:
  p=out/'saved'/(label+'.prp')
  with zipfile.ZipFile(p) as z:
   assert set(z.namelist())=={'prop.json','background.jpg'} and len(z.namelist())==2
   assert z.read('background.jpg')==background
  assert canonical(package(p))==m['original'],label+' graph,order,metadata or values changed'
  results[label]=dict(sha256=sha(p),backgroundSha256=hashlib.sha256(background).hexdigest(),exactCanonicalManifest=True)
 assert (out/'source.prp').read_bytes()==(out/'destination.prp').read_bytes()==FIXTURE.read_bytes()
 return results
def verify(out,m,report):
 fg=json.loads((out/'foreground.json').read_text());assert all(fg[k] for k in ['passed','cleanupCompleted','noInputHeld','restoredPrevious','restoredPointer'])
 assert not fg['ownershipLost'] and not fg['pendingMouseUp'] and [fg[k] for k in ['mouseDowns','mouseUps','keyEvents']]==[1,1,0]
 ids=fg['windowIDs'];assert set(ids)=={'source','destination'} and ids['source']!=ids['destination']
 events=fg['events'];assert len(events)==33 and [e['kind'] for e in events]==[5,1]+[6]*30+[2]
 assert all(e['window']==ids[e['label']] for e in events) and events[0]['label']==events[1]['label']=='source' and events[-1]['label']=='destination'
 payload=fg['dragPayload'];assert payload['changeCountBefore']!=payload['changeCountDuring'] and payload['elementIDs']==[m['sourceElementID']]
 assert (out/'drag-finished.json').read_text()=='drag-finished'
 rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(r['ok'] for r in rows)
 for c in m['checkpoints']:
  host,state,order=(rows[c[k]]['detail'] for k in ['host','state','order'])
  assert host=='host='+c['title'] and ' dirty=false ' in state and ' undo=- ' in state,state
  assert order==m['expectedOrder'],order
 saved=verify_packages(out,m)
 after=hashes(out/'profile');before=m['profileHashes'];changes={k for k in before.keys()|after.keys() if before.get(k)!=after.get(k)}
 assert changes<= {'.lock','logs/Application.Info.log'},changes
 assert (out/'profile/.lock').read_text()==str(report['appPID'])+'@'+platform.node()
 log=out/'profile/logs/Application.Info.log';assert log.stat().st_size<2_000_000
 original=SOURCE/'logs/Application.Info.log';assert log.read_bytes().startswith(original.read_bytes())
 assert not list((out/'settings').rglob('*')),'Unexpected private settings write'
 return dict(foreground=fg,savedPackages=saved,steps=len(rows),physicalDrags=1,exactCanonicalGraphs=True,sourceUnchanged=True,normalQuitClaimed=False,privateProfileChanges=sorted(changes))
def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists()
 c=candidate_binding(APP,APPROVED,CANONICAL,BUILD);original=canonical(package(FIXTURE));assert set(zipfile.ZipFile(FIXTURE).namelist())=={'prop.json','background.jpg'}
 out.mkdir();shutil.copytree(SOURCE,out/'profile');(out/'profile/.lock').unlink(missing_ok=True);(out/'settings').mkdir();(out/'saved').mkdir()
 for l in ['source','destination']:shutil.copy2(FIXTURE,out/(l+'.prp'))
 steps=[];checkpoints=[]
 def h(*args):return dict(action='hook',name='props',path=list(args))
 def wait(t=.3):return dict(action='wait',seconds=t)
 def focus(t):return dict(action='focus',window=t)
 def checkpoint(label,phase):
  title=label.title()+' Prop Order';row=dict(label=label,phase=phase,title=title)
  for name in ['host','state','order']:row[name]=len(steps);steps.append(h(name))
  checkpoints.append(row)
 steps += [wait(1.5)]
 for label in ['source','destination']:
  title=label.title()+' Prop Order'
  steps += [dict(action='menu',path=['Tools','Custom Prop Editor']),wait(.5),focus('Custom Prop Editor'),wait(.2),h('open',str(out/(label+'.prp'))),wait(.2),h('tab','order'),wait(.2),h('label-window',title),focus(title),wait(.2)]
  checkpoint(label,'before');steps += [dict(action='dump',name=label+'-ready'),dict(action='snapshot',name=label+'-ready')]
 steps += [wait(18)]
 for label in ['destination','source']:
  title=label.title()+' Prop Order';steps += [focus(title),wait(.2)];checkpoint(label,'after');steps += [h('save',str(out/'saved'/(label+'.prp'))),wait(.2)]
 steps += [dict(action='quit')];save(out/'steps.json',dict(steps=steps))
 inputs=[Path(__file__).resolve(),HELPER_SOURCE,HELPER,FIXTURE,BUILD,APP/'Contents/MacOS/Vixen',APP/'Contents/Info.plist',APP.parent/'private-app.json',CANONICAL/'Contents/MacOS/Vixen',out/'steps.json',out/'source.prp',out/'destination.prp',ROOT/'scripts/test_prop_drag_edges_foreground.py',ROOT/'scripts/test_editor_family_physical.py',ROOT/'scripts/prepare_private_test_app.py',ROOT/'scripts/vixen2_physical_contract.py',ROOT/'scripts/test_prop_cross_window_controls.py',ROOT/'target/prop-cross-window-helper-03/pure-controls.json',ROOT/'target/prop-cross-window-helper-02/consumer-controls.json']
 m=dict(app=str(APP/'Contents/MacOS/Vixen'),appSha256=sha(APP/'Contents/MacOS/Vixen'),bundleIdentifier=c['bundleIdentifier'],candidate=c,helper=str(HELPER),source=str(SOURCE),sourceHashes=hashes(SOURCE),profileHashes=hashes(out/'profile'),original=original,sourceElementID=next(e['id'] for e in original['elements'] if e['name']=='Pixel 1'),expectedOrder='1:Pixel 1|2:Pixel 2|3:Pixel 3',checkpoints=checkpoints,steps=len(steps)-1,pins={str(p):sha(p) for p in inputs},inputDeadlineSeconds=28,cleanupDeadlineSeconds=40,allInDeadlineSeconds=45,harnessWaitSeconds=18,launchPerformed=False,scope='One actual foreign-window Prop Order rejection between private identical-UUID documents; exact graph/background/order and clean-state evidence. No drop/selection hooks, keys, real Windows-authored prop or normal Quit claim.')
 save(out/'prepared.json',m);validate(out,m);(out/'used-sources').mkdir()
 for p in [Path(__file__).resolve(),HELPER_SOURCE,ROOT/'scripts/test_prop_cross_window_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],pins=len(m['pins']),launched=False)))
def terminate(child,end):
 if child is None:return
 try:os.killpg(child.pid,signal.SIGTERM)
 except ProcessLookupError:return
 try:child.wait(timeout=min(.3,max(.01,end-time.monotonic())))
 except subprocess.TimeoutExpired:pass
 finally:
  try:os.killpg(child.pid,signal.SIGKILL)
  except ProcessLookupError:pass
 child.wait(timeout=.3)
def run(out,m):
 start=time.monotonic();end=start+45;app=helper=None;before=None;r=dict(passed=False,scope=m['scope'])
 try:
  validate(out,m);before=owner_states()
  inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=1);assert not any('/Contents/MacOS/Vixen' in s or '/foreground-' in s for s in inventory.splitlines()),'Another native verification process exists'
  remaining(end,43);subprocess.run([m['helper'],'--desktop',str(out/'desktop-before.json')],check=True,timeout=1)
  desktop=json.loads((out/'desktop-before.json').read_text());assert desktop['accessibility'] and desktop['eventPosting'] and not desktop['permissionRequested']
  assert desktop['previous']['pid']>1 and desktop['previous']['executable'] and desktop['previous']['bundle'] and 'securityagent' not in desktop['previous']['bundle'].lower()
  (out/'ui').mkdir();(out/'runtime').mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_MUTE='1',VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
  with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as hlog:
   remaining(end,42);app=subprocess.Popen([m['app'],'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);r['appPID']=app.pid
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid))
   remaining(end,41);helper=subprocess.Popen([m['helper'],str(app.pid),str(out),m['app']],stdout=hlog,stderr=subprocess.STDOUT,start_new_session=True)
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid,helperPID=helper.pid,helperExecutable=m['helper'],helperProcessGroup=helper.pid))
   helper.wait(timeout=min(41,max(.01,end-time.monotonic()-2)));assert helper.returncode==0
   app.wait(timeout=min(1,max(.01,end-time.monotonic())));assert app.returncode==0
  r.update(verify(out,m,r),passed=True)
 except Exception as error:r['error']=repr(error)
 finally:
  errors=[]
  if helper is not None and helper.poll() is None:
   try:save(out/'abort-request.json',dict(abort=True));helper.wait(timeout=min(3,max(.01,end-time.monotonic()-1)))
   except subprocess.TimeoutExpired:pass
   except Exception as error:errors.append(repr(error))
  for child in [helper,app]:
   try:terminate(child,end)
   except Exception as error:errors.append(repr(error))
  try:
   if (out/'foreground.json').exists():r['foreground']=json.loads((out/'foreground.json').read_text())
  except Exception as error:errors.append(repr(error))
  try:r['ownerPreferencesUnchanged']=before==owner_states();r['inputsUnchanged']=all(sha(p)==h for p,h in m['pins'].items()) and hashes(SOURCE)==m['sourceHashes']
  except Exception as error:r.update(ownerPreferencesUnchanged=False,inputsUnchanged=False);errors.append(repr(error))
  r.update(cleanupErrors=errors,appExited=app is None or app.poll() is not None,helperExited=helper is None or helper.poll() is not None,elapsedSeconds=time.monotonic()-start,withinDeadline=time.monotonic()<=end)
  r['passed']=r['passed'] and not errors and all(r[k] for k in ['appExited','helperExited','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline']);save(out/'result.json',r)
 print(json.dumps({k:r[k] for k in ['passed','elapsedSeconds']}));return 0 if r['passed'] else 1
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();o=a.out.resolve()
 if a.prepare_only:prepare(o)
 else:raise SystemExit(run(o,json.loads((o/'prepared.json').read_text())))
