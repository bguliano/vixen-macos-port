#!/usr/bin/env python3
"""No apps or input: exercise batch decisions with native boundaries replaced."""
import contextlib,copy,importlib.util,io,json,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('batch',ROOT/'scripts/run_library_system_foreground_batch.py')
batch=importlib.util.module_from_spec(spec);spec.loader.exec_module(batch)
IDENTITY={'pid':123,'bundleIdentifier':'private.previous','executable':'/previous'}
FAMILY={'family':True}
GOOD={'passed':True,'ownerPreferencesUnchanged':True,'sourceUnchanged':True,'appUnchanged':True,'driverUnchanged':True}
FG={'passed':True,'targetExited':True,'restoredPrevious':True,'heldButtonAfterCleanup':False,'heldKeyAfterCleanup':False,'pointerInputPosted':False,'keyboardInputPosted':False}
SYS={'passed':True,'ownerPreferencesUnchanged':True,'processExited':True,'cleanupCompleted':True,'noInputHeld':True,'restoredPrevious':True,'inputsUnchanged':True,'appUnchanged':True,'driverUnchanged':True}
class GateTests(unittest.TestCase):
 def test_passed_family(self):self.assertTrue(batch.continuation_evidence(FAMILY,GOOD,FG)['safeToContinue'])
 def test_calibration_failure_without_input(self):
  data=dict(GOOD,passed=False);fg=dict(FG,passed=False,failurePhase='calibration_preinput',failureKind='calibration_rejected')
  self.assertTrue(batch.continuation_evidence(FAMILY,data,fg)['safeToContinue'])
 def test_family_cleanup_or_evidence_missing(self):
  for key in FG:
   if key not in ['targetExited','restoredPrevious','heldButtonAfterCleanup','heldKeyAfterCleanup']:continue
   for value in [None,not FG[key]]:
    with self.subTest(key=key,value=value):
     fg=dict(FG);fg[key]=value;self.assertFalse(batch.continuation_evidence(FAMILY,GOOD,fg)['safeToContinue'])
  for key in ['ownerPreferencesUnchanged','sourceUnchanged','appUnchanged']:
   with self.subTest(key=key):
    d=dict(GOOD);d.pop(key);self.assertFalse(batch.continuation_evidence(FAMILY,d,FG)['safeToContinue'])
 def test_failure_cannot_follow_input_or_unknown_phase(self):
  for override in [{'pointerInputPosted':True},{'keyboardInputPosted':True},{'pointerInputPosted':None},{'failurePhase':'drag'},{'failurePhase':'ownership'},{'failureKind':'timeout'},{'failureKind':None}]:
   with self.subTest(override=override):
    fg=dict(FG,passed=False,failurePhase='calibration_preinput',failureKind='calibration_rejected');fg.update(override)
    self.assertFalse(batch.continuation_evidence(FAMILY,dict(GOOD,passed=False),fg)['safeToContinue'])
 def test_system_diagnostic(self):
  d=dict(SYS,passed=False,failurePhase='rgb_control_readiness',failureKind='diagnostic',inputOwnershipMaintained=True)
  self.assertTrue(batch.continuation_evidence({'family':False},d)['safeToContinue'])
  for key in ['processExited','cleanupCompleted','noInputHeld','restoredPrevious','inputsUnchanged','appUnchanged','driverUnchanged','ownerPreferencesUnchanged','inputOwnershipMaintained']:
   with self.subTest(key=key):
    changed=dict(d);changed.pop(key);self.assertFalse(batch.continuation_evidence({'family':False},changed)['safeToContinue'])
  for key,value in [('failureKind','timeout'),('failureKind','ownership'),('failurePhase','font')]:
   with self.subTest(key=key,value=value):
    changed=dict(d);changed[key]=value;self.assertFalse(batch.continuation_evidence({'family':False},changed)['safeToContinue'])
 def test_system_pass(self):self.assertTrue(batch.continuation_evidence({'family':False},SYS)['safeToContinue'])
 def test_extra_suite_has_no_diagnostic_exception(self):
  d=dict(SYS,passed=False,failurePhase='rgb_control_readiness',failureKind='diagnostic',inputOwnershipMaintained=True)
  self.assertFalse(batch.continuation_evidence({'family':False,'diagnosticContinuationAllowed':False},d)['safeToContinue'])
