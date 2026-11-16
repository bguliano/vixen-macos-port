#!/usr/bin/env python3
"""Actual runner fault tests with mocked native processes; no app/input launched."""
import contextlib,copy,io,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_library_cross_window_foreground as r
ROOT=Path(__file__).resolve().parents[1]
FG=dict(passed=True,cleanupCompleted=True,noInputHeld=True,restoredPrevious=True)
class Clock:
 def __init__(self):self.now=0.
 def monotonic(self):return self.now
 def sleep(self,n):self.now+=n
class Tests(unittest.TestCase):
 def execute(self,mode):
  with tempfile.TemporaryDirectory(prefix='cross-library-control-',dir=ROOT/'target') as tmp:
   out=Path(tmp);(out/'profile').mkdir();(out/'profile/ColorPalette.xml').write_text('private');clock=Clock();children=[];launched=[];signals=[]
   m=dict(app='/private/app',helper='/private/helper',appSha256='pin',source='/private/source',sourceHashes={},frozenHashes={},sourceExecutableSha256='candidate',scope='fake')
   class Process:
    def __init__(self,cmd,**kwargs):
     self.pid=999970+len(children);self.returncode=None;self.done=False;children.append(self);launched.append(cmd)
     if len(children)==1:
      self.done=mode not in ['timeout','driver-spawn-failure','checkpoint-pending','checkpoint-timeout'];self.returncode=0 if self.done else None
      assert kwargs['env']['VIXEN_SETTINGS_DIR']==str(out/'ui') and kwargs['env']['VIXEN_MUTE']=='1'
      if mode=='app-spawn-slow':clock.now+=23
     else:
      if mode=='driver-spawn-failure':raise OSError('spawn failed')
      self.done=True;self.returncode=0;fg=dict(FG)
      if mode=='held':fg['noInputHeld']=False
      if mode=='unrestored':fg['restoredPrevious']=False
      if mode=='missing-cleanup':fg.pop('cleanupCompleted')
      if mode!='missing-foreground':(out/'foreground.json').write_text(json.dumps(fg))
      if mode in ['checkpoint-pending','checkpoint-timeout']:(out/'checkpoint-ready.request').write_text('ready')
    def poll(self):
     if self.pid==999970 and mode=='checkpoint-pending' and (out/'checkpoint-ready.json').exists():self.done=True;self.returncode=0
     return self.returncode if self.done else None
    def wait(self,timeout):self.done=True;self.returncode=0;return 0
   def validation(*args):
    if mode=='preflight-slow':clock.now+=8
    if mode=='validation-failure':raise AssertionError('input mismatch')
   def verify(*args):
    if mode=='semantic-failure':raise AssertionError('wrong saved library')
   def signal(pid,sig):
    signals.append((pid,sig))
    for child in children:
     if child.pid==pid:child.done=True;child.returncode=-15
   attempts=0
   def checkpoint(*args):
    nonlocal attempts
    attempts+=1
    if mode=='checkpoint-timeout' or (mode=='checkpoint-pending' and attempts<3):raise AssertionError('pending original native callback')
    return dict(passed=True,whileAlive=True)
   calls=0
   def prefs():
    nonlocal calls
    calls+=1
    if mode=='preferences-error' and calls>1:raise OSError('read failed')
    return {'path':'changed' if mode=='preferences-changed' and calls>1 else 'before'}
   with patch.object(r,'validate',validation),patch.object(r,'verify',verify),patch.object(r,'preferences',prefs),patch.object(r,'library_state',lambda p:{}),patch.object(r,'accept_checkpoint',checkpoint),patch.object(r,'sha',lambda p:'pin'),patch.object(r,'tree',lambda p:{}),patch.object(r.time,'monotonic',clock.monotonic),patch.object(r.time,'sleep',clock.sleep),patch.object(r.subprocess,'check_output',return_value=''),patch.object(r.subprocess,'Popen',Process),patch.object(r.os,'killpg',signal),contextlib.redirect_stdout(io.StringIO()):code=r.run(out,m)
   return code,json.loads((out/'result.json').read_text()),launched,signals
 def test_success_private_settings_cleanup(self):
  code,result,launched,_=self.execute('pass');self.assertEqual(code,0);self.assertTrue(result['passed']);self.assertEqual(len(launched),2)
 def test_full_budget_before_app_and_helper(self):
  for mode,count in [('preflight-slow',0),('validation-failure',0),('app-spawn-slow',1)]:
   with self.subTest(mode=mode):
    code,result,launched,_=self.execute(mode);self.assertEqual(code,1);self.assertFalse(result['passed']);self.assertEqual(len(launched),count)
 def test_missing_cleanup_held_restoration_state_preferences_fail_closed(self):
  for mode in ['held','unrestored','missing-cleanup','missing-foreground','semantic-failure','preferences-changed','preferences-error']:
   with self.subTest(mode=mode):
    code,result,_,_=self.execute(mode);self.assertEqual(code,1);self.assertFalse(result['passed'])
 def test_failed_helper_spawn_stops_only_owned_app(self):
  code,result,_,signals=self.execute('driver-spawn-failure');self.assertEqual(code,1);self.assertEqual(signals[0][0],999970);self.assertTrue(result['processExited'])
 def test_app_hang_bounded(self):
  code,result,_,signals=self.execute('timeout');self.assertEqual(code,1);self.assertTrue(signals);self.assertLessEqual(result['elapsedSeconds'],75)
 def test_saved_checkpoint_waits_for_real_callback_and_is_bounded(self):
  code,result,_,_=self.execute('checkpoint-pending');self.assertEqual(code,0);self.assertTrue(result['checkpoints']['ready']['passed']);self.assertIn('ready',result['checkpointWaits'])
  code,result,_,signals=self.execute('checkpoint-timeout');self.assertEqual(code,1);self.assertIn('did not settle',result['error']);self.assertLess(result['elapsedSeconds'],2);self.assertTrue(signals)
 def test_serialized_manifest_and_xml_checkpoints(self):
  with tempfile.TemporaryDirectory(prefix='cross-library-serialize-',dir=ROOT/'target') as tmp:
   out=Path(tmp);source=ROOT/'target/library-drag-fixture/profile';r.shutil.copytree(source,out/'profile')
   app=out/'Fixture.app/Contents';(app/'MacOS').mkdir(parents=True);(app/'MacOS/Vixen').write_text('nonexecutable synthetic fixture')
   info=app/'Info.plist';info.write_bytes(r.plistlib.dumps({'CFBundleIdentifier':'com.vixenlights.verification.synthetic'}))
   m=dict(source=str(source),sourceHashes=r.tree(source),preparedProfileHashes=r.tree(out/'profile'),initialLibrary=r.library_state(out/'profile'),frozenHashes={str(info):r.sha(info)},bundleIdentifier='com.vixenlights.verification.synthetic',app=str(app/'MacOS/Vixen'),controllers=0,inputDeadlineSeconds=40,allInDeadlineSeconds=75)
   r.save(out/'prepared.json',m);loaded=json.loads((out/'prepared.json').read_text());r.validate(out,loaded)
   ready=r.accept_checkpoint('ready',r.library_state(out/'profile'),loaded,{})
   self.assertTrue(ready['passed'])
   bad=copy.deepcopy(loaded);bad['initialLibrary']['entries'][0]['value'][1].append(['synthetic','changed'])
   with self.assertRaises(AssertionError):r.validate(out,bad)
 def test_xml_checkpoint_acceptance_uses_actual_retained_prior_native_copy(self):
  initial=r.library_state(ROOT/'target/library-drag-fixture/profile');copied=r.library_state(ROOT/'target/library-panel-to-toolbar-integrated-release/profile')
  copied['entries'][-1]['name']='Cross Window Curve';m={'initialLibrary':initial}
  ready=r.accept_checkpoint('ready',initial,m,{})
  value=r.accept_checkpoint('copied',copied,m,{'ready':ready});self.assertTrue(value['passed'])
  self.assertTrue(r.accept_checkpoint('rejected',copied,m,{'copied':value})['passed'])
  for mode in ['reorder','value','link','unrelated','rejected-byte-mutation']:
   changed=copy.deepcopy(copied)
   if mode=='reorder':changed['entries'][0],changed['entries'][1]=changed['entries'][1],changed['entries'][0]
   elif mode=='value':changed['entries'][-1]['points'][0][1]=11
   elif mode=='link':changed['entries'][-1]['link']='Curve A'
   elif mode=='unrelated':changed['otherXML']=['changed']
   else:changed['moduleStoreSha256']='changed'
   with self.subTest(mode=mode),self.assertRaises(AssertionError):r.accept_checkpoint('rejected',changed,m,{'copied':value})
if __name__=='__main__':unittest.main(verbosity=2)
