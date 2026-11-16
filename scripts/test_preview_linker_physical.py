#!/usr/bin/env python3
"""Prepare/run one private physical Preview linker workflow. Root alone launches."""
import argparse,hashlib,json,os,platform,shutil,subprocess,time
from pathlib import Path
from test_editor_family_physical import candidate_binding
from test_prop_cross_window import terminate
from test_prop_drag_edges_foreground import owner_states,hashes
import preview_linker_contract as contract
ROOT=Path(__file__).resolve().parents[1]
APP=ROOT/'target/private-app-counter-stats/Vixen.app';CANONICAL=ROOT/'target/counter-stats-release/Vixen.app';BUILD=ROOT/'target/counter-stats-integration/build-inputs.json';APPROVED='06eaac7244f8cc392acf3fb565261f55583210c695108316087f6fb5e1a90b6c'
SOURCE=ROOT/'target/preview-partials/profile-linker3';WEB=ROOT/'target/native-panels-quit-menu-fixture/profile/SystemData/ModuleStore.xml'
HELPER=ROOT/'target/preview-linker-helper-01/foreground-preview-linker';HELPER_SOURCE=ROOT/'scripts/foreground_preview_linker.swift'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
def remaining(end,n):assert end-time.monotonic()>=n,'Insufficient complete stage/cleanup budget'
def validate(out,m):
 assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
 assert all(sha(p)==h for p,h in m['pins'].items()),'Frozen input drift'
 assert hashes(SOURCE)==m['sourceHashes'] and hashes(out/'profile')==m['profileHashes']
 assert candidate_binding(APP,APPROVED,CANONICAL,BUILD)==m['candidate']
 assert [m[k] for k in ['inputDeadlineSeconds','cleanupDeadlineSeconds','allInDeadlineSeconds']]==[55,65,70]
 r=contract.typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());assert contract.canonical(r)==m['initialXML'] and [e.text for e in contract.nodes(r)]==[contract.IDS['Fixture Two']]*50
 assert m['elementIDs']==contract.IDS
 assert m['initialBackups']=={contract.backup(p):sha(p) for p in [out/'profile/SystemData/ModuleStore.xml',out/'profile/SystemData/SystemConfig.xml']}

def verify_foreground(fg,m):
 assert all(fg[k] for k in ['passed','cleanupCompleted','noInputHeld','restoredPrevious','restoredPointer','beforeSaveUnchanged','cancelStoreUnchanged'])
 assert not fg['ownershipLost'] and not fg['pendingMouseUp'] and [fg[k] for k in ['mouseDowns','mouseUps','keyEvents']]==[5,5,0]
 es=fg['events'];assert [e['kind'] for e in es]==([5,1]+[6]*30+[2])*2+[5,1,2]+[5,1,2,1,2]
 assert set(fg['windowIDs'])=={'drop','cancel'} and all(e['window']==fg['windowIDs'][e['label']] for e in es)
 assert [e['clickState'] for e in es]==[1]*72+[2,2]
 obs=fg['observations'];payloads=[e for e in obs if e['stage'].endswith('-payload')];assert [e['stage'] for e in payloads]==['valid-lower-half-payload','blank-payload']
 for index,(e,n) in enumerate(zip(payloads,['Fixture One','Fixture Two'])):
  assert e['source']==n and e['payload']==[m['elementIDs'][n]] and e['changeCountBefore']!=e['changeCountDuring']
  start,end=e['start'],e['end'];points=[start,start]+[[start[0]+i*3,start[1]] for i in range(1,7)]+[[start[0]+18+(end[0]-start[0]-18)*i/24,start[1]+(end[1]-start[1])*i/24] for i in range(1,25)]+[end]
  actual=es[index*33:(index+1)*33];assert all(x['label']=='drop' for x in actual)
  assert all(len(x['point'])==2 and all(abs(a-b)<1e-8 for a,b in zip(x['point'],point)) for x,point in zip(actual,points))
 selection=[e for e in obs if e['stage']=='double-click-target-selection'];assert len(selection)==1
 selection=selection[0];assert selection['selectedRows']==1 and selection['source']=='Fixture Two' and selection['sourceID']==m['elementIDs']['Fixture Two']
 assert all(e['label']=='cancel' for e in es[66:]) and [e['point'] for e in es[66:]]==[selection['targetPoint']]*3+[selection['sourcePoint']]*5
 assert 0<=fg['doubleClickActualGap']<fg['doubleClickInterval']<=2
 assert fg['doubleClickDownTimesSeconds']==[es[70]['elapsedSeconds'],es[72]['elapsedSeconds']]
 assert abs(fg['doubleClickDownTimesSeconds'][1]-fg['doubleClickDownTimesSeconds'][0]-fg['doubleClickActualGap'])<1e-8

 linked=[e for e in obs if 'linkedRowLabels' in e];assert [e['stage'] for e in linked]==['initial','after-valid','after-blank','reopened-saved','unsaved-double-click']
 for e,n in zip(linked,['Fixture Two','Fixture One','Fixture One','Fixture One','Fixture Two']):assert n in e['linkedRowLabels'] and ('Fixture Two' if n=='Fixture One' else 'Fixture One') not in e['linkedRowLabels']
 lower,blank=payloads[0]['end'],payloads[1]['end'];rf=linked[0]['rowFrame'];tf=linked[1]['tableFrame'];assert rf[0]<lower[0]<rf[0]+rf[2] and lower[1]==rf[1]+rf[3]*.75
 assert tf[0]<blank[0]<tf[0]+tf[2] and linked[1]['rowFrame'][1]+linked[1]['rowFrame'][3]+30<blank[1]<tf[1]+tf[3]
 return dict(actualDrags=2,actualClicks=3,physicalDoubleClick=True,blankAreaDropObserved=True,baselineFailureGuaranteed=False)

