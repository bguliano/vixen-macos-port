#!/usr/bin/env python3
"""Exercise queue timing and cleanup using fake processes only; no native launch/input."""
import contextlib,importlib.util,io,json,signal,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('native_queue',ROOT/'scripts/run_native_foreground_queue.py')
q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)
IDENTITY={'pid':123,'bundleIdentifier':'private.previous','executable':'/previous'}
GOOD={'passed':True,'ownerPreferencesUnchanged':True,'processExited':True,'cleanupCompleted':True,'noInputHeld':True,'restoredPrevious':True,'inputsUnchanged':True,'appUnchanged':True,'driverUnchanged':True}
class Clock:
 def __init__(self):self.now=0.
 def monotonic(self):return self.now
 def sleep(self,n):self.now+=n
class QueueTests(unittest.TestCase):
 def run_queue(self,mode):
  with tempfile.TemporaryDirectory(prefix='native-queue-test-',dir=ROOT/'target') as raw:
   out=Path(raw);clock=Clock();launched=[];waits=[];signals=[];cleanups=[];stages=[]
   pin=out/'pin';pin.write_text('before')
   for name,timeout in [('inline',75),('mapper',110)]:
    folder=out/name;folder.mkdir();stages.append({'name':name,'family':False,'diagnosticContinuationAllowed':False,'out':str(folder),'timeoutSeconds':timeout,'ownedExecutables':['/private/'+name], 'command':[name]})
   m={'absoluteDeadlineSeconds':240,'requestedIdleSeconds':240,'hashes':{str(pin):q.digest(pin)},'stages':stages,'desktopDriver':'/private/desktop'}
   (out/'prepared.json').write_text(json.dumps(m))
   class Process:
    def __init__(self,cmd,**kwargs):
     self.pid=999998;self.name=cmd[0];self.done=False;self.returncode=None;launched.append(self.name)
     if mode=='pin-change':pin.write_text('changed')
     if mode=='slow-popen':clock.now+=150
     if mode!='missing':
      d=dict(GOOD)
      if self.name=='inline' and mode=='failed':d['passed']=False
      if mode=='held':d['noInputHeld']=False
      (out/self.name/'result.json').write_text(json.dumps(d))
    def wait(self,timeout):
     waits.append(timeout)
     if mode in ['timeout','parent-exit-race','parent-kill-error'] and len(waits)==1:raise subprocess.TimeoutExpired(self.name,timeout)
     self.done=True;self.returncode=1 if self.name=='inline' and mode=='failed' else 0;return self.returncode
    def poll(self):return self.returncode if self.done else None
   def desktop(m,o,label,deadline):
    if mode=='preflight-cost' and label=='inline-before':clock.now=150
    return dict(IDENTITY,pid=456) if mode=='focus-change' and label=='inline-after' else IDENTITY
   def inventory(stage,deadline):
    if mode=='live-process' and launched:raise AssertionError('owned process remains')
    return {'matchingProcesses':[]}
   def signal_parent(*args):
    signals.append(args)
    if mode=='parent-exit-race':raise ProcessLookupError('already exited')
    if mode=='parent-kill-error':raise PermissionError('synthetic kill failure')
   error=None
   with patch.object(sys,'argv',['queue','--out',str(out),'--run-prepared']),patch.object(q.time,'monotonic',clock.monotonic),patch.object(q.time,'sleep',clock.sleep),patch.object(q.subprocess,'Popen',Process),patch.object(q,'desktop_snapshot',desktop),patch.object(q,'assert_no_owned_processes',inventory),patch.object(q,'cleanup_owned',lambda s,d:cleanups.append(s['name']) or []),patch.object(q.os,'killpg',signal_parent),contextlib.redirect_stdout(io.StringIO()):
    try:q.main()
    except (AssertionError,subprocess.TimeoutExpired) as e:error=type(e).__name__
   report=json.loads((out/'result.json').read_text())
   return launched,error,report,waits,signals,cleanups
 def test_all_success(self):
  launched,error,r,*_=self.run_queue('pass');self.assertEqual(launched,['inline','mapper']);self.assertIsNone(error);self.assertTrue(r['passed'])
 def test_prelaunch_checks_consuming_budget_prevent_launch(self):
  launched,error,r,*_=self.run_queue('preflight-cost');self.assertEqual(launched,[]);self.assertEqual(error,'AssertionError');self.assertIn('Prelaunch checks consumed',r['error']);self.assertFalse(r['passed'])
 def test_slow_spawn_does_not_renew_remaining_budget(self):
  launched,error,r,waits,*_=self.run_queue('slow-popen');self.assertEqual(launched,['inline']);self.assertEqual(waits,[70]);self.assertFalse(r['passed']);self.assertEqual(error,'AssertionError');self.assertEqual(r['elapsedSeconds'],150)
 def test_failed_held_missing_focus_or_integrity_never_advances(self):
  for mode in ['failed','held','missing','focus-change','pin-change','live-process']:
   with self.subTest(mode=mode):
    launched,error,r,_,_,cleanup=self.run_queue(mode);self.assertEqual(launched,['inline']);self.assertIsNotNone(error);self.assertFalse(r['passed']);self.assertEqual(cleanup,['inline'])
 def test_timeout_kills_parent_and_invokes_native_cleanup(self):
  launched,error,r,_,signals,cleanup=self.run_queue('timeout');self.assertEqual(launched,['inline']);self.assertEqual(error,'TimeoutExpired');self.assertEqual(signals,[(999998,signal.SIGTERM)]);self.assertEqual(cleanup,['inline']);self.assertFalse(r['passed'])
 def test_parent_cleanup_failure_cannot_skip_owned_cleanup(self):
  for mode in ['parent-exit-race','parent-kill-error']:
   with self.subTest(mode=mode):
    launched,error,r,_,signals,cleanup=self.run_queue(mode);self.assertEqual(launched,['inline']);self.assertEqual(error,'TimeoutExpired');self.assertEqual(cleanup,['inline']);self.assertFalse(r['passed'])
    self.assertEqual('runnerCleanupError' in r['stages'][0],mode=='parent-kill-error')
