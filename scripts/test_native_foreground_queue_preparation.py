#!/usr/bin/env python3
"""Exercise queue preparation without executing any prepared command."""
import importlib.util,json,sys,tempfile,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('prepare_queue',ROOT/'scripts/prepare_native_foreground_queue.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class PreparationTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory(prefix='queue-prepare-test-',dir=ROOT/'target');self.root=Path(self.temp.name);self.plans=[]
  self.runner=ROOT/'scripts/test_foreground_batch_control.py'
  for name,timeout in [('inline',75),('mapper',110)]:
   out=self.root/name;out.mkdir();exe=out/'fixture';driver=out/'driver';frozen=out/'runner-source.py'
   exe.write_bytes(b'not executable fixture');driver.write_bytes(b'not executable helper');frozen.write_bytes(self.runner.read_bytes())
   m={'stageName':name,'timeoutSeconds':timeout,'executable':str(exe),'driver':str(driver),'ownedExecutables':[str(exe),str(driver)],'executableSha256':module.digest(exe),'driverSha256':module.digest(driver),'command':[sys.executable,'-B',str(self.runner),'--out',str(out),'--run-prepared'],'pinnedInputHashes':{str(frozen):module.digest(frozen)},'originalInputHashes':{str(self.runner):module.digest(self.runner)}}
   p=out/'prepared.json';p.write_text(json.dumps(m));self.plans.append(p)
  self.desktop=self.plans[0]
 def tearDown(self):self.temp.cleanup()
 def change(self,index,mutate):
  p=self.plans[index];m=json.loads(p.read_text());mutate(m);p.write_text(json.dumps(m))
 def run_prepare(self,seconds=240):return module.prepare(self.root/'queue',self.plans,self.desktop,seconds)
 def test_valid_two_suite_queue_has_all_required_pins(self):
  m=self.run_prepare();self.assertEqual(m['stageWatchdogSumSeconds'],185);self.assertEqual(m['absoluteDeadlineSeconds'],240);self.assertFalse(m['launchPerformed']);self.assertIn(str(self.runner),m['hashes']);self.assertTrue(all(not x['family'] and not x['diagnosticContinuationAllowed'] for x in m['stages']));self.assertFalse(any((p.parent/'result.json').exists() for p in self.plans))
 def test_refuses_oversubscribed_budget(self):
  with self.assertRaises(AssertionError):self.run_prepare(205)
  self.assertFalse((self.root/'queue').exists())
 def test_refuses_already_attempted_suite(self):
  (self.plans[0].parent/'result.json').write_text('{}')
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_changed_frozen_runner(self):
  (self.plans[1].parent/'runner-source.py').write_text('changed')
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_wrong_current_runner_pin(self):
  self.change(0,lambda m:m['originalInputHashes'].update({str(self.runner):'0'*64}))
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_changed_executable(self):
  (self.plans[0].parent/'fixture').write_text('changed')
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_duplicate_stage_names(self):
  self.change(1,lambda m:m.update(stageName='inline'))
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_mismatched_command_destination(self):
  def wrong(m):m['command'][4]=str(self.root)
  self.change(1,wrong)
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_shell_command(self):
  self.change(0,lambda m:m.update(command=['/bin/sh','-c','echo test']))
  with self.assertRaises(AssertionError):self.run_prepare()
 def test_refuses_replacing_existing_queue(self):
  self.run_prepare()
  with self.assertRaises(AssertionError):self.run_prepare()
if __name__=='__main__':unittest.main(verbosity=2)