def verify(out,m,r):
 fg=json.loads((out/'foreground.json').read_text());physical=verify_foreground(fg,m)
 rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(x['ok'] for x in rows)
 infos=[x['detail'] for x in rows if x['action']=='designerLinkerInfo'];assert len(infos)==5
 for detail,name in zip(infos,['Fixture Two','Fixture One','Fixture One','Fixture Two','Fixture One']):contract.info(detail,name)
 final=contract.saved(out/'profile',m);assert final['moduleStoreSha256']==fg['savedStoreSha256']
 assert (out/'drop-complete').read_text()==(out/'cancel-complete').read_text()=='complete'
 before=m['profileHashes'];after=hashes(out/'profile');changed={k for k in before.keys()|after.keys() if before.get(k)!=after.get(k)}
 allowed={'.lock','logs/Application.Info.log','SystemData/ModuleStore.xml','SystemData/SystemConfig.xml'}|set(m['initialBackups']);assert changed<=allowed,changed
 assert {k:v for k,v in after.items() if k.startswith('SystemData/auto_backup/')}==m['initialBackups']
 assert (out/'profile/.lock').read_text()==str(r['appPID'])+'@'+platform.node()
 log=out/'profile/logs/Application.Info.log';assert log.stat().st_size<2_000_000 and log.read_bytes().startswith((SOURCE/'logs/Application.Info.log').read_bytes())
 assert not list((out/'settings').rglob('*')),'Unexpected private settings write'
 return dict(foreground=fg,**physical,saved=final,steps=len(rows),privateProfileChanges=sorted(changed),previewSaveHarnessAction=True,normalQuitClaimed=False,F82TreeDragClaimed=False)