class CleanupTests(unittest.TestCase):
 def exercise(self,mode):
  clock=Clock();stage={'ownedExecutables':['/private/app','/private/helper']};processes={201:'/private/app',202:'/private/helper',303:'/unrelated/app'};signals=[];calls=0
  def ps(*args,**kwargs):
   nonlocal calls
   calls+=1
   if mode=='reused' and calls==2:processes[201]='/unrelated/reused'
   return '\n'.join(f'{p} {exe}' for p,exe in processes.items())
  def send(pid,sig):
   signals.append((pid,sig))
   if pid==202 or sig==signal.SIGKILL:processes.pop(pid,None)
  with patch.object(q.time,'monotonic',clock.monotonic),patch.object(q.time,'sleep',clock.sleep),patch.object(q.subprocess,'check_output',ps),patch.object(q.os,'getpgid',lambda p:p),patch.object(q.os,'killpg',send),patch.object(q.os,'kill',send):
   result=q.cleanup_owned(stage,20)
  return result,signals,processes,clock.now
 def test_separate_owned_groups_terminate_then_kill(self):
  result,signals,remaining,elapsed=self.exercise('normal');self.assertIn((201,signal.SIGTERM),signals);self.assertIn((201,signal.SIGKILL),signals);self.assertIn((202,signal.SIGTERM),signals);self.assertEqual(remaining,{303:'/unrelated/app'});self.assertLess(elapsed,20)
 def test_changed_identity_is_never_signaled(self):
  result,signals,remaining,elapsed=self.exercise('reused');self.assertFalse(any(pid==201 for pid,_ in signals));self.assertEqual(remaining[201],'/unrelated/reused');self.assertEqual(remaining[303],'/unrelated/app')
if __name__=='__main__':unittest.main(verbosity=2)
