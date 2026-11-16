#!/usr/bin/env python3
"""Fault checks with fake native processes; never launches an app or posts input."""
import contextlib,io,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_layer_drag_regression as r
ROOT=Path(__file__).resolve().parents[1]
FG=dict(passed=True,cleanupCompleted=True,noInputHeld=True,restoredPrevious=True)
class Clock:
 def __init__(self):self.now=0.
 def monotonic(self):return self.now
 def sleep(self,n):self.now+=n
class Tests(unittest.TestCase):
 def execute(self,mode):
  with tempfile.TemporaryDirectory(prefix='layer-control-',dir=ROOT/'target') as tmp:
   out=Path(tmp);clock=Clock();children=[];launched=[];signals=[]
   m=dict(app='/private/app',helper='/private/driver',appSha256='pin',helperSha256='pin',source='/private/source',sourceHashes={},frozenHashes={},scope='fake',privateApp={'sourceExecutableSha256':'candidate'})
   class Process:
    def __init__(self,cmd,**kwargs):
     self.pid=999990+len(children);self.returncode=None;self.done=False;self.cmd=cmd;launched.append(cmd);children.append(self)
     if len(children)==1:
      self.done=mode!='timeout' and mode!='driver-spawn-failure';self.returncode=0 if self.done else None
      assert kwargs['env']['VIXEN_SETTINGS_DIR']==str(out/'ui')
      assert kwargs['env']['VIXEN_MUTE']=='1'
      if mode=='app-spawn-slow':clock.now+=20
     else:
      if mode=='driver-spawn-failure':raise OSError('spawn failed')
      self.done=True;self.returncode=0
      fg=dict(FG)
      if mode=='held':fg['noInputHeld']=False
      if mode=='unrestored':fg['restoredPrevious']=False
      if mode=='missing-cleanup':fg.pop('cleanupCompleted')
      if mode!='missing-foreground':(out/'foreground.json').write_text(json.dumps(fg))
    def poll(self):return self.returncode if self.done else None
    def wait(self,timeout):self.done=True;self.returncode=0;return 0
   def validation(o,m):
    if mode=='preflight-slow':clock.now=9
    if mode=='validation-failure':raise AssertionError('input mismatch')
   def verify(*args):
    if mode=='semantic-failure':raise AssertionError('incorrect saved state')
   def signal(pid,sig):
    signals.append((pid,sig))
    for child in children:
     if child.pid==pid:child.done=True;child.returncode=-15
   calls=0
   def prefs():
    nonlocal calls
    calls+=1
    if mode=='preferences-read-error' and calls>1:raise OSError('missing preferences')
    return {'path':'changed' if mode=='preferences-changed' and calls>1 else 'before'}
   with patch.object(r,'validate',validation),patch.object(r,'verify',verify),patch.object(r,'preferences',prefs),patch.object(r,'digest',lambda p:'pin'),patch.object(r,'hashes',lambda p:{}),patch.object(r.time,'monotonic',clock.monotonic),patch.object(r.time,'sleep',clock.sleep),patch.object(r.subprocess,'check_output',return_value=''),patch.object(r.subprocess,'Popen',Process),patch.object(r.os,'killpg',signal),contextlib.redirect_stdout(io.StringIO()):
    code=r.run(out,m)
   return code,json.loads((out/'result.json').read_text()),launched,signals
 def test_success_requires_cleanup_and_private_settings(self):
  code,result,launched,_=self.execute('pass');self.assertEqual(code,0);self.assertTrue(result['passed']);self.assertEqual(len(launched),2)
 def test_prelaunch_delay_or_validation_failure_never_launches(self):
  for mode in ['preflight-slow','validation-failure']:
   code,result,launched,_=self.execute(mode);self.assertEqual(code,1);self.assertFalse(result['passed']);self.assertEqual(launched,[])
 def test_missing_held_restore_preferences_and_semantic_fail_closed(self):
  for mode in ['held','unrestored','missing-cleanup','missing-foreground','semantic-failure','preferences-changed','preferences-read-error']:
   with self.subTest(mode=mode):
    code,result,_,_=self.execute(mode);self.assertEqual(code,1);self.assertFalse(result['passed'])
 def test_delay_after_app_launch_refuses_driver(self):
  code,result,launched,_=self.execute('app-spawn-slow');self.assertEqual(code,1);self.assertEqual(len(launched),1);self.assertIn('no longer fit',result['error'])
 def test_driver_spawn_failure_stops_owned_app(self):
  code,result,_,signals=self.execute('driver-spawn-failure');self.assertEqual(code,1);self.assertTrue(signals);self.assertFalse(result['passed'])
 def test_timeout_stops_owned_app_and_retains_failure(self):
  code,result,_,signals=self.execute('timeout');self.assertEqual(code,1);self.assertTrue(signals);self.assertIn('watchdog',result['error']);self.assertFalse(result['passed']);self.assertLessEqual(result['elapsedSeconds'],110)
if __name__=='__main__':unittest.main(verbosity=2)
