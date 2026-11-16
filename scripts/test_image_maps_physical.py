#!/usr/bin/env python3
"""Prepare one strict private Image Maps physical workflow; root alone launches."""
import argparse,hashlib,json,os,platform,shutil,subprocess,time
from pathlib import Path
from test_editor_family_physical import candidate_binding
from test_prop_cross_window import terminate
from test_prop_drag_edges_foreground import owner_states,hashes
from test_image_maps_ui import opening,maps
from vixen2_physical_contract import typed_xml,canonical
import image_maps_physical_contract as contract
ROOT=Path(__file__).resolve().parents[1]
APP=ROOT/'target/private-app-counter-stats/Vixen.app';CANONICAL=ROOT/'target/counter-stats-release/Vixen.app';BUILD=ROOT/'target/counter-stats-integration/build-inputs.json'
APPROVED='06eaac7244f8cc392acf3fb565261f55583210c695108316087f6fb5e1a90b6c'
SOURCE=ROOT/'target/image-maps-physical-helper-01/source-profile';HELPER=ROOT/'target/image-maps-physical-helper-01/foreground-image-maps-v3';HELPER_SOURCE=ROOT/'scripts/foreground_image_maps_physical.swift'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):Path(p).write_text(json.dumps(v,indent=2)+'\n')
def remaining(end,n):assert end-time.monotonic()>=n,'Insufficient full owned child and cleanup budget'
def validate(out,m):
 assert out.is_relative_to(ROOT/'target') and not any((out/n).exists() for n in ['ui','runtime','result.json','foreground.json'])
 assert all(sha(p)==v for p,v in m['pins'].items()),'Frozen input drift'
 assert hashes(SOURCE)==m['sourceHashes'] and hashes(out/'profile')==m['profileHashes']
 assert candidate_binding(APP,APPROVED,CANONICAL,BUILD)==m['candidate']
 assert [m[k] for k in ['inputDeadlineSeconds','cleanupDeadlineSeconds','allInDeadlineSeconds','harnessWaitSeconds']]==[40,48,80,50]
 config=typed_xml((out/'profile/SystemData/SystemConfig.xml').read_bytes());assert all(len(config.find(n))==0 for n in ['Controllers','Patches','Previews'])
 store=typed_xml((out/'profile/SystemData/ModuleStore.xml').read_bytes());assert [e.text for e in store.iter() if e.tag.endswith('}IsEnabled') and 'WebServer' in e.tag]==['false']
 assert contract.expected_store((out/'initial-ModuleStore.xml').read_bytes())==m['expectedStore']
 assert contract.backups(out/'profile')==m['backups']
 assert not list((out/'settings').iterdir())
def verify(out,m,r):
 fg=json.loads((out/'foreground.json').read_text());counts=contract.require_foreground(fg)
 rows=json.loads((out/'ui/result.json').read_text());assert len(rows)==m['steps'] and all(row['ok'] for row in rows)
 states=[row['detail'] for row in rows if row.get('detail','').startswith('rows [')]
 assert len(states)==2 and 'Rename Me' in states[0] and 'Renamed Map' not in states[0] and states[0].endswith('new=true edit=false remove=false clone=false')
 assert 'rows [(closed)]' in states[1] and 'Renamed Map' in states[1] and 'Rename Me' not in states[1] and 'lastClose true' in states[1]
 profile=contract.require_profile(out/'profile',m,hashes)
 assert (out/'profile/.lock').read_text()==str(r['appPID'])+'@'+platform.node()
 assert (out/'profile/logs/Application.Info.log').stat().st_size<2_000_000
 assert sha(out/'ui/ColorPalette.xml')==m['profileHashes']['ColorPalette.xml']
 return dict(foreground=fg,**counts,steps=len(rows),renderProbes=contract.require_render(rows),privateProfile=profile,normalQuitClaimed=False)