def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists();c=candidate_binding(APP,APPROVED,CANONICAL,BUILD)
 for p in [APP,HELPER]:subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(p)],capture_output=True,check=True,timeout=3)
 out.mkdir();shutil.copytree(SOURCE,out/'profile');(out/'profile/.lock').unlink(missing_ok=True);shutil.rmtree(out/'profile/SystemData/auto_backup');(out/'settings').mkdir()
 state=contract.prepare_private(out/'profile',WEB)
 steps=[]
 def a(action,**kw):steps.append(dict(action=action,**kw))
 def wait(t=.3):a('wait',seconds=t)
 def opened():
  a('previewTarget',text='Preview Dialog Verification');a('window',id='preview-setup');wait(.8);a('designerOnScreen');a('designerClickLight',index=0);wait();a('designerCommand',text='linkFirst');wait(.5)
 a('window',id='admin');wait(1);opened();a('designerLinkerInfo');a('dump',name='drop-ready');a('snapshot',name='drop-ready');wait(18)
 a('designerLinkerInfo');a('click',label='OK');wait();a('designerCommand',text='save');wait(2);a('closeWindow');wait(1)
 opened();a('designerLinkerInfo');a('dump',name='cancel-ready');a('snapshot',name='cancel-ready');wait(14)
 a('designerLinkerInfo');a('click',label='Cancel');wait();a('designerCommand',text='linkFirst');wait(.5);a('designerLinkerInfo');a('click',label='Cancel');wait();a('closeWindow');wait(.5);a('quit')
 save(out/'steps.json',dict(steps=steps))
 inputs=[Path(__file__).resolve(),ROOT/'scripts/preview_linker_contract.py',ROOT/'scripts/test_preview_linker_controls.py',HELPER_SOURCE,HELPER,BUILD,APP/'Contents/MacOS/Vixen',APP/'Contents/Info.plist',APP.parent/'private-app.json',CANONICAL/'Contents/MacOS/Vixen',out/'steps.json',WEB,ROOT/'scripts/test_editor_family_physical.py',ROOT/'scripts/test_prop_cross_window.py',ROOT/'scripts/test_prop_drag_edges_foreground.py',ROOT/'scripts/prepare_private_test_app.py',ROOT/'scripts/vixen2_physical_contract.py',ROOT/'target/preview-linker-helper-01/pure-controls.json',ROOT/'target/preview-linker-helper-01/consumer-controls.json']
 m=dict(app=str(APP/'Contents/MacOS/Vixen'),appSha256=sha(APP/'Contents/MacOS/Vixen'),bundleIdentifier=c['bundleIdentifier'],candidate=c,helper=str(HELPER),source=str(SOURCE),sourceHashes=hashes(SOURCE),profileHashes=hashes(out/'profile'),steps=len(steps)-1,pins={str(p):sha(p) for p in inputs},inputDeadlineSeconds=55,cleanupDeadlineSeconds=65,allInDeadlineSeconds=70,harnessWaitSeconds=[18,14],launchPerformed=False,scope='Preview F93: actual lower-half linked-row drop, blank-area rejection, Save/reopen, physical selected-row/source double-click then Cancel/reopen. Entire50pixel mapping and all unrelated XML exact; F82 not exercised. Existing harness opens/saves/reads; no assignment/drop/double-click hooks.',**state)
 save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));(out/'used-sources').mkdir()
 for p in [Path(__file__).resolve(),HELPER_SOURCE,ROOT/'scripts/preview_linker_contract.py',ROOT/'scripts/test_preview_linker_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],pins=len(m['pins']),launched=False)))

def run(out,m):
 start=time.monotonic();end=start+70;app=helper=None;before=None;r=dict(passed=False,scope=m['scope'])
 try:
  validate(out,m);before=owner_states();inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=1);assert not any('/Contents/MacOS/Vixen' in s or '/foreground-' in s for s in inventory.splitlines()),'Another native verification process exists'
  remaining(end,68);subprocess.run([m['helper'],'--desktop',str(out/'desktop-before.json')],check=True,timeout=1);desktop=json.loads((out/'desktop-before.json').read_text());assert desktop['accessibility'] and desktop['eventPosting'] and not desktop['permissionRequested']
  assert desktop['previous']['pid']>1 and desktop['previous']['executable'] and desktop['previous']['bundle'] and 'securityagent' not in desktop['previous']['bundle'].lower()
  (out/'ui').mkdir();(out/'runtime').mkdir();env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_MUTE='1',VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
  with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as hlog:
   remaining(end,67);app=subprocess.Popen([m['app'],'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);r['appPID']=app.pid;save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid));remaining(end,66)
   helper=subprocess.Popen([m['helper'],str(app.pid),str(out),m['app']],stdout=hlog,stderr=subprocess.STDOUT,start_new_session=True);save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid,helperPID=helper.pid,helperExecutable=m['helper'],helperProcessGroup=helper.pid));helper.wait(timeout=min(66,max(.01,end-time.monotonic()-2)));assert helper.returncode==0
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
  try:r['ownerPreferencesUnchanged']=before is not None and before==owner_states();r['inputsUnchanged']=all(sha(p)==h for p,h in m['pins'].items()) and hashes(SOURCE)==m['sourceHashes']
  except Exception as error:r.update(ownerPreferencesUnchanged=False,inputsUnchanged=False);errors.append(repr(error))
  r.update(cleanupErrors=errors,appExited=app is None or app.poll() is not None,helperExited=helper is None or helper.poll() is not None,elapsedSeconds=time.monotonic()-start,withinDeadline=time.monotonic()<=end);r['passed']=r['passed'] and not errors and all(r[k] for k in ['appExited','helperExited','ownerPreferencesUnchanged','inputsUnchanged','withinDeadline']);save(out/'result.json',r)
 print(json.dumps({k:r[k] for k in ['passed','elapsedSeconds']}));return 0 if r['passed'] else 1
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True);g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true');a=p.parse_args();o=a.out.resolve()
 if a.prepare_only:prepare(o)
 else:raise SystemExit(run(o,json.loads((o/'prepared.json').read_text())))