class QueueTests(unittest.TestCase):
 def run_queue(self,mode):
  with tempfile.TemporaryDirectory(prefix='batch-control-test-',dir=ROOT/'target') as raw:
   out=Path(raw);stages=[];launched=[];waits=[];desktop=[];signals=[]
   watched=out/'pin';watched.write_text('before')
   for name in ['color','gradient','system-panels']:
    folder=out/name;folder.mkdir();stages.append({'name':name,'family':name!='system-panels','out':str(folder),'timeoutSeconds':75 if name!='system-panels' else 60,'ownedExecutables':['/private/'+name],'command':[name]})
   m={'absoluteDeadlineSeconds':90 if mode=='budget' else 240,'requestedIdleSeconds':240,'hashes':{str(watched):batch.digest(watched)},'stages':stages,'desktopDriver':'/private/driver'}
   (out/'prepared.json').write_text(json.dumps(m))
   class Process:
    def __init__(self,cmd,**kwargs):
     self.pid=999999;self.name=cmd[0];self.done=False;self.returncode=None;launched.append(self.name)
     if mode=='pin-change':watched.write_text('changed')
     if mode!='missing':
      data=dict(SYS if self.name=='system-panels' else GOOD)
      fg=dict(FG)
      if self.name=='color' and mode in ['diagnostic','unsafe']:
       data['passed']=False;fg.update(passed=False,failurePhase='calibration_preinput',failureKind='calibration_rejected')
       if mode=='unsafe':fg['heldButtonAfterCleanup']=True
      (out/self.name/'result.json').write_text(json.dumps(data));(out/self.name/'foreground.json').write_text(json.dumps(fg))
    def wait(self,timeout):
     waits.append((self.name,timeout))
     if mode=='timeout' and len(waits)==1:raise subprocess.TimeoutExpired(self.name,timeout)
     self.done=True;self.returncode=1 if self.name=='color' and mode in ['diagnostic','unsafe'] else 0
     return self.returncode
    def poll(self):return self.returncode if self.done else None
   def snapshot(_m,_o,label,_deadline):
    desktop.append(label)
    return dict(IDENTITY,pid=456) if mode=='focus-change' and label=='color-after' else IDENTITY
   def check_process(stage,deadline):
    if mode=='live-process' and launched:raise AssertionError('owned process remains')
    return {'matchingProcesses':[]}
   error=None
   with patch.object(sys,'argv',['batch','--out',str(out),'--run-prepared']),patch.object(batch.subprocess,'Popen',Process),patch.object(batch,'desktop_snapshot',snapshot),patch.object(batch,'assert_no_owned_processes',check_process),patch.object(batch,'cleanup_family',lambda _:[]),patch.object(batch.os,'killpg',lambda *a:signals.append(a)),contextlib.redirect_stdout(io.StringIO()):
    try:batch.main()
    except (AssertionError,SystemExit,subprocess.TimeoutExpired) as e:error=type(e).__name__
   report=json.loads((out/'result.json').read_text())
   return launched,error,report,signals
 def test_all_pass(self):
  launched,error,r,_=self.run_queue('pass');self.assertEqual(len(launched),3);self.assertIsNone(error);self.assertTrue(r['passed'])
 def test_diagnostic_continues_but_never_passes(self):
  launched,error,r,_=self.run_queue('diagnostic');self.assertEqual(len(launched),3);self.assertEqual(error,'SystemExit');self.assertFalse(r['passed']);self.assertEqual(r['verificationFailures'],['color']);self.assertTrue(r['allStagesAttempted'])
 def test_unsafe_or_incomplete_never_continues(self):
  for mode in ['unsafe','missing','focus-change','pin-change','live-process']:
   with self.subTest(mode=mode):
    launched,error,r,_=self.run_queue(mode);self.assertEqual(launched,['color']);self.assertIsNotNone(error);self.assertFalse(r['passed']);self.assertNotIn('allStagesAttempted',r)
 def test_timeout_aborts_after_cleanup(self):
  launched,error,r,signals=self.run_queue('timeout');self.assertEqual(launched,['color']);self.assertEqual(error,'TimeoutExpired');self.assertFalse(r['passed']);self.assertEqual(len(signals),1)
 def test_budget_refuses_launch(self):
  launched,error,r,_=self.run_queue('budget');self.assertEqual(launched,[]);self.assertIsNotNone(error);self.assertFalse(r['passed'])
if __name__=='__main__':unittest.main(verbosity=2)