def prepare(out):
 assert out.is_relative_to(ROOT/'target') and not out.exists()
 c=candidate_binding(APP,APPROVED,CANONICAL,BUILD)
 for p in [APP,HELPER]:subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(p)],capture_output=True,check=True,timeout=3)
 out.mkdir();shutil.copytree(SOURCE,out/'profile');(out/'settings').mkdir()
 shutil.copy2(out/'profile/SystemData/ModuleStore.xml',out/'initial-ModuleStore.xml')
 def hook(name,*path):return dict(action='hook',name=name,path=list(path))
 steps=opening(out/'profile/Sequence/Image Maps Verification.tim')
 next(s for s in steps if s.get('name')=='sequencer.resize')['path']=['1100','780']
 steps += [dict(action='menu',path=['Edit','Select All Elements']),hook('sequencer.panel','Effect Editor'),dict(action='wait',seconds=.5),hook('effectEditor.renderProbe')]+maps()+[hook('imageMaps','state'),dict(action='snapshot',name='physical-ready'),dict(action='wait',seconds=50),hook('imageMaps','state'),hook('effectEditor.renderProbe'),hook('timelineState'),hook('sequencer.state'),dict(action='quit')]
 save(out/'steps.json',dict(steps=steps))
 inputs=[Path(__file__).resolve(),HELPER_SOURCE,HELPER,BUILD,APP/'Contents/MacOS/Vixen',APP/'Contents/Info.plist',APP.parent/'private-app.json',CANONICAL/'Contents/MacOS/Vixen',out/'steps.json',out/'initial-ModuleStore.xml',ROOT/'scripts/image_maps_physical_contract.py',ROOT/'scripts/test_image_maps_physical_controls.py',ROOT/'scripts/test_image_maps_ui.py',ROOT/'scripts/test_image_maps_live_ui.py',ROOT/'scripts/test_video_cache_ui.py',ROOT/'scripts/test_editor_family_physical.py',ROOT/'scripts/test_prop_cross_window.py',ROOT/'scripts/test_prop_drag_edges_foreground.py',ROOT/'scripts/prepare_private_test_app.py',ROOT/'scripts/vixen2_physical_contract.py',HELPER.parent/'pure-controls-v3.json',HELPER.parent/'consumer-controls-v3.json',HELPER.parent/'generator-provenance.json',ROOT/'crates/vixen-session/examples/image_maps_fixture.rs',ROOT/'docs/spec/marks-layers-analysis.md',ROOT/'docs/review/image-maps-native.md',ROOT/'upstream/Vixen/src/Vixen.Modules/App/LipSyncApp/LipSyncMapSelector.cs',ROOT/'upstream/Vixen/src/Vixen.Modules/App/LipSyncApp/LipSyncMapLibrary.cs',ROOT/'upstream/Vixen/src/Vixen.Modules/App/LipSyncApp/LipSyncMapData.cs',ROOT/'crates/vixen-session/src/marks/map_library.rs']
 m=dict(app=str(APP/'Contents/MacOS/Vixen'),appSha256=sha(APP/'Contents/MacOS/Vixen'),bundleIdentifier=c['bundleIdentifier'],candidate=c,helper=str(HELPER),source=str(SOURCE),sourceHashes=hashes(SOURCE),profileHashes=hashes(out/'profile'),systemConfig=canonical(typed_xml((out/'profile/SystemData/SystemConfig.xml').read_bytes())),expectedStore=contract.expected_store((out/'initial-ModuleStore.xml').read_bytes()),backups=contract.backups(out/'profile'),steps=len(steps)-1,pins={str(p):sha(p) for p in inputs},inputDeadlineSeconds=40,cleanupDeadlineSeconds=48,allInDeadlineSeconds=80,harnessWaitSeconds=50,launchPerformed=False,scope='Physical F95 slow selected-name click starts inline rename, Escape and Return, same-row pointer reselection, then F90/F92 double-click on a name selected for >0.6 seconds opens exact editor. Distinct pending editor notes cancelled; root accept preserves all data except exact renamed key/reference/folder. Live cached render readback. Private harness supplies setup/open only and read-only observations; no rename/select/edit hooks, file panels, external editor, or normal Quit claim.')
 save(out/'prepared.json',m);validate(out,json.loads((out/'prepared.json').read_text()));(out/'used-sources').mkdir()
 for p in [Path(__file__).resolve(),HELPER_SOURCE,ROOT/'scripts/image_maps_physical_contract.py',ROOT/'scripts/test_image_maps_physical_controls.py']:shutil.copy2(p,out/'used-sources'/p.name)
 print(json.dumps(dict(prepared=str(out/'prepared.json'),steps=m['steps'],pins=len(m['pins']),launched=False)))
def run(out,m):
 start=time.monotonic();end=start+80;app=helper=None;before=None;r=dict(passed=False,scope=m['scope'])
 try:
  validate(out,m);before=owner_states()
  inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=1);assert not any('/Contents/MacOS/Vixen' in s or '/foreground-' in s for s in inventory.splitlines()),'Another native verification process exists'
  remaining(end,77);desktop=json.loads(subprocess.check_output([m['helper'],'--desktop'],text=True,timeout=1));save(out/'desktop-before.json',desktop)
  assert all(desktop[k] for k in ['accessibility','eventPosting','noPhysicalInputHeld'])
  previous=desktop['application'];assert previous['pid']>1 and previous['executable'] and previous['bundleIdentifier'] and 'securityagent' not in previous['bundleIdentifier'].lower()
  assert len(desktop['pointer'])==2
  (out/'ui').mkdir();(out/'runtime').mkdir();shutil.copy2(out/'profile/ColorPalette.xml',out/'ui/ColorPalette.xml')
  env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))};env.update(VIXEN_DATA=str(out/'profile'),VIXEN_SETTINGS_DIR=str(out/'settings'),VIXEN_UI_TEST=str(out/'steps.json'),VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_MUTE='1',VIXEN_UI_TEST_TRACE='1',TMPDIR=str(out/'runtime')+'/')
  with (out/'process.log').open('w') as log,(out/'foreground.log').open('w') as hlog:
   remaining(end,75);app=subprocess.Popen([m['app'],'no_controllers','no_execution'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);r['appPID']=app.pid
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid))
   cfg=dict(pid=app.pid,out=str(out),executable=m['app'],bundleIdentifier=m['bundleIdentifier'],previousApplication=previous,pointer=desktop['pointer'],workflow='image-maps-physical',moduleStore=str(out/'profile/SystemData/ModuleStore.xml'))
   save(out/'foreground-config.json',cfg);remaining(end,73)
   helper=subprocess.Popen([m['helper'],str(out/'foreground-config.json')],stdout=hlog,stderr=subprocess.STDOUT,start_new_session=True)
   save(out/'owned-processes.json',dict(appPID=app.pid,appExecutable=m['app'],appProcessGroup=app.pid,helperPID=helper.pid,helperExecutable=m['helper'],helperProcessGroup=helper.pid))
   helper.wait(timeout=min(49,max(.01,end-time.monotonic()-4)));assert helper.returncode==0
   app.wait(timeout=max(.01,end-time.monotonic()-3));assert app.returncode==0
  r.update(verify(out,m,r),passed=True)
 except Exception as error:r['error']=repr(error)
 finally:
  errors=[]
  if helper is not None and helper.poll() is None:
   try:save(out/'abort-request.json',dict(abort=True));helper.wait(timeout=min(4,max(.01,end-time.monotonic()-1)))
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
