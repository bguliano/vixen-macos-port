#!/usr/bin/env python3
import copy,json,tempfile,time,unittest,zipfile
from pathlib import Path
from unittest.mock import Mock,patch,call
import test_prop_cross_window as r
class Controls(unittest.TestCase):
 def fixture(self,out):
  (out/'saved').mkdir();original=r.canonical(r.package(r.FIXTURE))
  for name in ['source','destination']:
   (out/(name+'.prp')).write_bytes(r.FIXTURE.read_bytes());(out/'saved'/(name+'.prp')).write_bytes(r.FIXTURE.read_bytes())
  return dict(original=original)
 def replace(self,p,change):
  with zipfile.ZipFile(p) as z:items={n:z.read(n) for n in z.namelist()}
  change(items)
  with zipfile.ZipFile(p,'w') as z:
   for n,b in items.items():z.writestr(n,b)
 def test_equal_ids_and_exact_preserved_packages(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);m=self.fixture(o);self.assertEqual(set(r.verify_packages(o,m)),{'source','destination'})
 def test_reorder_uuid_background_and_extra_file_rejected(self):
  def jsonchange(items,fn):
   m=json.loads(items['prop.json']);fn(m);items['prop.json']=json.dumps(m).encode()
  mutations=[lambda d:jsonchange(d,lambda m:m['elements'][0]['childIds'].reverse()),lambda d:jsonchange(d,lambda m:m['elements'][1].update(id='00000000-0000-0000-0000-000000000000')),lambda d:d.update({'background.jpg':b'changed'}),lambda d:d.update({'unexpected':b'x'})]
  for change in mutations:
   with tempfile.TemporaryDirectory() as d:
    o=Path(d);m=self.fixture(o);self.replace(o/'saved/destination.prp',change)
    with self.assertRaises(AssertionError):r.verify_packages(o,m)
 def test_modified_date_only_normalization(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);m=self.fixture(o)
   def change(items):
    v=json.loads(items['prop.json']);v['prop']['modifiedDate']='2026-11-12T20:00:00';items['prop.json']=json.dumps(v).encode()
   self.replace(o/'saved/source.prp',change);self.assertEqual(set(r.verify_packages(o,m)),{'source','destination'})
 def test_deadline_and_group_cleanup(self):
  with self.assertRaises(AssertionError):r.remaining(time.monotonic()+2,41)
  child=Mock(pid=13579);child.poll.return_value=0
  with patch.object(r.os,'killpg') as kill:r.terminate(child,time.monotonic()+2)
  self.assertEqual(kill.call_args_list,[call(13579,r.signal.SIGTERM),call(13579,r.signal.SIGKILL)])
 def test_partial_report_does_not_skip_terminal_evidence(self):
  with tempfile.TemporaryDirectory() as d:
   o=Path(d);(o/'foreground.json').write_text('{')
   with patch.object(r,'validate',side_effect=AssertionError('fixture preflight failure')),patch.object(r,'owner_states',return_value={}),patch.object(r,'hashes',return_value={}):self.assertEqual(r.run(o,dict(scope='offline control',pins={},sourceHashes={})),1)
   m=json.loads((o/'result.json').read_text());self.assertFalse(m['passed']);self.assertTrue(m['appExited'] and m['helperExited']);self.assertIn('ownerPreferencesUnchanged',m);self.assertTrue(m['cleanupErrors'])
if __name__=='__main__':
 result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));r.save(r.ROOT/'target/prop-cross-window-helper-02/consumer-controls.json',dict(passed=result.wasSuccessful(),methods=result.testsRun,nativeOperations=0));raise SystemExit(not result.wasSuccessful())
